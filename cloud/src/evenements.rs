//! Journal des événements par site (coupures du tunnel, alertes envoyées) et envois qui en
//! découlent : alerte « hors ligne » après 15 minutes, « de retour en ligne », résumé
//! hebdomadaire du lundi 8 h (heure de Paris). Table `evenements_sites`, gardée 90 jours.
//!
//! Les fonctions de ce module qui prennent une `Connection` et une date sont pures et
//! testables ; `entretenir` (appelée chaque minute par `relais::entretenir`) fait les envois.

use crate::base;
use crate::courriels;
use crate::notifications::{self, Notification};
use crate::relais::Partage;
use crate::sites::{self, Site};
use rusqlite::{Connection, OptionalExtension, params};

/// Un site doit rester hors ligne aussi longtemps avant l'alerte.
pub const DELAI_ALERTE: u64 = 15 * 60;
/// Au plus une alerte par site et par heure.
pub const INTERVALLE_ALERTES: u64 = 3600;
/// Conservation du journal.
pub const CONSERVATION: u64 = 90 * 24 * 3600;
/// Les coupures plus courtes ne comptent pas dans le résumé (reconnexions instantanées).
pub const COUPURE_MIN: u64 = 60;
/// Clé de `reglages` : début (UTC) du dernier lundi 8 h pour lequel le résumé est parti.
pub const CLE_SEMAINE: &str = "resume_derniere_semaine";

/// Le tunnel du site vient de se fermer : début d'une coupure (une seule ouverte à la fois).
pub fn tunnel_ferme(c: &Connection, site: &str, t: u64) -> rusqlite::Result<()> {
    let en_cours: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM evenements_sites WHERE site = ?1 AND type = 'coupure' AND fin IS NULL)", [site], |r| r.get(0))?;
    if !en_cours {
        c.execute("INSERT INTO evenements_sites (site, type, debut) VALUES (?1, 'coupure', ?2)", params![site, t as i64])?;
    }
    Ok(())
}

/// Le tunnel du site est ouvert : la coupure en cours se termine. Rend son début si une alerte
/// « hors ligne » est partie pour elle (alors seulement, on envoie « de retour en ligne »).
pub fn tunnel_ouvert(c: &Connection, site: &str, t: u64) -> rusqlite::Result<Option<u64>> {
    let Some(debut) = c.query_row("SELECT debut FROM evenements_sites WHERE site = ?1 AND type = 'coupure' AND fin IS NULL ORDER BY id DESC LIMIT 1", [site], |r| r.get::<_, i64>(0)).optional()? else { return Ok(None) };
    c.execute("UPDATE evenements_sites SET fin = ?1 WHERE site = ?2 AND type = 'coupure' AND fin IS NULL", params![t as i64, site])?;
    let alerte: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM evenements_sites WHERE site = ?1 AND type = 'alerte' AND detail = 'envoyee' AND debut >= ?2)", params![site, debut], |r| r.get(0))?;
    Ok(alerte.then_some(debut as u64))
}

/// Coupures en cours depuis `DELAI_ALERTE` sans alerte (envoyée ou non) : (site, début).
pub fn candidats_alerte(c: &Connection, t: u64) -> rusqlite::Result<Vec<(String, u64)>> {
    let mut q = c.prepare("SELECT site, debut FROM evenements_sites e WHERE type = 'coupure' AND fin IS NULL AND debut <= ?1 AND NOT EXISTS (SELECT 1 FROM evenements_sites a WHERE a.site = e.site AND a.type = 'alerte' AND a.debut >= e.debut)")?;
    q.query_map([t.saturating_sub(DELAI_ALERTE) as i64], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)))?.collect()
}

/// Pas de rafale : aucune alerte envoyée pour ce site depuis une heure.
pub fn alerte_permise(c: &Connection, site: &str, t: u64) -> rusqlite::Result<bool> {
    let recente: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM evenements_sites WHERE site = ?1 AND type = 'alerte' AND detail = 'envoyee' AND debut > ?2)", params![site, t.saturating_sub(INTERVALLE_ALERTES) as i64], |r| r.get(0))?;
    Ok(!recente)
}

