//! File d'attente des sorties : une page d'un travail vers une machine. L'ordre de la liste
//! est l'ordre de passage ; chaque machine prend la première sortie en attente qui la vise.
//! Gardée dans `<données>/file.json`.

use crate::machines::Operation;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Etat {
    EnAttente,
    Suspendue,
    EnCours,
    Terminee,
    Echec,
    Annulee,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sortie {
    pub id: Uuid,
    /// Identifiant de la machine dans le catalogue (`epson`, `kona`…).
    pub machine: String,
    /// Absente des files d'avant le catalogue : déduite de la machine à la lecture.
    #[serde(default)]
    pub operation: Option<Operation>,
    pub travail: Uuid,
    /// Nom du travail, pour l'affichage.
    pub nom: String,
    /// Page du travail, numérotée à partir de 1.
    pub page: usize,
    pub etat: Etat,
    #[serde(default)]
    pub code_erreur: Option<String>,
    #[serde(default)]
    pub erreur: Option<String>,
    /// Secondes depuis l'époque Unix.
    pub cree_le: u64,
    #[serde(default)]
    pub debut: Option<u64>,
    #[serde(default)]
    pub fin: Option<u64>,
}

impl Sortie {
    pub fn nouvelle(machine: &str, operation: Operation, travail: Uuid, nom: &str, page: usize, le: u64) -> Self {
        Sortie { id: Uuid::new_v4(), machine: machine.into(), operation: Some(operation), travail, nom: nom.into(), page, etat: Etat::EnAttente, code_erreur: None, erreur: None, cree_le: le, debut: None, fin: None }
    }

    pub fn operation(&self) -> Operation {
        self.operation.unwrap_or(if self.machine == "kona" { Operation::Decoupe } else { Operation::Impression })
    }

    /// En attente, suspendue ou en cours : le travail ne peut pas être supprimé ni relancé.
    pub fn active(&self) -> bool {
        matches!(self.etat, Etat::EnAttente | Etat::Suspendue | Etat::EnCours)
    }
}

/// Action demandée depuis l'écran sur une sortie.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Suspendre,
    Reprendre,
    Annuler,
    /// Passe devant toutes les sorties en attente.
    EnTete,
    Monter,
    Descendre,
    /// Nouvelle sortie identique, sans refaire le rendu.
    Refaire,
}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub sorties: Vec<Sortie>,
}

impl File {
    pub fn lire(donnees: &Path) -> File {
        let mut f: File = std::fs::read(fichier(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default();
        // Une sortie coupée par un arrêt ne repart pas seule : une découpe à moitié faite ne
        // se reprend pas au milieu.
        for s in f.sorties.iter_mut().filter(|s| s.etat == Etat::EnCours) {
            s.etat = Etat::Echec;
            s.code_erreur = Some("interrompu".into());
        }
        f
    }

    pub fn ecrire(&self, donnees: &Path) -> std::io::Result<()> {
        let tmp = fichier(donnees).with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap())?;
        std::fs::rename(tmp, fichier(donnees))
    }

    /// Prochaine sortie à lancer sur `machine` (aucune si une y tourne déjà).
    pub fn prochaine(&self, machine: &str) -> Option<usize> {
        let mut pour = self.sorties.iter().enumerate().filter(|(_, s)| s.machine == machine);
        if pour.clone().any(|(_, s)| s.etat == Etat::EnCours) {
            return None;
        }
        pour.find(|(_, s)| s.etat == Etat::EnAttente).map(|(i, _)| i)
    }

    pub fn active_pour(&self, travail: Uuid) -> bool {
        self.sorties.iter().any(|s| s.travail == travail && s.active())
    }

    /// Applique l'action ; `Err` porte le code d'erreur traduit par l'écran.
    pub fn agir(&mut self, id: Uuid, action: Action, le: u64) -> Result<(), &'static str> {
        let i = self.sorties.iter().position(|s| s.id == id).ok_or("introuvable")?;
        let etat = self.sorties[i].etat;
        match action {
            Action::Suspendre if etat == Etat::EnAttente => self.sorties[i].etat = Etat::Suspendue,
            Action::Reprendre if etat == Etat::Suspendue => self.sorties[i].etat = Etat::EnAttente,
            Action::Annuler if matches!(etat, Etat::EnAttente | Etat::Suspendue) => self.sorties[i].etat = Etat::Annulee,
            Action::EnTete | Action::Monter | Action::Descendre if matches!(etat, Etat::EnAttente | Etat::Suspendue) => {
                // Places des sorties encore à passer sur la même machine : on échange avec la
                // voisine, les autres machines gardent leurs places.
                let m = self.sorties[i].machine.clone();
                let places: Vec<usize> = self.sorties.iter().enumerate().filter(|(_, x)| x.machine == m && matches!(x.etat, Etat::EnAttente | Etat::Suspendue)).map(|(j, _)| j).collect();
                let rang = places.iter().position(|&j| j == i).unwrap();
                match action {
                    Action::Monter if rang > 0 => self.sorties.swap(i, places[rang - 1]),
                    Action::Descendre if rang + 1 < places.len() => self.sorties.swap(i, places[rang + 1]),
                    Action::EnTete => (1..=rang).rev().for_each(|r| self.sorties.swap(places[r], places[r - 1])),
                    _ => {}
                }
            }
            Action::Refaire if !matches!(etat, Etat::EnCours) => {
                let s = &self.sorties[i];
                let copie = Sortie::nouvelle(&s.machine, s.operation(), s.travail, &s.nom, s.page, le);
                self.sorties.push(copie);
            }
            _ => return Err("action_impossible"),
        }
        Ok(())
    }

