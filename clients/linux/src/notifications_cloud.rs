//! Notifications du cloud : suivi du curseur `dernier`/`apres` et choix de la cible du clic.
//! Module pur (pas de réseau, pas de GTK), testable à part. L'interrogation toutes les 60 s
//! et l'affichage sont dans `fenetre.rs` ; l'appel HTTP dans `cloud.rs`.

use crate::cloud::Site;
use serde::Deserialize;

/// Une notification de `GET /api/moi/notifications`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct NotificationCloud {
    pub id: u64,
    #[serde(default, rename = "type")]
    pub genre: String,
    #[serde(default)]
    pub site: Option<String>,
    #[serde(default)]
    pub titre: String,
    #[serde(default)]
    pub corps: String,
    #[serde(default)]
    pub lien: String,
}

/// Réponse de `GET /api/moi/notifications`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct ReponseNotifications {
    #[serde(default)]
    pub notifications: Vec<NotificationCloud>,
    #[serde(default)]
    pub dernier: u64,
}

/// Suite à donner à une réponse.
#[derive(Debug, Default, PartialEq)]
pub struct Suite {
    /// Notifications nouvelles, à afficher, dans l'ordre.
    pub a_afficher: Vec<NotificationCloud>,
    /// Il en reste côté plate-forme : relancer tout de suite avec le nouvel `apres`.
    pub relancer: bool,
}

/// Curseur : id de la dernière notification traitée. `None` tant que rien n'est mémorisé
/// (premier appel : sans `apres`, sans rattrapage).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Curseur {
    apres: Option<u64>,
}

impl Curseur {
    /// Relit la valeur persistée ; une valeur illisible vaut « rien de mémorisé ».
    pub fn depuis(valeur: Option<&str>) -> Self {
        Self { apres: valeur.and_then(|v| v.trim().parse().ok()) }
    }

    pub fn apres(&self) -> Option<u64> {
        self.apres
    }

    /// Valeur à persister, si le curseur est amorcé.
    pub fn a_retenir(&self) -> Option<String> {
        self.apres.map(|n| n.to_string())
    }

    /// Absorbe une réponse. Sans curseur : on mémorise `dernier`, rien à afficher. Avec :
    /// on ne garde que les ids au-delà du curseur et on avance jusqu'au dernier reçu.
    pub fn absorber(&mut self, rep: ReponseNotifications) -> Suite {
        let Some(apres) = self.apres else {
            self.apres = Some(rep.dernier);
            return Suite::default();
        };
        let nouvelles: Vec<_> = rep.notifications.into_iter().filter(|n| n.id > apres).collect();
        let Some(dernier_recu) = nouvelles.iter().map(|n| n.id).max() else {
            // Rien d'affichable : on rattrape `dernier` pour ne pas redemander les mêmes ids.
            self.apres = Some(apres.max(rep.dernier));
            return Suite::default();
        };
        self.apres = Some(dernier_recu);
        Suite { relancer: rep.dernier > dernier_recu, a_afficher: nouvelles }
    }
}

/// Ce que fait le clic sur une notification.
#[derive(Debug, PartialEq, Eq)]
pub enum Cible {
    /// Ouvrir ce site du compte dans l'app.
    Site(String),
    /// Ouvrir cette adresse dans le navigateur.
    Lien(String),
}

/// Site connu du compte → l'ouvrir ; sinon le lien (s'il y en a un).
pub fn cible(site: Option<&str>, lien: &str, sites: &[Site]) -> Option<Cible> {
    if let Some(s) = site.filter(|s| !s.is_empty()) && sites.iter().any(|x| x.id == s) {
        return Some(Cible::Site(s.to_string()));
    }
    let lien = lien.trim();
    (!lien.is_empty()).then(|| Cible::Lien(lien.to_string()))
}

/// Paramètre de l'action `app.ouvrir-notification-cloud` : `site` (vide si aucun) et `lien`,
/// séparés par un saut de ligne (ni l'un ni l'autre n'en contient).
pub fn encoder_cible(site: Option<&str>, lien: &str) -> String {
    format!("{}\n{}", site.unwrap_or_default(), lien)
}