/// Note le traitement de la coupure en cours (envoyée ou ignorée : on ne la réexamine plus).
pub fn noter_alerte(c: &Connection, site: &str, t: u64, envoyee: bool) -> rusqlite::Result<()> {
    c.execute("INSERT INTO evenements_sites (site, type, debut, detail) VALUES (?1, 'alerte', ?2, ?3)", params![site, t as i64, if envoyee { "envoyee" } else { "ignoree" }])?;
    Ok(())
}

/// Comptes propriétaires ou administrateurs du site, non suspendus, qui reçoivent les alertes.
pub fn destinataires(c: &Connection, site: &str) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut q = c.prepare("SELECT c.id, c.email, c.nom FROM droits d JOIN comptes c ON c.id = d.compte WHERE d.site = ?1 AND d.role IN ('proprietaire', 'admin') AND c.suspendu = 0 AND c.alertes = 1 ORDER BY c.id")?;
    q.query_map([site], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect()
}

pub fn purger(c: &Connection, t: u64) -> rusqlite::Result<()> {
    c.execute("DELETE FROM evenements_sites WHERE debut < ?1", [t.saturating_sub(CONSERVATION) as i64])?;
    Ok(())
}

// ————— Résumé hebdomadaire —————

#[derive(Clone, Debug, PartialEq)]
pub struct ResumeSite {
    pub nom: String,
    pub coupures: usize,
    /// Durée totale hors ligne, en secondes.
    pub hors_ligne: u64,
    pub plus_longue: u64,
    pub alertes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResumeCompte {
    pub nom: String,
    pub email: String,
    pub sites: Vec<ResumeSite>,
    /// Apps connectées dans la semaine : (appareil, date).
    pub applis: Vec<(String, u64)>,
}

/// Coupures `(début, fin)` d'une semaine `[debut, fin[` : (nombre, durée totale, plus longue).
/// Les durées sont ramenées à la semaine ; une coupure de moins d'une minute ne compte pas ;
/// une coupure encore ouverte dure jusqu'à la fin de la semaine.
pub fn agreger_coupures(coupures: &[(u64, Option<u64>)], debut: u64, fin: u64) -> (usize, u64, u64) {
    let (mut n, mut total, mut longue) = (0, 0, 0);
    for &(d, f) in coupures {
        let f = f.unwrap_or(fin);
        if d >= fin || f <= debut || f.saturating_sub(d) < COUPURE_MIN {
            continue;
        }
        let duree = f.min(fin) - d.max(debut);
        n += 1;
        total += duree;
        longue = longue.max(duree);
    }
    (n, total, longue)
}

fn resume_site(c: &Connection, site: &str, nom: &str, debut: u64, fin: u64) -> rusqlite::Result<ResumeSite> {
    let mut q = c.prepare("SELECT debut, fin FROM evenements_sites WHERE site = ?1 AND type = 'coupure' AND debut < ?3 AND (fin IS NULL OR fin > ?2)")?;
    let coupures: Vec<(u64, Option<u64>)> = q.query_map(params![site, debut as i64, fin as i64], |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, Option<i64>>(1)?.map(|f| f as u64))))?.collect::<rusqlite::Result<_>>()?;
    let (n, total, longue) = agreger_coupures(&coupures, debut, fin);
    let alertes: i64 = c.query_row("SELECT COUNT(*) FROM evenements_sites WHERE site = ?1 AND type = 'alerte' AND detail = 'envoyee' AND debut >= ?2 AND debut < ?3", params![site, debut as i64, fin as i64], |r| r.get(0))?;
    Ok(ResumeSite { nom: nom.into(), coupures: n, hors_ligne: total, plus_longue: longue, alertes: alertes as usize })
}

