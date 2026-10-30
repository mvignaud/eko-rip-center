//! Client HTTP de l'API JSON du serveur. Tout est bloquant : les appels se font dans des
//! fils à part (`gio::spawn_blocking`), jamais dans le fil GTK. Le cookie de session reste en
//! mémoire.

use crate::libelles;
use crate::modele::{Machine, Reglage, Session, Sortie, Systeme, Travail};
use serde::de::DeserializeOwned;
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const COOKIE: &str = "eko_rip_session";

/// Erreur d'un appel : le code stable du serveur (ou `reseau`), son détail, et le libellé
/// français à afficher.
#[derive(Debug, Clone)]
pub struct Erreur {
    pub statut: u16,
    pub code: String,
    pub detail: Option<String>,
}

impl Erreur {
    pub fn locale(code: &str, detail: impl Into<Option<String>>) -> Self {
        Erreur { statut: 0, code: code.into(), detail: detail.into() }
    }

    /// Session perdue ou jamais ouverte : retour à l'écran de connexion.
    pub fn session_perdue(&self) -> bool {
        self.statut == 401
    }

    /// Libellé du code s'il est connu, sinon le code et le détail.
    pub fn message(&self) -> String {
        if let Some(statut) = self.code.strip_prefix("http_") {
            return format!("Réponse inattendue du serveur (HTTP {statut}).");
        }
        match libelles::libelle(&self.code) {
            Some(l) => l.to_string(),
            None => match self.detail.as_deref().filter(|d| !d.is_empty()) {
                Some(d) => format!("{} : {d}", self.code),
                None => self.code.clone(),
            },
        }
    }
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

pub type Resultat<T> = Result<T, Erreur>;

pub struct Client {
    base: String,
    /// Requêtes courtes : délais de lecture bornés.
    agent: ureq::Agent,
    /// Envois et téléchargements de fichiers : aucun délai de lecture (rendus longs).
    pub(crate) agent_long: ureq::Agent,
    cookie: Mutex<Option<String>>,
}

impl Client {
    pub fn new(base: &str) -> Self {
        let ua = format!("EKO-RIP-Client/{VERSION} (Linux)");
        let court = ureq::AgentBuilder::new().user_agent(&ua).timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(30)).timeout_write(Duration::from_secs(30)).build();
        let long = ureq::AgentBuilder::new().user_agent(&ua).timeout_connect(Duration::from_secs(5)).build();
        Client { base: base.trim_end_matches('/').to_string(), agent: court, agent_long: long, cookie: Mutex::new(None) }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub(crate) fn url(&self, chemin: &str) -> String {
        format!("{}{chemin}", self.base)
    }

    pub(crate) fn avec_cookie(&self, r: ureq::Request) -> ureq::Request {
        match self.cookie.lock().unwrap().as_deref() {
            Some(c) => r.set("Cookie", c),
            None => r,
        }
    }

    fn requete(&self, methode: &str, chemin: &str) -> ureq::Request {
        self.avec_cookie(self.agent.request(methode, &self.url(chemin)))
    }

    /// Réponse ou erreur décodée (`{"code","detail"}`), le cookie de session gardé au passage.
    pub(crate) fn verifier(&self, r: Result<ureq::Response, ureq::Error>) -> Resultat<ureq::Response> {
        match r {
            Ok(rep) => {
                for c in rep.all("set-cookie") {
                    if let Some(paire) = c.split(';').next().filter(|p| p.starts_with(COOKIE)) {
                        let (nom, valeur) = paire.split_once('=').unwrap_or((paire, ""));
                        *self.cookie.lock().unwrap() = if nom == COOKIE && !valeur.is_empty() { Some(paire.to_string()) } else { None };
                    }
                }
                Ok(rep)
            }
            Err(ureq::Error::Status(statut, rep)) => {
                let corps: serde_json::Value = rep.into_json().unwrap_or_default();
                let code = corps["code"].as_str().map_or_else(|| format!("http_{statut}"), str::to_string);
                Err(Erreur { statut, code, detail: corps["detail"].as_str().map(str::to_string) })
            }
            Err(ureq::Error::Transport(t)) => Err(Erreur::locale("reseau", t.to_string())),
        }
    }