pub fn decoder_cible(parametre: &str) -> (Option<String>, String) {
    let (site, lien) = parametre.split_once('\n').unwrap_or(("", parametre));
    ((!site.is_empty()).then(|| site.to_string()), lien.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(id: u64) -> NotificationCloud {
        NotificationCloud { id, titre: format!("t{id}"), lien: "https://x/y".into(), ..Default::default() }
    }

    fn rep(ids: &[u64], dernier: u64) -> ReponseNotifications {
        ReponseNotifications { notifications: ids.iter().map(|i| n(*i)).collect(), dernier }
    }

    fn site(id: &str) -> Site {
        Site { id: id.into(), ..Default::default() }
    }

    #[test]
    fn premier_appel_memorise_sans_rattrapage() {
        let mut c = Curseur::default();
        assert_eq!(c.apres(), None);
        let s = c.absorber(rep(&[], 42));
        assert!(s.a_afficher.is_empty() && !s.relancer);
        assert_eq!(c.apres(), Some(42));
        assert_eq!(c.a_retenir().as_deref(), Some("42"));
    }

    #[test]
    fn premier_appel_ignore_meme_des_notifications() {
        let mut c = Curseur::default();
        assert!(c.absorber(rep(&[1, 2], 2)).a_afficher.is_empty());
        assert_eq!(c.apres(), Some(2));
    }

    #[test]
    fn valeur_persistee() {
        assert_eq!(Curseur::depuis(Some("17\n")).apres(), Some(17));
        assert_eq!(Curseur::depuis(Some("abc")).apres(), None);
        assert_eq!(Curseur::depuis(None).apres(), None);
    }

    #[test]
    fn nouvelles_notifications_avancent_le_curseur() {
        let mut c = Curseur::depuis(Some("10"));
        let s = c.absorber(rep(&[11, 12], 12));
        assert_eq!(s.a_afficher.iter().map(|x| x.id).collect::<Vec<_>>(), [11, 12]);
        assert!(!s.relancer);
        assert_eq!(c.apres(), Some(12));
    }

    #[test]
    fn relance_si_dernier_depasse() {
        let mut c = Curseur::depuis(Some("10"));
        let s = c.absorber(rep(&[11, 12], 80));
        assert!(s.relancer);
        assert_eq!(c.apres(), Some(12));
    }

    #[test]
    fn ids_deja_vus_ne_sont_pas_reaffiches() {
        let mut c = Curseur::depuis(Some("12"));
        let s = c.absorber(rep(&[11, 12, 13], 13));
        assert_eq!(s.a_afficher.iter().map(|x| x.id).collect::<Vec<_>>(), [13]);
    }

    #[test]
    fn liste_vide_rattrape_dernier_sans_boucle() {
        let mut c = Curseur::depuis(Some("10"));
        let s = c.absorber(rep(&[], 30));
        assert!(s.a_afficher.is_empty() && !s.relancer);
        assert_eq!(c.apres(), Some(30));
        // Un `dernier` en retard ne fait jamais reculer le curseur.
        c.absorber(rep(&[], 5));
        assert_eq!(c.apres(), Some(30));
    }

    #[test]
    fn cible_site_connu_sinon_lien() {
        let sites = [site("a"), site("b")];
        assert_eq!(cible(Some("b"), "https://l", &sites), Some(Cible::Site("b".into())));
        assert_eq!(cible(Some("z"), "https://l", &sites), Some(Cible::Lien("https://l".into())));
        assert_eq!(cible(None, "https://l", &sites), Some(Cible::Lien("https://l".into())));
        assert_eq!(cible(Some(""), "https://l", &sites), Some(Cible::Lien("https://l".into())));
        assert_eq!(cible(Some("z"), " ", &sites), None);
        assert_eq!(cible(Some("a"), "", &sites), Some(Cible::Site("a".into())));
    }

    #[test]
    fn parametre_aller_retour() {
        let p = encoder_cible(Some("s1"), "https://e/x?a=1");
        assert_eq!(decoder_cible(&p), (Some("s1".into()), "https://e/x?a=1".into()));
        let p = encoder_cible(None, "https://e");
        assert_eq!(decoder_cible(&p), (None, "https://e".into()));
    }

    #[test]
    fn decodage_json() {
        let r: ReponseNotifications = serde_json::from_str(
            r#"{"notifications":[{"id":3,"type":"site_hors_ligne","site":null,"titre":"T","corps":"C","lien":"https://p/s","cree_le":"2026-01-01"}],"dernier":3}"#,
        )
        .unwrap();
        assert_eq!(r.dernier, 3);
        assert_eq!(r.notifications[0].genre, "site_hors_ligne");
        assert_eq!(r.notifications[0].site, None);
    }
}
