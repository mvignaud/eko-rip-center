//! Rendus en parallèle (roadmap 9.11) : jusqu'à N rendus à la fois (`EKO_RIP_RENDUS`).
//!
//! Les travaux attendent leur tour dans l'ordre d'arrivée, un import manuel (écran, planche,
//! nouveau rendu) passant devant les arrivées automatiques (dossiers surveillés, ERP).
//! Les arrivées automatiques sortent vers la file des machines dans leur ordre d'arrivée,
//! même quand un petit fichier finit son rendu avant un gros arrivé plus tôt : le travail fini
//! attend que ceux d'avant soient sortis (ou en échec). Un import manuel sort dès qu'il est
//! prêt.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Notify, Semaphore, SemaphorePermit};

/// Au plus 16 rendus simultanés : au-delà, la mémoire manque bien avant les cœurs.
pub const MAX: usize = 16;

/// Nombre de rendus simultanés par défaut : un par cœur, 4 au plus.
pub fn par_defaut() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, 4)
}

pub struct Rendus<T> {
    places: Semaphore,
    simultanes: usize,
    /// Travaux en attente d'une place : (automatique, n° d'arrivée), le plus petit passe.
    attente: Mutex<BTreeSet<(bool, u64)>>,
    reveil: Notify,
    arrivees: AtomicU64,
    /// Rang de sortie des arrivées automatiques, donné à l'inscription.
    rangs: AtomicU64,
    sorties: Mutex<Sorties<T>>,
}

struct Sorties<T> {
    prochain: u64,
    /// Rendus finis qui attendent ceux d'avant ; `None` : rien à sortir (échec, abandon).
    prets: BTreeMap<u64, Option<T>>,
}

/// Inscription d'un travail, faite à son arrivée (avant toute attente) pour garder l'ordre.
pub struct Place {
    cle: (bool, u64),
    rang: Option<u64>,
}

/// Place de rendu tenue ; la rendre par [`Jeton::terminer`].
pub struct Jeton<'a, T> {
    rendus: &'a Rendus<T>,
    permis: Option<SemaphorePermit<'a>>,
    rang: Option<u64>,
}

impl<T> Rendus<T> {
    pub fn new(simultanes: usize) -> Self {
        let simultanes = simultanes.clamp(1, MAX);
        Rendus {
            places: Semaphore::new(simultanes),
            simultanes,
            attente: Default::default(),
            reveil: Notify::new(),
            arrivees: AtomicU64::new(0),
            rangs: AtomicU64::new(0),
            sorties: Mutex::new(Sorties { prochain: 0, prets: BTreeMap::new() }),
        }
    }

    pub fn simultanes(&self) -> usize {
        self.simultanes
    }

    /// (rendus en cours, travaux en attente d'une place).
    pub fn occupation(&self) -> (usize, usize) {
        (self.simultanes - self.places.available_permits(), self.attente.lock().unwrap().len())
    }

    pub fn inscrire(&self, automatique: bool) -> Place {
        let cle = (automatique, self.arrivees.fetch_add(1, Ordering::SeqCst));
        self.attente.lock().unwrap().insert(cle);
        Place { cle, rang: automatique.then(|| self.rangs.fetch_add(1, Ordering::SeqCst)) }
    }

    /// Attend que le travail soit le premier de l'attente et qu'une place se libère.
    pub async fn attendre(&self, place: Place) -> Jeton<'_, T> {
        // Tâche annulée en attente : elle quitte l'attente, et son rang ne bloque pas la sortie.
        struct Garde<'a, T>(&'a Rendus<T>, Option<Place>);
        impl<T> Drop for Garde<'_, T> {
            fn drop(&mut self) {
                if let Some(p) = self.1.take() {
                    self.0.attente.lock().unwrap().remove(&p.cle);
                    self.0.reveil.notify_waiters();
                    if let Some(r) = p.rang {
                        self.0.poser(r, None, |_| {});
                    }
                }
            }
        }
        let mut garde = Garde(self, Some(place));
        let cle = garde.1.as_ref().unwrap().cle;
        loop {
            let reveil = self.reveil.notified();
            tokio::pin!(reveil);
            reveil.as_mut().enable();
            {
                let mut attente = self.attente.lock().unwrap();
                if attente.first() == Some(&cle)
                    && let Ok(permis) = self.places.try_acquire()
                {
                    attente.remove(&cle);
                    drop(attente);
                    // Le suivant de l'attente peut prendre une autre place libre.
                    self.reveil.notify_waiters();
                    let rang = garde.1.take().unwrap().rang;
                    return Jeton { rendus: self, permis: Some(permis), rang };
                }
            }
            reveil.await;
        }
    }

    /// Un rendu fini qui attend encore son tour de sortie (arrivée automatique) répond à `f`.
    pub fn retenu(&self, f: impl Fn(&T) -> bool) -> bool {
        self.sorties.lock().unwrap().prets.values().flatten().any(f)
    }

    /// Range un rendu fini à son rang et sort, dans l'ordre, tout ce qui est prêt.
    fn poser(&self, rang: u64, sortie: Option<T>, mut sortir: impl FnMut(T)) {
        let mut s = self.sorties.lock().unwrap();
        s.prets.insert(rang, sortie);
        loop {
            let prochain = s.prochain;
            let Some(pret) = s.prets.remove(&prochain) else { break };
            s.prochain += 1;
            if let Some(t) = pret {
                sortir(t);
            }
        }
    }
}