/// Un résumé par compte propriétaire/administrateur (non suspendu, qui l'a activé) ayant au
/// moins un site connu de `sites`, pour la semaine `[debut, fin[`.
pub fn resumes(c: &Connection, sites: &[Site], debut: u64, fin: u64) -> rusqlite::Result<Vec<ResumeCompte>> {
    let comptes: Vec<(i64, String, String)> = c.prepare("SELECT DISTINCT c.id, c.email, c.nom FROM comptes c JOIN droits d ON d.compte = c.id WHERE d.role IN ('proprietaire', 'admin') AND c.suspendu = 0 AND c.resume = 1 ORDER BY c.id")?.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
    let mut sortie = Vec::new();
    for (id, email, nom) in comptes {
        let ids: Vec<String> = c.prepare("SELECT site FROM droits WHERE compte = ?1 AND role IN ('proprietaire', 'admin') ORDER BY site")?.query_map([id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let mut par_site = Vec::new();
        for s in ids {
            if let Some(site) = sites.iter().find(|x| x.id == s) {
                par_site.push(resume_site(c, &s, &site.nom, debut, fin)?);
            }
        }
        if par_site.is_empty() {
            continue;
        }
        let applis: Vec<(String, u64)> = c.prepare("SELECT appareil, cree_le FROM jetons_applis WHERE compte = ?1 AND cree_le >= ?2 AND cree_le < ?3 ORDER BY cree_le")?.query_map(params![id, debut as i64, fin as i64], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)))?.collect::<rusqlite::Result<_>>()?;
        sortie.push(ResumeCompte { nom, email, sites: par_site, applis });
    }
    Ok(sortie)
}

/// La semaine à résumer à l'instant `t`, une seule fois : `[lundi 8 h précédent, dernier lundi 8 h[`.
/// Le dernier lundi traité est mémorisé dans `reglages` avant l'envoi (au plus un envoi par
/// semaine, même si le service redémarre). Au tout premier passage, on mémorise sans envoyer :
/// le premier résumé part au lundi suivant.
pub fn semaine_a_envoyer(c: &Connection, t: u64) -> rusqlite::Result<Option<(u64, u64)>> {
    let lundi = crate::temps::lundi_8h_precedent(t);
    let dernier = base::reglage(c, CLE_SEMAINE)?.and_then(|v| v.parse::<u64>().ok());
    match dernier {
        Some(d) if d >= lundi => Ok(None),
        None => {
            base::regler(c, CLE_SEMAINE, &lundi.to_string())?;
            Ok(None)
        }
        Some(_) => {
            base::regler(c, CLE_SEMAINE, &lundi.to_string())?;
            Ok(Some((crate::temps::lundi_8h_precedent(lundi - 1), lundi)))
        }
    }
}

// ————— Tâches du relais —————

fn nom_du_site(r: &Partage, site: &str) -> String {
    sites::lire(&r.config.donnees).ok().and_then(|l| l.into_iter().find(|s| s.id == site)).map_or_else(|| site.to_string(), |s| s.nom)
}

/// Le tunnel du site vient de se fermer.
pub fn site_hors_ligne(r: &Partage, site: &str) {
    if let Err(e) = tunnel_ferme(&r.base(), site, crate::maintenant()) {
        eprintln!("journal des sites : {e}");
    }
}

/// Le tunnel du site vient de s'ouvrir : « de retour en ligne » si l'alerte hors ligne est partie.
pub fn site_en_ligne(r: &Partage, site: &str) {
    let t = crate::maintenant();
    let (retour, dest) = {
        let b = r.base();
        let retour = tunnel_ouvert(&b, site, t).unwrap_or_else(|e| {
            eprintln!("journal des sites : {e}");
            None
        });
        (retour, destinataires(&b, site).unwrap_or_default())
    };
    let Some(debut) = retour else { return };
    let nom = nom_du_site(r, site);
    for (compte, email, qui) in dest {
        let courriel = courriels::site_en_ligne(&r.config.url_publique, &email, &qui, &nom, debut, t);
        notifications::notifier(r, compte, Some(courriel), Notification::site_en_ligne(&r.config.url_publique, site, &nom, debut, t));
    }
}

