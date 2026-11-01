//! « Se connecter au cloud » : connexion à la plate-forme E-KO par OAuth 2 (code d'autorisation
//! avec PKCE, RFC 6749, 7636 et 8252). Le navigateur par défaut fait la saisie du mot de passe ;
//! il revient sur une écoute locale (`http://127.0.0.1:<port>/retour`). Tout est bloquant :
//! à appeler hors du fil GTK. Le jeton n'est jamais écrit dans les journaux ni dans une erreur.

use crate::api::{Erreur, Resultat, VERSION};
use crate::config;
use crate::notifications_cloud::ReponseNotifications;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const PLATEFORME_PAR_DEFAUT: &str = "https://platform.e-ko.rip";
const CLIENT_ID: &str = "eko-rip-center";
/// Attente maximale du retour du navigateur.
pub const DELAI_RETOUR: Duration = Duration::from_secs(300);
const FICHIER_JETON: &str = "cloud.json";

/// Plate-forme à utiliser : `EKO_RIP_CLOUD_URL` si elle est définie, sinon la plate-forme E-KO.
pub fn plateforme() -> String {
    plateforme_depuis(std::env::var("EKO_RIP_CLOUD_URL").ok().as_deref())
}

fn plateforme_depuis(surcharge: Option<&str>) -> String {
    let v = surcharge.map(str::trim).filter(|v| !v.is_empty()).unwrap_or(PLATEFORME_PAR_DEFAUT);
    v.trim_end_matches('/').to_string()
}

// --- Réponses de la plate-forme ---

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Compte {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub nom: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Site {
    pub id: String,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub adresse: String,
    #[serde(default)]
    pub en_ligne: bool,
    #[serde(default)]
    pub role: String,
}

impl Site {
    /// Rôle sur le site, en français.
    pub fn role_libelle(&self) -> &'static str {
        match self.role.as_str() {
            "proprietaire" => "Propriétaire",
            "admin" | "administrateur" => "Administrateur",
            "operateur" => "Opérateur",
            "lecture" => "Lecture seule",
            _ => "Accès",
        }
    }
}

/// `GET /api/moi`.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Moi {
    #[serde(default)]
    pub compte: Compte,
    #[serde(default)]
    pub sites: Vec<Site>,
}

