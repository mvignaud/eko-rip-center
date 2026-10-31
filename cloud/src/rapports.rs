//! Rapports de bug des installations E-KO RIP (`POST /api/rapports`).
//!
//! Deux sortes : « Signaler un problème » (rapport `manuel`, écrit par quelqu'un) et les
//! rapports automatiques (`plantage`, `erreur`, `lenteur`), envoyés seulement par les
//! installations dont l'administrateur a donné son accord.
//!
//! Chaque rapport est gardé ici, en privé, 90 jours (`rapports/<référence>.json`), journal et
//! e-mail compris ; la page Administration du portail les montre. La plate-forme ouvre aussi une
//! issue GitHub (dépôt destiné à devenir public) qui ne porte QUE le titre, la description, le
//! type de poste, la version, l'OS et la référence : jamais le journal ni l'e-mail. Les rapports
//! automatiques de même signature (type de poste, nature, code) partagent une seule issue, avec
//! un commentaire de compteur au plus une fois par jour.
//!
//! GitHub : jeton (droit « Issues : lecture et écriture » sur le seul dépôt) et dépôt
//! `propriétaire/nom` réglés dans Administration ; sans eux, les rapports sont seulement gardés.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

pub const NATURES: &[&str] = &["manuel", "plantage", "erreur", "lenteur"];
pub const CONSERVATION_S: u64 = 90 * 86_400;
pub const MAX_JOURNAL: usize = 512 * 1024;
/// Taille maximale du corps de `POST /api/rapports`.
pub const MAX_CORPS: usize = 640 * 1024;

/// Écritures de `signatures.json` l'une après l'autre.
static SIGNATURES: Mutex<()> = Mutex::new(());

#[derive(Debug, Deserialize)]
pub struct Demande {
    /// Identifiant d'installation (celui du ping), facultatif.
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type")]
    pub type_poste: String,
    pub version: String,
    pub os: String,
    pub nature: String,
    /// Code de l'erreur (`rendu_echoue`…) ; vide pour un rapport manuel.
    #[serde(default)]
    pub code: String,
    pub titre: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub email: String,
    /// Version du serveur RIP quand le rapport vient d'un client par son serveur.
    #[serde(default)]
    pub serveur: String,
    #[serde(default)]
    pub journal: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rapport {
    pub reference: String,
    pub recu_le: u64,
    #[serde(rename = "type")]
    pub type_poste: String,
    pub version: String,
    pub os: String,
    pub nature: String,
    pub code: String,
    pub titre: String,
    pub description: String,
    pub email: String,
    pub serveur: String,
    pub journal: String,
    #[serde(default)]
    pub issue: Option<String>,
}

fn propre(s: &str, max: usize, multiligne: bool) -> Option<String> {
    let t = s.trim();
    (t.chars().count() <= max && !t.chars().any(|c| c.is_control() && !(multiligne && matches!(c, '\n' | '\t' | '\r')))).then(|| t.replace('\r', ""))
}

/// Vérifie et met en forme une demande ; None si elle est invalide.
pub fn valider(d: &Demande, maintenant: u64) -> Option<Rapport> {
    if !crate::installations::TYPES.contains(&d.type_poste.as_str()) || !NATURES.contains(&d.nature.as_str()) {
        return None;
    }
    if !d.id.is_empty() && !(d.id.len() == 32 && d.id.bytes().all(|o| o.is_ascii_digit() || (b'a'..=b'f').contains(&o))) {
        return None;
    }
    let code = propre(&d.code, 64, false)?;
    if !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || (d.nature != "manuel" && code.is_empty()) {
        return None;
    }
    let titre = propre(&d.titre, 120, false).filter(|t| !t.is_empty())?;
    let description = propre(&d.description, 5000, true)?;
    if d.nature == "manuel" && description.is_empty() {
        return None;
    }
    let email = propre(&d.email, 120, false)?;
    if !email.is_empty() && !(email.contains('@') && !email.contains(char::is_whitespace)) {
        return None;
    }
    if d.journal.len() > MAX_JOURNAL {
        return None;
    }
    Some(Rapport {
        reference: nouvelle_reference(),
        recu_le: maintenant,
        type_poste: d.type_poste.clone(),
        version: propre(&d.version, 32, false).filter(|v| !v.is_empty())?,
        os: propre(&d.os, 64, false).filter(|v| !v.is_empty())?,
        nature: d.nature.clone(),
        code,
        titre,
        description,
        email,
        serveur: propre(&d.serveur, 32, false)?,
        journal: d.journal.chars().filter(|c| !c.is_control() || matches!(c, '\n' | '\t')).collect(),
        issue: None,
    })
}

/// `R-` et 8 caractères sans ambiguïté.
pub fn nouvelle_reference() -> String {
    const A: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).expect("générateur aléatoire");
    format!("R-{}", b.iter().map(|o| A[(*o as usize) % A.len()] as char).collect::<String>())
}