impl<T> Jeton<'_, T> {
    /// Rend la place. `sortie` part par `sortir` à son tour (arrivée automatique) ou tout de
    /// suite (import manuel) ; `None` quand il n'y a rien à mettre en file.
    pub fn terminer(mut self, sortie: Option<T>, mut sortir: impl FnMut(T)) {
        self.liberer();
        match self.rang.take() {
            Some(r) => self.rendus.poser(r, sortie, sortir),
            None => sortie.into_iter().for_each(&mut sortir),
        }
    }

    fn liberer(&mut self) {
        if self.permis.take().is_some() {
            self.rendus.reveil.notify_waiters();
        }
    }
}

impl<T> Drop for Jeton<'_, T> {
    /// Rendu interrompu (panique) : sa place et son rang sont rendus, les suivants sortiront
    /// au prochain rendu fini.
    fn drop(&mut self) {
        self.liberer();
        if let Some(r) = self.rang.take() {
            let mut s = self.rendus.sorties.lock().unwrap();
            s.prets.insert(r, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    /// Un rendu fini qui attend un rendu plus ancien est « retenu » jusqu'à sa sortie.
    #[test]
    fn rendu_retenu_jusqu_a_sa_sortie() {
        let r: Rendus<u32> = Rendus::new(2);
        let mut sortis = vec![];
        r.poser(1, Some(7), |t| sortis.push(t));
        assert!(r.retenu(|t| *t == 7) && sortis.is_empty());
        r.poser(0, Some(3), |t| sortis.push(t));
        assert_eq!(sortis, [3, 7]);
        assert!(!r.retenu(|t| *t == 7));
    }

    /// 20 fichiers déposés, 4 rendus à la fois, durées dans le désordre : jamais plus de 4
    /// rendus, 4 atteints, et la sortie suit l'ordre de dépôt.
    #[tokio::test]
    async fn ordre_de_depot_garde() {
        let r: Arc<Rendus<usize>> = Arc::new(Rendus::new(4));
        let sortis = Arc::new(Mutex::new(vec![]));
        let (en_cours, pic) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)));
        let mut taches = vec![];
        for i in 0..20usize {
            let place = r.inscrire(true);
            let (r, sortis, en_cours, pic) = (r.clone(), sortis.clone(), en_cours.clone(), pic.clone());
            taches.push(tokio::spawn(async move {
                let j = r.attendre(place).await;
                let n = en_cours.fetch_add(1, Ordering::SeqCst) + 1;
                pic.fetch_max(n, Ordering::SeqCst);
                // Les derniers déposés finissent vite, les premiers lentement.
                tokio::time::sleep(Duration::from_millis(((20 - i) * 7 % 50) as u64 + 5)).await;
                en_cours.fetch_sub(1, Ordering::SeqCst);
                // Le 7 échoue : rien à sortir, la suite ne reste pas bloquée.
                j.terminer((i != 7).then_some(i), |x| sortis.lock().unwrap().push(x));
            }));
        }
        for t in taches {
            t.await.unwrap();
        }
        assert_eq!(pic.load(Ordering::SeqCst), 4);
        let attendu: Vec<usize> = (0..20).filter(|&i| i != 7).collect();
        assert_eq!(*sortis.lock().unwrap(), attendu);
        assert_eq!(r.occupation(), (0, 0));
    }

    /// Une place, tenue : 3 arrivées automatiques attendent, puis un import manuel arrive ;
    /// il passe devant elles et sort dès qu'il est rendu.
    #[tokio::test]
    async fn import_manuel_prioritaire() {
        let r: Arc<Rendus<&'static str>> = Arc::new(Rendus::new(1));
        let ordre = Arc::new(Mutex::new(vec![]));
        let tenue = r.attendre(r.inscrire(true)).await;
        let mut taches = vec![];
        for (nom, auto) in [("auto1", true), ("auto2", true), ("auto3", true), ("manuel", false)] {
            let place = r.inscrire(auto);
            let (r, ordre) = (r.clone(), ordre.clone());
            taches.push(tokio::spawn(async move {
                let j = r.attendre(place).await;
                ordre.lock().unwrap().push(nom);
                j.terminer(Some(nom), |_| {});
            }));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(r.occupation(), (1, 4));
        tenue.terminer(None, |_| {});
        for t in taches {
            t.await.unwrap();
        }
        assert_eq!(*ordre.lock().unwrap(), ["manuel", "auto1", "auto2", "auto3"]);
    }

    /// Un import manuel fini sort tout de suite, sans attendre une arrivée automatique plus
    /// ancienne encore en rendu ; une attente abandonnée ne bloque pas la sortie.
    #[tokio::test]
    async fn manuel_sort_sans_attendre() {
        let r: Rendus<&str> = Rendus::new(2);
        let mut sortis = vec![];
        let lent = r.attendre(r.inscrire(true)).await;
        let manuel = r.attendre(r.inscrire(false)).await;
        // Arrivée automatique annulée pendant son attente (aucune place libre).
        let abandon = r.inscrire(true);
        assert!(tokio::time::timeout(Duration::from_millis(20), r.attendre(abandon)).await.is_err());
        manuel.terminer(Some("manuel"), |x| sortis.push(x));
        assert_eq!(sortis, ["manuel"]);
        let suivant = r.inscrire(true);
        lent.terminer(Some("lent"), |x| sortis.push(x));
        r.attendre(suivant).await.terminer(Some("suivant"), |x| sortis.push(x));
        assert_eq!(sortis, ["manuel", "lent", "suivant"]);
    }
}