// --- PKCE ---

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Base64 « URL » (RFC 4648 §5) sans remplissage.
pub fn base64url(octets: &[u8]) -> String {
    let mut s = String::with_capacity(octets.len().div_ceil(3) * 4);
    for bloc in octets.chunks(3) {
        let n = bloc.iter().enumerate().fold(0u32, |a, (i, b)| a | u32::from(*b) << (16 - 8 * i));
        for i in 0..=bloc.len() {
            s.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    s
}

/// `n` octets aléatoires tirés du système.
fn aleatoire(n: usize) -> std::io::Result<Vec<u8>> {
    let mut v = vec![0u8; n];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut v)?;
    Ok(v)
}

/// `code_challenge` = base64url(SHA-256(verifier)).
pub fn defi(verifier: &str) -> String {
    let mut somme = gtk::glib::Checksum::new(gtk::glib::ChecksumType::Sha256).expect("SHA-256 disponible");
    somme.update(verifier.as_bytes());
    let hex = somme.string().unwrap_or_default();
    let octets: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
    base64url(&octets)
}

// --- Adresses ---

/// Encodage pourcent de tout ce qui n'est pas un caractère non réservé (RFC 3986).
pub fn encoder(texte: &str) -> String {
    let mut s = String::with_capacity(texte.len());
    for b in texte.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// Décodage pourcent d'une valeur de requête (`+` = espace).
pub fn decoder(texte: &str) -> String {
    let o = texte.as_bytes();
    let mut v = Vec::with_capacity(o.len());
    let mut i = 0;
    while i < o.len() {
        match o[i] {
            b'+' => v.push(b' '),
            b'%' if  o.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) => {
                v.push(u8::from_str_radix(&texte[i + 1..i + 3], 16).unwrap_or(b'?'));
                i += 2;
            }
            b => v.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&v).into_owned()
}

/// Adresse à ouvrir dans le navigateur pour autoriser l'app.
pub fn url_autorisation(plateforme: &str, redirect_uri: &str, challenge: &str, state: &str, appareil: &str) -> String {
    format!(
        "{}/oauth/autoriser?client_id={CLIENT_ID}&response_type=code&redirect_uri={}&code_challenge={}&code_challenge_method=S256&state={}&appareil={}",
        plateforme.trim_end_matches('/'),
        encoder(redirect_uri),
        encoder(challenge),
        encoder(state),
        encoder(appareil)
    )
}

// --- Retour du navigateur ---

/// Ce que dit la requête reçue sur l'écoute locale.
#[derive(Debug, PartialEq, Eq)]
pub enum Retour {
    /// Code d'autorisation, avec le bon `state`.
    Code(String),
    /// Refus de l'utilisateur (ou erreur de la plate-forme), avec le bon `state`.
    Refus(String),
    /// Autre chose (mauvais chemin, `state` faux, paramètres absents) : ignorée.
    Invalide,
}

/// Lit la première ligne d'une requête HTTP (`GET /retour?code=…&state=… HTTP/1.1`).
pub fn lire_retour(requete: &str, state_attendu: &str) -> Retour {
    let ligne = requete.lines().next().unwrap_or_default();
    let mut mots = ligne.split_whitespace();
    let (Some("GET"), Some(cible)) = (mots.next(), mots.next()) else { return Retour::Invalide };
    let (chemin, requete) = cible.split_once('?').unwrap_or((cible, ""));
    if chemin != "/retour" {
        return Retour::Invalide;
    }
    let (mut code, mut state, mut erreur) = (None, None, None);
    for paire in requete.split('&') {
        let (k, v) = paire.split_once('=').unwrap_or((paire, ""));
        match k {
            "code" => code = Some(decoder(v)),
            "state" => state = Some(decoder(v)),
            "error" => erreur = Some(decoder(v)),
            _ => {}
        }
    }
    if state.as_deref() != Some(state_attendu) || state_attendu.is_empty() {
        return Retour::Invalide;
    }
    match (code, erreur) {
        (_, Some(e)) => Retour::Refus(if e.is_empty() { "access_denied".into() } else { e }),
        (Some(c), None) if !c.is_empty() => Retour::Code(c),
        _ => Retour::Invalide,
    }
}

fn page(titre: &str, message: &str) -> String {
    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{titre}</title>\
<style>body{{font-family:sans-serif;display:grid;place-items:center;min-height:100vh;margin:0;background:#f6f5f4;color:#1e1e1e}}main{{text-align:center;padding:2rem}}</style></head>\
<body><main><h1>{titre}</h1><p>{message}</p></main></body></html>"
    )
}

fn repondre(flux: &mut TcpStream, statut: &str, titre: &str, message: &str) {
    let corps = page(titre, message);
    let _ = write!(
        flux,
        "HTTP/1.1 {statut}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{corps}",
        corps.len()
    );
    let _ = flux.flush();
    let _ = flux.shutdown(Shutdown::Both);
}

/// Écoute locale ouverte sur un port libre de la boucle locale.
pub fn ecouter() -> std::io::Result<(TcpListener, u16)> {
    let l = TcpListener::bind(("127.0.0.1", 0))?;
    let port = l.local_addr()?.port();
    Ok((l, port))
}

/// Adresse de retour pour un port d'écoute.
pub fn redirect_uri(port: u16) -> String {
    format!("http://127.0.0.1:{port}/retour")
}

/// Attend le retour du navigateur : rend le code d'autorisation. Seule une requête
/// `GET /retour` valide (bon `state`) termine l'attente ; le reste reçoit une erreur et est
/// ignoré. L'écoute se ferme à la sortie (délai dépassé, annulation, refus ou succès).
pub fn attendre_retour(ecoute: TcpListener, state: &str, delai: Duration, annule: &AtomicBool) -> Resultat<String> {
    ecoute.set_nonblocking(true).map_err(|e| Erreur::locale("reseau", e.to_string()))?;
    let fin = Instant::now() + delai;
    loop {
        if annule.load(Ordering::Relaxed) {
            return Err(Erreur::locale("cloud_annule", None));
        }
        if Instant::now() >= fin {
            return Err(Erreur::locale("cloud_delai", None));
        }
        let mut flux = match ecoute.accept() {
            Ok((f, _)) => f,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => return Err(Erreur::locale("reseau", e.to_string())),
        };
        let _ = flux.set_nonblocking(false);
        let _ = flux.set_read_timeout(Some(Duration::from_secs(2)));
        let mut tampon = [0u8; 8192];
        let mut n = 0;
        // Au plus 2 s par connexion en tout, pour que l'annulation et le délai restent tenus
        // même face à un programme local qui envoie octet par octet.
        let limite = Instant::now() + Duration::from_secs(2);
        while n < tampon.len() && Instant::now() < limite {
            match flux.read(&mut tampon[n..]) {
                Ok(0) | Err(_) => break,
                Ok(k) => n += k,
            }
            if tampon[..n].contains(&b'\n') {
                break;
            }
        }
        match lire_retour(&String::from_utf8_lossy(&tampon[..n]), state) {
            Retour::Code(c) => {
                repondre(&mut flux, "200 OK", "Connexion réussie", "Vous pouvez fermer cet onglet et revenir à E-KO RIP Center.");
                return Ok(c);
            }
            Retour::Refus(e) => {
                repondre(&mut flux, "200 OK", "Connexion refusée", "L'accès n'a pas été autorisé. Vous pouvez fermer cet onglet.");
                return Err(Erreur::locale(&e, None));
            }
            Retour::Invalide => repondre(&mut flux, "400 Bad Request", "Requête inattendue", "Cette adresse ne sert qu'au retour de la connexion E-KO."),
        }
    }
}

// --- Appels à la plate-forme ---

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .user_agent(&format!("EKO-RIP-Client/{VERSION} (Linux)"))
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

/// Erreur d'un appel : `{ "error" }` (OAuth) ou `{ "code" }` (API) ; jamais de jeton dedans.
fn verifier(r: Result<ureq::Response, ureq::Error>) -> Resultat<ureq::Response> {
    match r {
        Ok(rep) => Ok(rep),
        Err(ureq::Error::Status(statut, rep)) => {
            let corps: serde_json::Value = rep.into_json().unwrap_or_default();
            let code = corps["error"].as_str().or_else(|| corps["code"].as_str()).map_or_else(|| format!("http_{statut}"), str::to_string);
            Err(Erreur { statut, code, detail: corps["detail"].as_str().or_else(|| corps["error_description"].as_str()).map(str::to_string) })
        }
        Err(ureq::Error::Transport(t)) => Err(Erreur::locale("reseau", t.to_string())),
    }
}

/// Échange le code d'autorisation contre un jeton d'accès.
pub fn echanger_code(plateforme: &str, code: &str, redirect_uri: &str, verifier_pkce: &str) -> Resultat<String> {
    let url = format!("{}/oauth/jeton", plateforme.trim_end_matches('/'));
    let champs = [("grant_type", "authorization_code"), ("client_id", CLIENT_ID), ("code", code), ("redirect_uri", redirect_uri), ("code_verifier", verifier_pkce)];
    let rep = verifier(agent().post(&url).send_form(&champs))?;
    let v: serde_json::Value = rep.into_json().map_err(|e| Erreur::locale("reponse_illisible", e.to_string()))?;
    v["access_token"].as_str().filter(|j| !j.is_empty()).map(str::to_string).ok_or_else(|| Erreur::locale("reponse_illisible", None))
}

/// `GET /api/moi` : le compte et ses sites. 401 (`session_perdue`) : jeton révoqué ou expiré.
pub fn moi(plateforme: &str, jeton: &str) -> Resultat<Moi> {
    let url = format!("{}/api/moi", plateforme.trim_end_matches('/'));
    let rep = verifier(agent().get(&url).set("Authorization", &format!("Bearer {jeton}")).call())?;
    rep.into_json().map_err(|e| Erreur::locale("reponse_illisible", e.to_string()))
}

/// `GET /api/moi/notifications` : sans `apres`, rend seulement `dernier` ; avec, au plus 50
/// notifications plus récentes. 401 (`session_perdue`) : jeton invalide.
pub fn notifications(plateforme: &str, jeton: &str, apres: Option<u64>) -> Resultat<ReponseNotifications> {
    let mut url = format!("{}/api/moi/notifications", plateforme.trim_end_matches('/'));
    if let Some(a) = apres {
        url.push_str(&format!("?apres={a}"));
    }
    let rep = verifier(agent().get(&url).set("Authorization", &format!("Bearer {jeton}")).call())?;
    rep.into_json().map_err(|e| Erreur::locale("reponse_illisible", e.to_string()))
}

/// `POST /api/moi/deconnexion` : révoque le jeton côté plate-forme.
pub fn deconnecter(plateforme: &str, jeton: &str) -> Resultat<()> {
    let url = format!("{}/api/moi/deconnexion", plateforme.trim_end_matches('/'));
    verifier(agent().post(&url).set("Authorization", &format!("Bearer {jeton}")).call()).map(|_| ())
}

/// Ce qu'il faut garder du côté de l'écoute avant d'ouvrir le navigateur.
pub struct Preparation {
    pub ecoute: TcpListener,
    pub verifier: String,
    pub state: String,
    pub redirect_uri: String,
    pub url: String,
}

/// Ouvre l'écoute locale, tire le vérificateur et le `state`, compose l'adresse d'autorisation.
pub fn preparer(plateforme: &str, appareil: &str) -> Resultat<Preparation> {
    let tirage = |n| aleatoire(n).map(|o| base64url(&o)).map_err(|e| Erreur::locale("cloud_aleatoire", e.to_string()));
    let (verifier, state) = (tirage(32)?, tirage(16)?);
    let (ecoute, port) = ecouter().map_err(|e| Erreur::locale("reseau", e.to_string()))?;
    let redirect_uri = redirect_uri(port);
    let url = url_autorisation(plateforme, &redirect_uri, &defi(&verifier), &state, appareil);
    Ok(Preparation { ecoute, verifier, state, redirect_uri, url })
}

/// Seconde moitié de la connexion, dans un fil à part : attend le navigateur, échange le code,
/// lit le compte. Rend le jeton et `GET /api/moi`.
pub fn terminer(plateforme: &str, p: Preparation, annule: &AtomicBool) -> Resultat<(String, Moi)> {
    let code = attendre_retour(p.ecoute, &p.state, DELAI_RETOUR, annule)?;
    let jeton = echanger_code(plateforme, &code, &p.redirect_uri, &p.verifier)?;
    let m = moi(plateforme, &jeton)?;
    Ok((jeton, m))
}

// --- Jeton gardé sur disque ---

#[derive(Serialize, Deserialize)]
struct Fichier {
    plateforme: String,
    jeton: String,
}

fn ecrire_dans(dossier: &Path, plateforme: &str, jeton: &str) -> std::io::Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    std::fs::create_dir_all(dossier)?;
    let chemin = dossier.join(FICHIER_JETON);
    let contenu = serde_json::to_vec(&Fichier { plateforme: plateforme.into(), jeton: jeton.into() })?;
    // Fichier temporaire puis renommage : jamais de jeton à moitié écrit.
    let temporaire = dossier.join(format!("{FICHIER_JETON}.tmp"));
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporaire)?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(&contenu)?;
    f.sync_all()?;
    std::fs::rename(&temporaire, &chemin)
}

fn lire_dans(dossier: &Path, plateforme: &str) -> Option<String> {
    let f: Fichier = serde_json::from_slice(&std::fs::read(dossier.join(FICHIER_JETON)).ok()?).ok()?;
    (f.plateforme == plateforme && !f.jeton.is_empty()).then_some(f.jeton)
}

/// Jeton retenu pour cette plate-forme (un jeton d'une autre plate-forme est ignoré).
pub fn jeton_retenu(plateforme: &str) -> Option<String> {
    lire_dans(&config::dossier(), plateforme)
}

pub fn retenir_jeton(plateforme: &str, jeton: &str) -> std::io::Result<()> {
    ecrire_dans(&config::dossier(), plateforme, jeton)
}

pub fn oublier_jeton() {
    let _ = std::fs::remove_file(config::dossier().join(FICHIER_JETON));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn base64url_rfc4648() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        assert_eq!(base64url(&[0xfb, 0xff, 0xfe]), "-__-");
        assert!(!base64url(&aleatoire(32).unwrap()).contains(['=', '+', '/']));
    }

    #[test]
    fn defi_pkce_rfc7636_annexe_b() {
        assert_eq!(defi("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn tirages_de_bonne_longueur() {
        assert_eq!(base64url(&aleatoire(32).unwrap()).len(), 43);
        assert_ne!(aleatoire(16).unwrap(), aleatoire(16).unwrap());
    }

    #[test]
    fn plateforme_surchargeable() {
        assert_eq!(plateforme_depuis(None), "https://platform.e-ko.rip");
        assert_eq!(plateforme_depuis(Some("  ")), "https://platform.e-ko.rip");
        assert_eq!(plateforme_depuis(Some("http://127.0.0.1:9000/")), "http://127.0.0.1:9000");
    }

    #[test]
    fn encodage_pourcent() {
        assert_eq!(encoder("http://127.0.0.1:1/retour"), "http%3A%2F%2F127.0.0.1%3A1%2Fretour");
        assert_eq!(encoder("Atelier é"), "Atelier%20%C3%A9");
        assert_eq!(decoder("Atelier%20%C3%A9+x%zz%4"), "Atelier é x%zz%4");
    }

    #[test]
    fn url_d_autorisation() {
        let u = url_autorisation("https://platform.e-ko.rip/", "http://127.0.0.1:4321/retour", "ch-_", "st", "PC de Léa");
        assert_eq!(
            u,
            "https://platform.e-ko.rip/oauth/autoriser?client_id=eko-rip-center&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A4321%2Fretour&code_challenge=ch-_&code_challenge_method=S256&state=st&appareil=PC%20de%20L%C3%A9a"
        );
    }

    #[test]
    fn requete_de_retour() {
        let ok = lire_retour("GET /retour?code=abc%2B1&state=etat HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", "etat");
        assert_eq!(ok, Retour::Code("abc+1".into()));
        assert_eq!(lire_retour("GET /retour?code=abc&state=autre HTTP/1.1", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("GET /retour?code=abc HTTP/1.1", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("GET /retour?state=etat HTTP/1.1", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("GET /ailleurs?code=abc&state=etat HTTP/1.1", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("POST /retour?code=abc&state=etat HTTP/1.1", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("", "etat"), Retour::Invalide);
        assert_eq!(lire_retour("GET /retour?error=access_denied&state=etat HTTP/1.1", "etat"), Retour::Refus("access_denied".into()));
        // Un refus avec un faux state n'arrête rien.
        assert_eq!(lire_retour("GET /retour?error=access_denied&state=faux HTTP/1.1", "etat"), Retour::Invalide);
    }

    fn requete(port: u16, ligne: &str) -> String {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(c, "{ligne}\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut rep = String::new();
        c.read_to_string(&mut rep).unwrap();
        rep
    }

    #[test]
    fn ecoute_locale_succes_apres_requetes_parasites() {
        let (ecoute, port) = ecouter().unwrap();
        let annule = Arc::new(AtomicBool::new(false));
        let a = annule.clone();
        let fil = std::thread::spawn(move || attendre_retour(ecoute, "etat", Duration::from_secs(10), &a));
        let parasite = requete(port, "GET /favicon.ico HTTP/1.1");
        assert!(parasite.starts_with("HTTP/1.1 400"));
        let faux = requete(port, "GET /retour?code=x&state=faux HTTP/1.1");
        assert!(faux.starts_with("HTTP/1.1 400"));
        let bon = requete(port, "GET /retour?code=le-code&state=etat HTTP/1.1");
        assert!(bon.starts_with("HTTP/1.1 200") && bon.contains("Connexion réussie"));
        assert_eq!(fil.join().unwrap().unwrap(), "le-code");
        // L'écoute est fermée.
        assert!(TcpStream::connect(("127.0.0.1", port)).is_err());
    }

    #[test]
    fn ecoute_locale_refus_delai_et_annulation() {
        let (ecoute, port) = ecouter().unwrap();
        let fil = std::thread::spawn(move || attendre_retour(ecoute, "etat", Duration::from_secs(10), &AtomicBool::new(false)));
        let rep = requete(port, "GET /retour?error=access_denied&state=etat HTTP/1.1");
        assert!(rep.contains("Connexion refusée"));
        assert_eq!(fil.join().unwrap().unwrap_err().code, "access_denied");

        let (ecoute, _) = ecouter().unwrap();
        let e = attendre_retour(ecoute, "etat", Duration::from_millis(150), &AtomicBool::new(false)).unwrap_err();
        assert_eq!(e.code, "cloud_delai");

        let (ecoute, _) = ecouter().unwrap();
        let e = attendre_retour(ecoute, "etat", Duration::from_secs(10), &AtomicBool::new(true)).unwrap_err();
        assert_eq!(e.code, "cloud_annule");
    }

    #[test]
    fn preparation_complete() {
        let p = preparer("https://platform.e-ko.rip", "PC").unwrap();
        assert_eq!(p.verifier.len(), 43);
        assert!(p.redirect_uri.starts_with("http://127.0.0.1:") && p.redirect_uri.ends_with("/retour"));
        assert!(p.url.contains(&format!("code_challenge={}", defi(&p.verifier))));
        assert!(p.url.contains(&format!("state={}", p.state)));
    }

    #[test]
    fn jeton_sur_disque_en_0600() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("eko-cloud-essai-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(lire_dans(&d, "https://p"), None);
        ecrire_dans(&d, "https://p", "ekoa_secret").unwrap();
        let mode = std::fs::metadata(d.join(FICHIER_JETON)).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(lire_dans(&d, "https://p").as_deref(), Some("ekoa_secret"));
        assert_eq!(lire_dans(&d, "https://autre"), None);
        // Réécriture d'un fichier existant : droits conservés.
        ecrire_dans(&d, "https://p", "ekoa_2").unwrap();
        assert_eq!(std::fs::metadata(d.join(FICHIER_JETON)).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(lire_dans(&d, "https://p").as_deref(), Some("ekoa_2"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn lecture_de_moi() {
        let m: Moi = serde_json::from_str(r#"{"compte":{"email":"a@b.fr","nom":"A"},"sites":[{"id":"k3x9ab","nom":"Atelier","adresse":"https://k3x9ab.platform.e-ko.rip","en_ligne":true,"role":"operateur"}]}"#).unwrap();
        assert_eq!(m.compte.email, "a@b.fr");
        assert_eq!(m.sites[0].role_libelle(), "Opérateur");
        assert!(crate::config::normaliser_adresse(&m.sites[0].adresse).is_some_and(|a| a == m.sites[0].adresse));
    }

    #[test]
    fn client_de_site_porte_le_jeton_sur_chaque_requete() {
        let (ecoute, port) = ecouter().unwrap();
        let fil = std::thread::spawn(move || {
            let mut vus = vec![];
            for _ in 0..2 {
                let (mut f, _) = ecoute.accept().unwrap();
                let mut t = [0u8; 4096];
                let n = f.read(&mut t).unwrap();
                vus.push(String::from_utf8_lossy(&t[..n]).to_lowercase());
                f.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}").unwrap();
            }
            vus
        });
        let c = crate::api::Client::cloud(&format!("http://127.0.0.1:{port}"), "ekoa_essai");
        assert!(c.est_cloud());
        let _ = c.session();
        // Chemin des envois et téléchargements (agent long).
        let _ = c.avec_cookie(c.agent_long.get(&c.url("/api/travaux/x/pages/1/epreuve.tif"))).call();
        for requete in fil.join().unwrap() {
            assert!(requete.contains("authorization: bearer ekoa_essai"), "{requete}");
        }
        assert!(!crate::api::Client::new("http://x").est_cloud());
    }
}