/// Chaque minute : alertes « hors ligne » à envoyer, purge du journal, résumé du lundi 8 h.
pub async fn entretenir(r: &Partage) {
    let t = crate::maintenant();
    alertes(r, t);
    resume(r, t);
}

fn alertes(r: &Partage, t: u64) {
    let candidats = {
        let b = r.base();
        let _ = purger(&b, t);
        let _ = notifications::purger(&b, t);
        candidats_alerte(&b, t).unwrap_or_default()
    };
    if candidats.is_empty() {
        return;
    }
    let connus = sites::lire(&r.config.donnees).unwrap_or_default();
    let connectes = r.connectes();
    for (site, debut) in candidats {
        let Some(s) = connus.iter().find(|s| s.id == site) else { continue };
        if connectes.contains(&site) {
            continue;
        }
        let (dest, permise) = {
            let b = r.base();
            (destinataires(&b, &site).unwrap_or_default(), alerte_permise(&b, &site, t).unwrap_or(false))
        };
        // Le journal et les pushs partent même sans SMTP : seul l'e-mail dépend de ses réglages.
        let envoyee = permise && !dest.is_empty();
        if let Err(e) = noter_alerte(&r.base(), &site, t, envoyee) {
            eprintln!("journal des sites : {e}");
            continue;
        }
        if envoyee {
            for (compte, email, qui) in dest {
                let courriel = courriels::site_hors_ligne(&r.config.url_publique, &email, &qui, &s.nom, debut, t);
                notifications::notifier(r, compte, Some(courriel), Notification::site_hors_ligne(&r.config.url_publique, &site, &s.nom, debut, t));
            }
        } else {
            eprintln!("alerte hors ligne du site {site} non envoyée ({})", if !permise { "déjà une alerte dans l'heure" } else { "aucun destinataire" });
        }
    }
}