pub fn dossier(donnees: &Path) -> PathBuf {
    donnees.join("rapports")
}

fn chemin(donnees: &Path, reference: &str) -> Option<PathBuf> {
    let ok = reference.len() == 10 && reference.starts_with("R-") && reference[2..].bytes().all(|o| o.is_ascii_uppercase() || o.is_ascii_digit());
    ok.then(|| dossier(donnees).join(format!("{reference}.json")))
}

fn ecrire_fichier(chemin: &Path, contenu: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(d) = chemin.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = chemin.with_extension("tmp");
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    let mut f = o.open(&tmp)?;
    f.write_all(contenu)?;
    f.sync_all()?;
    std::fs::rename(&tmp, chemin)
}

pub fn enregistrer(donnees: &Path, r: &Rapport) -> std::io::Result<()> {
    let c = chemin(donnees, &r.reference).ok_or(std::io::ErrorKind::InvalidInput)?;
    ecrire_fichier(&c, &serde_json::to_vec_pretty(r).expect("sérialisation du rapport"))
}

pub fn lire(donnees: &Path, reference: &str) -> Option<Rapport> {
    serde_json::from_slice(&std::fs::read(chemin(donnees, reference)?).ok()?).ok()
}

/// Les rapports gardés, du plus récent au plus ancien.
pub fn lister(donnees: &Path) -> Vec<Rapport> {
    let mut v: Vec<Rapport> = std::fs::read_dir(dossier(donnees))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
        .collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.recu_le));
    v
}