    fn json<T: DeserializeOwned>(&self, chemin: &str) -> Resultat<T> {
        let rep = self.verifier(self.requete("GET", chemin).call())?;
        rep.into_json().map_err(|e| Erreur::locale("reponse_illisible", e.to_string()))
    }

    fn envoyer_json(&self, methode: &str, chemin: &str, corps: serde_json::Value) -> Resultat<()> {
        self.verifier(self.requete(methode, chemin).send_json(corps)).map(|_| ())
    }

    fn sans_corps(&self, methode: &str, chemin: &str) -> Resultat<()> {
        self.verifier(self.requete(methode, chemin).call()).map(|_| ())
    }

    // --- Session ---

    pub fn session(&self) -> Resultat<Session> {
        self.json("/api/session")
    }

    pub fn connecter(&self, nom: &str, mot_de_passe: &str) -> Resultat<()> {
        self.envoyer_json("POST", "/api/session", serde_json::json!({ "nom": nom, "mot_de_passe": mot_de_passe }))
    }

    pub fn deconnecter(&self) -> Resultat<()> {
        let r = self.sans_corps("DELETE", "/api/session");
        *self.cookie.lock().unwrap() = None;
        r
    }

    // --- Lectures ---

    pub fn travaux(&self) -> Resultat<Vec<Travail>> {
        self.json("/api/travaux")
    }

    pub fn file(&self) -> Resultat<Vec<Sortie>> {
        self.json("/api/file")
    }

    pub fn machines(&self) -> Resultat<Vec<Machine>> {
        self.json("/api/machines")
    }

    pub fn systeme(&self) -> Resultat<Systeme> {
        self.json("/api/systeme")
    }

    pub fn reglages(&self) -> Resultat<Vec<Reglage>> {
        self.json("/api/reglages")
    }

    /// Aperçu PNG d'une page (numérotée à partir de 1).
    pub fn apercu(&self, travail: &str, page: usize) -> Resultat<Vec<u8>> {
        let rep = self.verifier(self.requete("GET", &format!("/api/travaux/{travail}/pages/{page}/apercu.png")).call())?;
        let mut octets = vec![];
        rep.into_reader().take(256 << 20).read_to_end(&mut octets).map_err(|e| Erreur::locale("reseau", e.to_string()))?;
        Ok(octets)
    }

    /// Épreuve TIFF d'une page, écrite en flux dans `destination` (elle peut peser des Go).
    pub fn epreuve(&self, travail: &str, page: usize, destination: &std::path::Path) -> Resultat<()> {
        let chemin = format!("/api/travaux/{travail}/pages/{page}/epreuve.tif");
        let rep = self.verifier(self.avec_cookie(self.agent_long.get(&self.url(&chemin))).call())?;
        let ecrire = || -> std::io::Result<()> {
            let mut f = std::fs::File::create(destination)?;
            std::io::copy(&mut rep.into_reader(), &mut f)?;
            f.sync_all()
        };
        ecrire().map_err(|e| {
            let _ = std::fs::remove_file(destination);
            Erreur::locale("ecriture_impossible", e.to_string())
        })
    }

    // --- Actions ---

    pub fn imprimer(&self, travail: &str, page: usize, machine: &str) -> Resultat<()> {
        self.envoyer_json("POST", "/api/file", serde_json::json!({ "travail": travail, "page": page, "machine": machine }))
    }

    pub fn relancer(&self, travail: &str) -> Resultat<()> {
        self.sans_corps("POST", &format!("/api/travaux/{travail}/relancer"))
    }

    pub fn supprimer(&self, travail: &str) -> Resultat<()> {
        self.sans_corps("DELETE", &format!("/api/travaux/{travail}"))
    }

    /// `suspendre`, `reprendre`, `annuler`, `en_tete`, `monter`, `descendre`, `refaire`.
    pub fn agir_sur_sortie(&self, sortie: &str, action: &str) -> Resultat<()> {
        self.sans_corps("POST", &format!("/api/file/{sortie}/{action}"))
    }
}