fn resume(r: &Partage, t: u64) {
    let (semaine, comptes) = {
        let b = r.base();
        let Ok(Some((debut, fin))) = semaine_a_envoyer(&b, t) else { return };
        let connus = sites::lire(&r.config.donnees).unwrap_or_default();
        ((debut, fin), resumes(&b, &connus, debut, fin).unwrap_or_default())
    };
    if !r.courriels_actifs() {
        eprintln!("résumé hebdomadaire non envoyé (e-mails non réglés)");
        return;
    }
    for c in comptes {
        r.envoyer(courriels::resume_hebdomadaire(&r.config.url_publique, &c, semaine.0, semaine.1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_test() -> (std::path::PathBuf, Connection) {
        let d = std::env::temp_dir().join(format!("eko-cloud-evt-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        (d, c)
    }

    #[test]
    fn alerte_apres_quinze_minutes_puis_retour() {
        let (d, c) = base_test();
        let t0 = 1_000_000;
        tunnel_ferme(&c, "s1", t0).unwrap();
        tunnel_ferme(&c, "s1", t0 + 5).unwrap();
        assert!(candidats_alerte(&c, t0 + DELAI_ALERTE - 1).unwrap().is_empty(), "pas avant 15 minutes");
        assert_eq!(candidats_alerte(&c, t0 + DELAI_ALERTE).unwrap(), vec![("s1".to_string(), t0)], "une seule coupure ouverte");
        assert!(alerte_permise(&c, "s1", t0 + DELAI_ALERTE).unwrap());
        noter_alerte(&c, "s1", t0 + DELAI_ALERTE, true).unwrap();
        assert!(candidats_alerte(&c, t0 + 2 * DELAI_ALERTE).unwrap().is_empty(), "une alerte par coupure");
        assert_eq!(tunnel_ouvert(&c, "s1", t0 + 1200).unwrap(), Some(t0), "retour en ligne annoncé");
        assert_eq!(tunnel_ouvert(&c, "s1", t0 + 1300).unwrap(), None, "plus de coupure en cours");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn pas_de_retour_sans_alerte_et_pas_de_rafale() {
        let (d, c) = base_test();
        // Coupure courte, jamais alertée : pas de « retour en ligne ».
        tunnel_ferme(&c, "s1", 1000).unwrap();
        assert_eq!(tunnel_ouvert(&c, "s1", 1100).unwrap(), None);
        // Première alerte à t, puis une deuxième coupure dans l'heure : alerte refusée, et donc pas de retour annoncé.
        let t = 100_000;
        tunnel_ferme(&c, "s1", t - 1000).unwrap();
        noter_alerte(&c, "s1", t, true).unwrap();
        assert_eq!(tunnel_ouvert(&c, "s1", t + 60).unwrap(), Some(t - 1000));
        tunnel_ferme(&c, "s1", t + 100).unwrap();
        let t2 = t + 100 + DELAI_ALERTE;
        assert_eq!(candidats_alerte(&c, t2).unwrap().len(), 1);
        assert!(!alerte_permise(&c, "s1", t2).unwrap(), "au plus une alerte par heure");
        noter_alerte(&c, "s1", t2, false).unwrap();
        assert_eq!(tunnel_ouvert(&c, "s1", t2 + 10).unwrap(), None, "alerte non partie : pas de retour");
        // Une heure plus tard, c'est de nouveau permis.
        assert!(alerte_permise(&c, "s1", t + INTERVALLE_ALERTES + 1).unwrap());
        // Un autre site n'est pas concerné.
        assert!(alerte_permise(&c, "s2", t2).unwrap());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn destinataires_et_purge() {
        let (d, c) = base_test();
        let mk = |e: &str| base::creer_compte(&c, e, e, None).unwrap().unwrap();
        let (p, a, o, l, s) = (mk("p@x.fr"), mk("a@x.fr"), mk("o@x.fr"), mk("l@x.fr"), mk("s@x.fr"));
        for (id, role) in [(p, "proprietaire"), (a, "admin"), (o, "operateur"), (l, "lecture"), (s, "admin")] {
            base::donner_droit(&c, "site", id, role).unwrap();
        }
        base::suspendre(&c, s, true).unwrap();
        base::regler_preferences(&c, a, false, true).unwrap();
        assert_eq!(destinataires(&c, "site").unwrap().iter().map(|x| x.1.as_str()).collect::<Vec<_>>(), ["p@x.fr"], "rôle, suspension et case « alertes »");
        base::regler_preferences(&c, a, true, true).unwrap();
        assert_eq!(destinataires(&c, "site").unwrap().len(), 2);
        // Purge à 90 jours.
        let t = 200 * 86_400;
        tunnel_ferme(&c, "vieux", t - CONSERVATION - 10).unwrap();
        tunnel_ferme(&c, "recent", t - 10).unwrap();
        purger(&c, t).unwrap();
        let n: i64 = c.query_row("SELECT COUNT(*) FROM evenements_sites", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn agregation_des_coupures() {
        let (debut, fin) = (1_000_000, 1_000_000 + 7 * 86_400);
        let v = [
            (debut - 3600, Some(debut + 1800)),  // commencée avant la semaine : 30 min comptées
            (debut + 1000, Some(debut + 1030)),  // 30 s : ignorée
            (debut + 5000, Some(debut + 5000 + 7200)), // 2 h
            (fin - 600, None),                   // encore ouverte à la fin : 10 min
            (fin + 10, Some(fin + 5000)),        // après la semaine
        ];
        assert_eq!(agreger_coupures(&v, debut, fin), (3, 1800 + 7200 + 600, 7200));
        assert_eq!(agreger_coupures(&[], debut, fin), (0, 0, 0));
    }

    #[test]
    fn contenu_du_resume() {
        let (d, c) = base_test();
        let id = base::creer_compte(&c, "chef@x.fr", "Chef", None).unwrap().unwrap();
        let autre = base::creer_compte(&c, "lecteur@x.fr", "Lecteur", None).unwrap().unwrap();
        base::donner_droit(&c, "s1", id, "proprietaire").unwrap();
        base::donner_droit(&c, "s2", id, "admin").unwrap();
        base::donner_droit(&c, "s1", autre, "lecture").unwrap();
        let sites = vec![
            Site { id: "s1".into(), nom: "Atelier <1>".into(), cle_sha256: String::new(), cree_le: 0, derniere_connexion: None, proprietaire: None },
            Site { id: "s2".into(), nom: "Atelier 2".into(), cle_sha256: String::new(), cree_le: 0, derniere_connexion: None, proprietaire: None },
        ];
        let lundi = crate::temps::lundi_8h_precedent(crate::maintenant());
        let debut = lundi - 7 * 86_400;
        tunnel_ferme(&c, "s1", debut + 3600).unwrap();
        tunnel_ouvert(&c, "s1", debut + 3600 + 5400).unwrap();
        noter_alerte(&c, "s1", debut + 3600 + 900, true).unwrap();
        c.execute("INSERT INTO jetons_applis (empreinte, compte, appareil, cree_le, expire, vu_le) VALUES ('e', ?1, 'Mac de Claire', ?2, ?3, ?2)", params![id, debut + 100, lundi + 99_999]).unwrap();
        let v = resumes(&c, &sites, debut, lundi).unwrap();
        assert_eq!(v.len(), 1, "seuls les propriétaires et administrateurs reçoivent un résumé");
        assert_eq!(v[0].email, "chef@x.fr");
        assert_eq!(v[0].sites, vec![ResumeSite { nom: "Atelier <1>".into(), coupures: 1, hors_ligne: 5400, plus_longue: 5400, alertes: 1 }, ResumeSite { nom: "Atelier 2".into(), coupures: 0, hors_ligne: 0, plus_longue: 0, alertes: 0 }]);
        assert_eq!(v[0].applis, vec![("Mac de Claire".to_string(), debut + 100)]);
        // Contenu de l'e-mail : chiffres, échappement, désactivation par la case.
        let m = courriels::resume_hebdomadaire("https://platform.e-ko.rip", &v[0], debut, lundi);
        assert!(m.texte.contains("Atelier <1>") && m.texte.contains("1 h 30") && m.texte.contains("Mac de Claire"), "{}", m.texte);
        assert!(m.html.contains("Atelier &lt;1&gt;") && !m.html.contains("Atelier <1>"));
        base::regler_preferences(&c, id, true, false).unwrap();
        assert!(resumes(&c, &sites, debut, lundi).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn une_seule_fois_par_semaine() {
        let (d, c) = base_test();
        let mercredi = crate::temps::jours_depuis_civil(2026, 11, 4) as u64 * 86_400 + 10 * 3600;
        let lundi = crate::temps::lundi_8h_precedent(mercredi);
        assert_eq!(semaine_a_envoyer(&c, mercredi).unwrap(), None, "premier passage : mémorisé, rien d'envoyé");
        assert_eq!(semaine_a_envoyer(&c, mercredi + 3600).unwrap(), None);
        // Le lundi suivant à 7 h 59 : rien ; à 8 h 00 : la semaine écoulée, une seule fois.
        let suivant = lundi + 7 * 86_400;
        assert_eq!(semaine_a_envoyer(&c, suivant - 60).unwrap(), None);
        assert_eq!(semaine_a_envoyer(&c, suivant).unwrap(), Some((lundi, suivant)));
        assert_eq!(semaine_a_envoyer(&c, suivant + 60).unwrap(), None, "pas de second envoi");
        assert_eq!(base::reglage(&c, CLE_SEMAINE).unwrap().unwrap(), suivant.to_string());
        let _ = std::fs::remove_dir_all(&d);
    }
}