/// Efface les rapports de plus de 90 jours.
pub fn purger(donnees: &Path, maintenant: u64) {
    for r in lister(donnees) {
        if maintenant.saturating_sub(r.recu_le) > CONSERVATION_S
            && let Some(c) = chemin(donnees, &r.reference)
        {
            let _ = std::fs::remove_file(c);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Issues GitHub

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    pub issue: u64,
    pub url: String,
    /// Occurrences pas encore signalées en commentaire.
    pub en_attente: u32,
    pub dernier_commentaire: u64,
}

fn fichier_signatures(donnees: &Path) -> PathBuf {
    donnees.join("signatures.json")
}

fn lire_signatures(donnees: &Path) -> HashMap<String, Signature> {
    std::fs::read(fichier_signatures(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

fn ecrire_signatures(donnees: &Path, s: &HashMap<String, Signature>) {
    let _ = ecrire_fichier(&fichier_signatures(donnees), &serde_json::to_vec_pretty(s).expect("signatures"));
}

pub fn signature(r: &Rapport) -> Option<String> {
    (r.nature != "manuel").then(|| format!("{}:{}:{}", r.type_poste, r.nature, r.code))
}

/// Texte venu d'un atelier, rendu inoffensif dans une issue publique : pas de mention
/// (`@nom` notifierait quelqu'un), pas de référence d'issue (`#12`), cité ligne à ligne.
pub fn neutraliser(s: &str) -> String {
    s.replace('@', "@\u{200b}").replace('#', "#\u{200b}").replace('<', "&lt;").replace('>', "&gt;")
}

fn citer(s: &str) -> String {
    s.lines().map(|l| format!("> {}", neutraliser(l))).collect::<Vec<_>>().join("\n")
}

/// Titre et corps de l'issue : jamais le journal ni l'e-mail.
pub fn issue(r: &Rapport) -> (String, String) {
    let poste = match r.serveur.as_str() {
        "" => format!("{} {}", r.type_poste, r.version),
        s => format!("{} {} (serveur {s})", r.type_poste, r.version),
    };
    let titre = if r.nature == "manuel" { format!("[Rapport] {}", neutraliser(&r.titre)) } else { format!("[Auto] {} {} ({})", r.nature, r.code, r.type_poste) };
    let mut corps = String::new();
    if r.nature == "manuel" {
        corps.push_str(&citer(&r.description));
        corps.push_str("\n\n");
    } else {
        corps.push_str(&format!("Rapport automatique : **{}**, code `{}`.\n\n", r.nature, r.code));
    }
    corps.push_str(&format!(
        "| | |\n|---|---|\n| Poste | {} |\n| Système | {} |\n| Référence | {} |\n\n_Envoyé depuis E-KO RIP. Le journal, s'il a été joint, est gardé en privé sur la plate-forme (Administration › Rapports)._",
        neutraliser(&poste),
        neutraliser(&r.os),
        r.reference
    ));
    (titre, corps)
}

pub struct Github {
    pub jeton: String,
    /// `propriétaire/nom`.
    pub depot: String,
    /// `https://api.github.com` (changé par les tests).
    pub api: String,
}

impl Github {
    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(20))).user_agent(format!("eko-rip-cloud/{}", env!("CARGO_PKG_VERSION"))).build().into()
    }

    fn poster(&self, chemin: &str, corps: serde_json::Value) -> Option<serde_json::Value> {
        let mut rep = Self::agent()
            .post(format!("{}/repos/{}/{chemin}", self.api, self.depot))
            .header("Authorization", format!("Bearer {}", self.jeton))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send_json(corps)
            .ok()?;
        if rep.status().as_u16() != 201 {
            eprintln!("GitHub : {} sur {chemin}", rep.status());
            return None;
        }
        rep.body_mut().read_json().ok()
    }

    fn ouvrir(&self, r: &Rapport) -> Option<(u64, String)> {
        let (titre, corps) = issue(r);
        let v = self.poster("issues", serde_json::json!({ "title": titre, "body": corps }))?;
        Some((v["number"].as_u64()?, v["html_url"].as_str()?.to_string()))
    }

    fn commenter(&self, numero: u64, texte: &str) -> bool {
        self.poster(&format!("issues/{numero}/comments"), serde_json::json!({ "body": texte })).is_some()
    }
}

/// Ouvre l'issue d'un rapport (ou complète celle de sa signature) et note son adresse dans le
/// rapport. Bloquant : à lancer hors du fil asynchrone.
pub fn publier(donnees: &Path, gh: &Github, mut r: Rapport, maintenant: u64) {
    let url = match signature(&r) {
        None => gh.ouvrir(&r).map(|(_, u)| u),
        Some(sig) => {
            let _g = SIGNATURES.lock().unwrap_or_else(|p| p.into_inner());
            let mut toutes = lire_signatures(donnees);
            match toutes.get_mut(&sig) {
                Some(s) => {
                    s.en_attente += 1;
                    if maintenant.saturating_sub(s.dernier_commentaire) >= 86_400 {
                        let texte = format!("{} nouvelle(s) occurrence(s), dernière : {} {} sur {} (référence {}).", s.en_attente, r.type_poste, neutraliser(&r.version), neutraliser(&r.os), r.reference);
                        if gh.commenter(s.issue, &texte) {
                            s.en_attente = 0;
                            s.dernier_commentaire = maintenant;
                        }
                    }
                    let u = s.url.clone();
                    ecrire_signatures(donnees, &toutes);
                    Some(u)
                }
                None => gh.ouvrir(&r).inspect(|(n, u)| {
                    toutes.insert(sig, Signature { issue: *n, url: u.clone(), en_attente: 0, dernier_commentaire: maintenant });
                    ecrire_signatures(donnees, &toutes);
                }).map(|(_, u)| u),
            }
        }
    };
    if let Some(u) = url {
        r.issue = Some(u);
        let _ = enregistrer(donnees, &r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn demande() -> Demande {
        Demande {
            id: String::new(),
            type_poste: "client-mac".into(),
            version: "0.30.0".into(),
            os: "macOS 26.1.0".into(),
            nature: "manuel".into(),
            code: String::new(),
            titre: "L'aperçu reste blanc".into(),
            description: "Ligne 1\n@quelqu1 voir #3".into(),
            email: "atelier@exemple.fr".into(),
            serveur: "0.30.0".into(),
            journal: "rendu de <fichier> échoué".into(),
        }
    }

    #[test]
    fn valide_et_refuse() {
        let r = valider(&demande(), 10).unwrap();
        assert!(r.reference.starts_with("R-") && r.reference.len() == 10);
        assert!(chemin(Path::new("/x"), &r.reference).is_some());
        assert!(chemin(Path::new("/x"), "R-../../et").is_none());
        for d in [
            Demande { type_poste: "autre".into(), ..demande() },
            Demande { nature: "x".into(), ..demande() },
            Demande { titre: " ".into(), ..demande() },
            Demande { description: String::new(), ..demande() },
            Demande { email: "pas un e-mail".into(), ..demande() },
            Demande { nature: "erreur".into(), code: String::new(), ..demande() },
            Demande { nature: "erreur".into(), code: "a b".into(), ..demande() },
            Demande { id: "court".into(), ..demande() },
            Demande { titre: "a\u{7}b".into(), ..demande() },
            Demande { journal: "x".repeat(MAX_JOURNAL + 1), ..demande() },
        ] {
            assert!(valider(&d, 10).is_none(), "{d:?}");
        }
        assert!(valider(&Demande { nature: "erreur".into(), code: "rendu_echoue".into(), description: String::new(), ..demande() }, 10).is_some());
    }

    #[test]
    fn issue_sans_journal_ni_email() {
        let r = valider(&demande(), 10).unwrap();
        let (titre, corps) = issue(&r);
        assert_eq!(titre, "[Rapport] L'aperçu reste blanc");
        assert!(!corps.contains("atelier@exemple.fr") && !corps.contains("fichier") && !corps.contains("@quelqu1") && !corps.contains("#3"));
        assert!(corps.contains("> Ligne 1") && corps.contains(&r.reference) && corps.contains("client-mac 0.30.0 (serveur 0.30.0)"));
        let auto = valider(&Demande { nature: "plantage".into(), code: "panique".into(), type_poste: "serveur".into(), serveur: String::new(), ..demande() }, 10).unwrap();
        assert_eq!(signature(&auto).as_deref(), Some("serveur:plantage:panique"));
        assert_eq!(issue(&auto).0, "[Auto] plantage panique (serveur)");
        assert!(signature(&r).is_none());
    }

    #[test]
    fn garder_lister_purger() {
        let d = std::env::temp_dir().join(format!("eko-cloud-rapports-{}", crate::aleatoire_hex(6)));
        let a = valider(&demande(), 10).unwrap();
        let b = valider(&demande(), 20).unwrap();
        enregistrer(&d, &a).unwrap();
        enregistrer(&d, &b).unwrap();
        assert_eq!(lister(&d).iter().map(|r| r.recu_le).collect::<Vec<_>>(), vec![20, 10]);
        assert_eq!(lire(&d, &a.reference).unwrap(), a);
        purger(&d, 15 + CONSERVATION_S);
        assert_eq!(lister(&d).len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }
}