    /// Oublie l'historique d'un travail supprimé (ses sorties actives restent).
    pub fn oublier_travail(&mut self, travail: Uuid) {
        self.sorties.retain(|s| s.travail != travail || s.active());
    }

    /// Retire de la liste une sortie finie (terminée, en échec, annulée).
    pub fn retirer(&mut self, id: Uuid) -> Result<(), &'static str> {
        let i = self.sorties.iter().position(|s| s.id == id).ok_or("introuvable")?;
        if self.sorties[i].active() {
            return Err("action_impossible");
        }
        self.sorties.remove(i);
        Ok(())
    }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("file.json")
}

/// Dossier où une sortie sans pilote (plaques de l'Epson) est copiée.
pub fn dossier_sortie(donnees: &Path, s: &Sortie) -> PathBuf {
    donnees.join("sorties").join(&s.machine).join(format!("{}-p{}-{}", s.travail, s.page, s.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KONA: &str = "kona";
    const EPSON: &str = "epson";

    fn file(machines: &[&str]) -> File {
        File { sorties: machines.iter().enumerate().map(|(i, &m)| Sortie::nouvelle(m, if m == KONA { Operation::Decoupe } else { Operation::Impression }, Uuid::new_v4(), &format!("t{i}"), 1, 0)).collect() }
    }

    fn noms(f: &File) -> Vec<&str> {
        f.sorties.iter().map(|s| s.nom.as_str()).collect()
    }

    #[test]
    fn chaque_machine_prend_la_premiere_en_attente() {
        let mut f = file(&[KONA, EPSON, KONA]);
        assert_eq!((f.prochaine(KONA), f.prochaine(EPSON)), (Some(0), Some(1)));
        f.sorties[0].etat = Etat::EnCours;
        assert_eq!(f.prochaine(KONA), None, "une à la fois par machine");
        assert_eq!(f.prochaine(EPSON), Some(1), "l'autre machine n'attend pas");
        f.sorties[0].etat = Etat::Terminee;
        f.sorties[2].etat = Etat::Suspendue;
        assert_eq!(f.prochaine(KONA), None, "suspendue : on ne la lance pas");
    }

    #[test]
    fn deplacer_dans_la_file_de_sa_machine() {
        let mut f = file(&[KONA, EPSON, KONA, KONA]);
        let (t2, t3) = (f.sorties[2].id, f.sorties[3].id);
        f.agir(t3, Action::EnTete, 0).unwrap();
        assert_eq!(noms(&f), ["t3", "t1", "t0", "t2"], "l'Epson garde sa place");
        f.agir(t2, Action::Monter, 0).unwrap();
        assert_eq!(noms(&f), ["t3", "t1", "t2", "t0"]);
        f.agir(t2, Action::Descendre, 0).unwrap();
        f.agir(t2, Action::Descendre, 0).unwrap();
        assert_eq!(noms(&f), ["t3", "t1", "t0", "t2"], "descendre au-delà du dernier : il reste dernier");
        assert_eq!(f.prochaine(KONA), Some(0));
        assert_eq!(f.sorties[f.prochaine(KONA).unwrap()].nom, "t3");
    }

    #[test]
    fn etats_et_actions_permises() {
        let mut f = file(&[KONA]);
        let id = f.sorties[0].id;
        assert_eq!(f.agir(id, Action::Reprendre, 0), Err("action_impossible"));
        f.agir(id, Action::Suspendre, 0).unwrap();
        assert!(f.active_pour(f.sorties[0].travail));
        assert_eq!(f.retirer(id), Err("action_impossible"), "on ne retire pas une sortie à passer");
        f.agir(id, Action::Annuler, 0).unwrap();
        f.agir(id, Action::Refaire, 5).unwrap();
        assert_eq!(f.sorties.len(), 2);
        assert_eq!((f.sorties[1].etat, f.sorties[1].cree_le, f.sorties[1].travail), (Etat::EnAttente, 5, f.sorties[0].travail));
        f.retirer(id).unwrap();
        assert_eq!(f.sorties.len(), 1);
        assert_eq!(f.agir(Uuid::new_v4(), Action::Annuler, 0), Err("introuvable"));
    }

    #[test]
    fn file_d_avant_le_catalogue() {
        let ancienne = r#"{"id":"6f9619ff-8b86-d011-b42d-00c04fc964ff","machine":"kona","travail":"6f9619ff-8b86-d011-b42d-00c04fc964fe","nom":"x","page":1,"etat":"terminee","cree_le":0}"#;
        let s: Sortie = serde_json::from_str(ancienne).unwrap();
        assert_eq!((s.machine.as_str(), s.operation()), ("kona", Operation::Decoupe));
        let s: Sortie = serde_json::from_str(&ancienne.replace("kona", "epson")).unwrap();
        assert_eq!(s.operation(), Operation::Impression);
    }

    #[test]
    fn sortie_coupee_par_un_arret_ne_repart_pas() {
        let d = std::env::temp_dir().join(format!("eko-rip-file-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let mut f = file(&[KONA, KONA]);
        f.sorties[0].etat = Etat::EnCours;
        f.ecrire(&d).unwrap();
        let relue = File::lire(&d);
        assert_eq!((relue.sorties[0].etat, relue.sorties[0].code_erreur.as_deref()), (Etat::Echec, Some("interrompu")));
        assert_eq!(relue.sorties[1].etat, Etat::EnAttente);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
