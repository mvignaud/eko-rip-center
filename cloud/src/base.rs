//! Base du portail (`portail.sqlite`, droits 0600) : comptes, sessions, droits sur les sites,
//! réglages (clés Google / Facebook). Les sites eux-mêmes et leurs clés restent dans
//! `sites.json`, lu par le relais à chaque ouverture de tunnel.

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS comptes (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    nom TEXT NOT NULL,
    hachage TEXT,
    google TEXT UNIQUE,
    facebook TEXT UNIQUE,
    super_admin INTEGER NOT NULL DEFAULT 0,
    suspendu INTEGER NOT NULL DEFAULT 0,
    cree_le INTEGER NOT NULL,
    echecs INTEGER NOT NULL DEFAULT 0,
    bloque_jusqua INTEGER NOT NULL DEFAULT 0,
    email_verifie INTEGER NOT NULL DEFAULT 0,
    alertes INTEGER NOT NULL DEFAULT 1,
    resume INTEGER NOT NULL DEFAULT 1,
    notifications_proposees INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS sessions (
    jeton TEXT PRIMARY KEY,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    csrf TEXT NOT NULL,
    expire INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS droits (
    site TEXT NOT NULL,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    PRIMARY KEY (site, compte)
);
CREATE TABLE IF NOT EXISTS reglages (cle TEXT PRIMARY KEY, valeur TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS jetons_applis (
    empreinte TEXT PRIMARY KEY,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    appareil TEXT NOT NULL,
    cree_le INTEGER NOT NULL,
    expire INTEGER NOT NULL,
    vu_le INTEGER NOT NULL,
    jeton_expo TEXT,
    plateforme_expo TEXT
);
-- Jetons à usage unique envoyés par e-mail (mot de passe oublié, confirmation de l'adresse) :
-- seule l'empreinte SHA-256 est gardée.
CREATE TABLE IF NOT EXISTS jetons_courriel (
    empreinte TEXT PRIMARY KEY,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    objet TEXT NOT NULL,
    cree_le INTEGER NOT NULL,
    expire INTEGER NOT NULL
);
-- Journal par site : `coupure` (debut, fin vide tant que le tunnel est fermé) et `alerte`
-- (debut = envoi, detail = `envoyee` ou `ignoree`). Purgé au-delà de 90 jours.
CREATE TABLE IF NOT EXISTS evenements_sites (
    id INTEGER PRIMARY KEY,
    site TEXT NOT NULL,
    type TEXT NOT NULL,
    debut INTEGER NOT NULL,
    fin INTEGER,
    detail TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS evenements_sites_site ON evenements_sites (site, debut);
-- Journal des notifications par compte (interrogé par les apps de bureau), gardé 30 jours.
-- AUTOINCREMENT : un identifiant n'est jamais réutilisé, même après la purge.
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    type TEXT NOT NULL,
    site TEXT,
    titre TEXT NOT NULL,
    corps TEXT NOT NULL,
    lien TEXT NOT NULL,
    cree_le INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS notifications_compte ON notifications (compte, id);
-- Navigateurs inscrits au Web Push (portail).
CREATE TABLE IF NOT EXISTS abonnements_web (
    id INTEGER PRIMARY KEY,
    compte INTEGER NOT NULL REFERENCES comptes(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    appareil TEXT NOT NULL DEFAULT '',
    cree_le INTEGER NOT NULL
);
";

/// Durée d'un jeton d'app (clients Mac, Linux, Windows, mobile).
pub const DUREE_JETON_APPLI: u64 = 180 * 24 * 3600;
/// Préfixe des jetons d'app : les reconnaître sans base.
pub const PREFIXE_JETON_APPLI: &str = "ekoa_";

/// Durée d'une session du portail.
pub const DUREE_SESSION: u64 = 30 * 24 * 3600;
const ECHECS_MAX: i64 = 5;
const BLOCAGE: u64 = 5 * 60;

pub fn ouvrir(donnees: &Path) -> rusqlite::Result<Connection> {
    let _ = std::fs::create_dir_all(donnees);
    let chemin = donnees.join("portail.sqlite");
    let neuf = !chemin.exists();
    let c = Connection::open(&chemin)?;
    #[cfg(unix)]
    if neuf {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&chemin, std::fs::Permissions::from_mode(0o600));
    }
    let _ = neuf;
    c.busy_timeout(std::time::Duration::from_secs(5))?;
    c.execute_batch(SCHEMA)?;
    migrer(&c)?;
    Ok(c)
}

/// Migration douce d'une base créée avant les e-mails : colonnes ajoutées si absentes.
/// Les comptes existants comptent pour vérifiés (ils ne peuvent pas être bloqués après coup).
fn migrer(c: &Connection) -> rusqlite::Result<()> {
    let colonnes: Vec<String> = c.prepare("PRAGMA table_info(comptes)")?.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<_>>()?;
    let a = |nom: &str| colonnes.iter().any(|x| x == nom);
    if !a("email_verifie") {
        c.execute_batch("ALTER TABLE comptes ADD COLUMN email_verifie INTEGER NOT NULL DEFAULT 0; UPDATE comptes SET email_verifie = 1;")?;
    }
    if !a("alertes") {
        c.execute_batch("ALTER TABLE comptes ADD COLUMN alertes INTEGER NOT NULL DEFAULT 1;")?;
    }
    if !a("resume") {
        c.execute_batch("ALTER TABLE comptes ADD COLUMN resume INTEGER NOT NULL DEFAULT 1;")?;
    }
    if !a("notifications_proposees") {
        c.execute_batch("ALTER TABLE comptes ADD COLUMN notifications_proposees INTEGER NOT NULL DEFAULT 0;")?;
    }
    let jetons: Vec<String> = c.prepare("PRAGMA table_info(jetons_applis)")?.query_map([], |r| r.get::<_, String>(1))?.collect::<rusqlite::Result<_>>()?;
    if !jetons.iter().any(|x| x == "jeton_expo") {
        c.execute_batch("ALTER TABLE jetons_applis ADD COLUMN jeton_expo TEXT; ALTER TABLE jetons_applis ADD COLUMN plateforme_expo TEXT;")?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct Compte {
    pub id: i64,
    pub email: String,
    pub nom: String,
    pub super_admin: bool,
    pub suspendu: bool,
    pub cree_le: u64,
    pub a_mot_de_passe: bool,
    pub google: bool,
    pub facebook: bool,
    /// Adresse confirmée (lien reçu par e-mail, ou e-mail vérifié par Google).
    pub email_verifie: bool,
    /// Reçoit les alertes « site hors ligne ».
    pub alertes: bool,
    /// Reçoit le résumé hebdomadaire.
    pub resume: bool,
    /// La proposition « Recevoir les alertes sur cet ordinateur ? » a été vue (acceptée ou reportée).
    pub notifications_proposees: bool,
}

const COLONNES: &str = "id, email, nom, super_admin, suspendu, cree_le, hachage IS NOT NULL, google IS NOT NULL, facebook IS NOT NULL, email_verifie, alertes, resume, notifications_proposees";

fn compte(r: &rusqlite::Row) -> rusqlite::Result<Compte> {
    Ok(Compte { id: r.get(0)?, email: r.get(1)?, nom: r.get(2)?, super_admin: r.get(3)?, suspendu: r.get(4)?, cree_le: r.get::<_, i64>(5)? as u64, a_mot_de_passe: r.get(6)?, google: r.get(7)?, facebook: r.get(8)?, email_verifie: r.get(9)?, alertes: r.get(10)?, resume: r.get(11)?, notifications_proposees: r.get(12)? })
}

pub fn hacher(mot_de_passe: &str) -> String {
    use argon2::PasswordHasher;
    let mut sel = [0u8; 16];
    getrandom::fill(&mut sel).expect("générateur aléatoire");
    argon2::Argon2::default().hash_password_with_salt(mot_de_passe.as_bytes(), &sel).expect("Argon2").to_string()
}

fn verifier(mot_de_passe: &str, hachage: &str) -> bool {
    use argon2::PasswordVerifier;
    argon2::Argon2::default().verify_password(mot_de_passe.as_bytes(), hachage).is_ok()
}

/// E-mail plausible (la propriété de l'adresse se prouve par le lien de confirmation).
pub fn email_valide(e: &str) -> bool {
    let e = e.trim();
    (3..=254).contains(&e.len()) && !e.chars().any(|c| c.is_whitespace() || c.is_control()) && e.split_once('@').is_some_and(|(a, d)| !a.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.'))
}

pub fn nom_valide(n: &str) -> bool {
    let n = n.trim();
    (1..=80).contains(&n.chars().count()) && !n.chars().any(char::is_control)
}

/// Les deux mots de passe de première connexion (inscription, changement) : 10 caractères au moins.
pub fn mot_de_passe_valide(m: &str) -> bool {
    (10..=200).contains(&m.chars().count())
}

pub fn creer_compte(c: &Connection, email: &str, nom: &str, hachage: Option<&str>) -> rusqlite::Result<Option<i64>> {
    let n = c.execute("INSERT OR IGNORE INTO comptes (email, nom, hachage, cree_le) VALUES (?1, ?2, ?3, ?4)", params![email.trim(), nom.trim(), hachage, crate::maintenant() as i64])?;
    Ok((n == 1).then(|| c.last_insert_rowid()))
}

pub fn compte_par_id(c: &Connection, id: i64) -> rusqlite::Result<Option<Compte>> {
    c.query_row(&format!("SELECT {COLONNES} FROM comptes WHERE id = ?1"), [id], compte).optional()
}

pub fn compte_par_email(c: &Connection, email: &str) -> rusqlite::Result<Option<Compte>> {
    c.query_row(&format!("SELECT {COLONNES} FROM comptes WHERE email = ?1"), [email.trim()], compte).optional()
}

/// `fournisseur` : `google` ou `facebook` (colonne).
pub fn compte_par_fournisseur(c: &Connection, fournisseur: &str, ident: &str) -> rusqlite::Result<Option<Compte>> {
    let col = colonne_fournisseur(fournisseur);
    c.query_row(&format!("SELECT {COLONNES} FROM comptes WHERE {col} = ?1"), [ident], compte).optional()
}

fn colonne_fournisseur(f: &str) -> &'static str {
    match f {
        "google" => "google",
        "facebook" => "facebook",
        _ => panic!("fournisseur inconnu : {f}"),
    }
}

pub fn lier_fournisseur(c: &Connection, compte: i64, fournisseur: &str, ident: &str) -> rusqlite::Result<bool> {
    let col = colonne_fournisseur(fournisseur);
    Ok(c.execute(&format!("UPDATE OR IGNORE comptes SET {col} = ?1 WHERE id = ?2"), params![ident, compte])? == 1)
}

/// Un fournisseur qui a VÉRIFIÉ l'e-mail (Google) prend un compte existant à cet e-mail : celui
/// qui prouve l'adresse en est le titulaire. Tout autre accès est retiré (mot de passe,
/// autre fournisseur, sessions ouvertes) : quelqu'un qui aurait inscrit cet e-mail avant son
/// titulaire, sans pouvoir le prouver, perd la main. Le titulaire se connecte ensuite par ce
/// fournisseur.
pub fn reprendre_par_fournisseur(c: &Connection, compte: i64, fournisseur: &str, ident: &str) -> rusqlite::Result<bool> {
    let col = colonne_fournisseur(fournisseur);
    let t = c.unchecked_transaction()?;
    t.execute("UPDATE comptes SET hachage = NULL, google = NULL, facebook = NULL, echecs = 0, bloque_jusqua = 0, email_verifie = 1 WHERE id = ?1", [compte])?;
    t.execute("DELETE FROM sessions WHERE compte = ?1", [compte])?;
    t.execute("DELETE FROM jetons_courriel WHERE compte = ?1", [compte])?;
    let lie = t.execute(&format!("UPDATE OR IGNORE comptes SET {col} = ?1 WHERE id = ?2"), params![ident, compte])? == 1;
    if lie {
        t.commit()?;
    }
    Ok(lie)
}

pub enum Connexion {
    Ok(i64),
    Refusee,
    Bloquee,
    Suspendu,
}

/// Vérifie e-mail + mot de passe ; 5 échecs bloquent le compte 5 minutes.
pub fn connecter(c: &Connection, email: &str, mot_de_passe: &str) -> rusqlite::Result<Connexion> {
    let ligne: Option<(i64, Option<String>, i64, i64, bool)> = c.query_row("SELECT id, hachage, echecs, bloque_jusqua, suspendu FROM comptes WHERE email = ?1", [email.trim()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional()?;
    let maintenant = crate::maintenant() as i64;
    let Some((id, hachage, echecs, bloque, suspendu)) = ligne else {
        // Même coût qu'un vrai essai : on ne devine pas les comptes au temps de réponse.
        let _ = verifier(mot_de_passe, "$argon2id$v=19$m=19456,t=2,p=1$c2VsLWZhY3RpY2U$YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo");
        return Ok(Connexion::Refusee);
    };
    if bloque > maintenant {
        return Ok(Connexion::Bloquee);
    }
    if hachage.as_deref().is_some_and(|h| verifier(mot_de_passe, h)) {
        if suspendu {
            return Ok(Connexion::Suspendu);
        }
        c.execute("UPDATE comptes SET echecs = 0 WHERE id = ?1", [id])?;
        return Ok(Connexion::Ok(id));
    }
    let echecs = echecs + 1;
    let bloque = if echecs >= ECHECS_MAX { maintenant + BLOCAGE as i64 } else { 0 };
    c.execute("UPDATE comptes SET echecs = ?1, bloque_jusqua = ?2 WHERE id = ?3", params![if bloque > 0 { 0 } else { echecs }, bloque, id])?;
    Ok(Connexion::Refusee)
}

/// Ouvre une session : (jeton du cookie, jeton anti-falsification). Seule l'empreinte du
/// jeton est gardée.
pub fn ouvrir_session(c: &Connection, compte: i64) -> rusqlite::Result<(String, String)> {
    let jeton = crate::aleatoire_hex(32);
    let csrf = crate::aleatoire_hex(16);
    let maintenant = crate::maintenant() as i64;
    c.execute("DELETE FROM sessions WHERE expire < ?1", [maintenant])?;
    c.execute("INSERT INTO sessions (jeton, compte, csrf, expire) VALUES (?1, ?2, ?3, ?4)", params![crate::sha256_hex(&jeton), compte, csrf, maintenant + DUREE_SESSION as i64])?;
    Ok((jeton, csrf))
}

/// Session valide : (compte, jeton anti-falsification). Un compte suspendu n'en a plus.
pub fn session(c: &Connection, jeton: &str) -> rusqlite::Result<Option<(Compte, String)>> {
    let ligne: Option<(i64, String)> = c.query_row("SELECT compte, csrf FROM sessions WHERE jeton = ?1 AND expire >= ?2", params![crate::sha256_hex(jeton), crate::maintenant() as i64], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    let Some((id, csrf)) = ligne else { return Ok(None) };
    Ok(compte_par_id(c, id)?.filter(|cp| !cp.suspendu).map(|cp| (cp, csrf)))
}

pub fn fermer_session(c: &Connection, jeton: &str) -> rusqlite::Result<()> {
    c.execute("DELETE FROM sessions WHERE jeton = ?1", [crate::sha256_hex(jeton)])?;
    Ok(())
}

pub fn reglage(c: &Connection, cle: &str) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT valeur FROM reglages WHERE cle = ?1", [cle], |r| r.get(0)).optional().map(|v| v.filter(|s: &String| !s.is_empty()))
}

pub fn regler(c: &Connection, cle: &str, valeur: &str) -> rusqlite::Result<()> {
    c.execute("INSERT INTO reglages (cle, valeur) VALUES (?1, ?2) ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur", params![cle, valeur.trim()])?;
    Ok(())
}

pub fn lister_comptes(c: &Connection) -> rusqlite::Result<Vec<(Compte, i64)>> {
    let mut q = c.prepare(&format!("SELECT {COLONNES}, (SELECT COUNT(*) FROM droits d WHERE d.compte = comptes.id AND d.role = 'proprietaire') FROM comptes ORDER BY id"))?;
    q.query_map([], |r| Ok((compte(r)?, r.get(13)?)))?.collect()
}

pub fn suspendre(c: &Connection, compte: i64, suspendu: bool) -> rusqlite::Result<()> {
    c.execute("UPDATE comptes SET suspendu = ?1 WHERE id = ?2 AND super_admin = 0", params![suspendu, compte])?;
    if suspendu {
        c.execute("DELETE FROM sessions WHERE compte = ?1", [compte])?;
    }
    Ok(())
}

pub fn nommer_super_admin(c: &Connection, email: &str) -> rusqlite::Result<bool> {
    Ok(c.execute("UPDATE comptes SET super_admin = 1, suspendu = 0 WHERE email = ?1", [email.trim()])? == 1)
}

pub fn supprimer_compte(c: &Connection, compte: i64) -> rusqlite::Result<()> {
    c.execute("DELETE FROM comptes WHERE id = ?1", [compte])?;
    Ok(())
}

/// Rôles sur un site : `proprietaire`, `admin`, `operateur`, `lecture`.
pub fn donner_droit(c: &Connection, site: &str, compte: i64, role: &str) -> rusqlite::Result<()> {
    c.execute("INSERT INTO droits (site, compte, role) VALUES (?1, ?2, ?3) ON CONFLICT(site, compte) DO UPDATE SET role = excluded.role", params![site, compte, role])?;
    Ok(())
}

pub fn droit(c: &Connection, site: &str, compte: i64) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT role FROM droits WHERE site = ?1 AND compte = ?2", params![site, compte], |r| r.get(0)).optional()
}

/// Sites d'un compte : (site, rôle).
pub fn sites_du_compte(c: &Connection, compte: i64) -> rusqlite::Result<Vec<(String, String)>> {
    let mut q = c.prepare("SELECT site, role FROM droits WHERE compte = ?1 ORDER BY site")?;
    q.query_map([compte], |r| Ok((r.get(0)?, r.get(1)?)))?.collect()
}

pub fn proprietaire(c: &Connection, site: &str) -> rusqlite::Result<Option<String>> {
    c.query_row("SELECT c.email FROM droits d JOIN comptes c ON c.id = d.compte WHERE d.site = ?1 AND d.role = 'proprietaire'", [site], |r| r.get(0)).optional()
}

pub fn nombre_de_sites(c: &Connection, compte: i64) -> rusqlite::Result<i64> {
    c.query_row("SELECT COUNT(*) FROM droits WHERE compte = ?1 AND role = 'proprietaire'", [compte], |r| r.get(0))
}

pub fn oublier_site(c: &Connection, site: &str) -> rusqlite::Result<()> {
    c.execute("DELETE FROM droits WHERE site = ?1", [site])?;
    c.execute("DELETE FROM evenements_sites WHERE site = ?1", [site])?;
    Ok(())
}

// ————— Jetons envoyés par e-mail —————

pub const OBJET_REINIT: &str = "reinit";
pub const OBJET_CONFIRMATION: &str = "confirmation";
/// Lien « mot de passe oublié » : 1 heure.
pub const DUREE_JETON_REINIT: u64 = 3600;
/// Lien de confirmation de l'adresse : 24 heures.
pub const DUREE_JETON_CONFIRMATION: u64 = 24 * 3600;

/// Crée un jeton à usage unique pour `compte` (un seul jeton vivant par compte et par objet) ;
/// seule son empreinte est gardée. Rend le jeton à mettre dans le lien de l'e-mail.
pub fn creer_jeton_courriel(c: &Connection, compte: i64, objet: &str, duree: u64) -> rusqlite::Result<String> {
    let mut alea = [0u8; 32];
    getrandom::fill(&mut alea).expect("générateur aléatoire");
    let jeton = crate::base64url(&alea);
    let t = crate::maintenant() as i64;
    c.execute("DELETE FROM jetons_courriel WHERE expire < ?1 OR (compte = ?2 AND objet = ?3)", params![t, compte, objet])?;
    c.execute("INSERT INTO jetons_courriel (empreinte, compte, objet, cree_le, expire) VALUES (?1, ?2, ?3, ?4, ?5)", params![crate::sha256_hex(&jeton), compte, objet, t, t + duree as i64])?;
    Ok(jeton)
}

/// Compte d'un jeton encore valable, sans le consommer (pour afficher le formulaire).
pub fn jeton_courriel_valide(c: &Connection, jeton: &str, objet: &str) -> rusqlite::Result<Option<i64>> {
    c.query_row("SELECT compte FROM jetons_courriel WHERE empreinte = ?1 AND objet = ?2 AND expire >= ?3", params![crate::sha256_hex(jeton), objet, crate::maintenant() as i64], |r| r.get(0)).optional()
}

/// Consomme un jeton (usage unique) : rend son compte s'il était valable. Dans la transaction `t`.
fn prendre_jeton_courriel(t: &Connection, jeton: &str, objet: &str) -> rusqlite::Result<Option<i64>> {
    let empreinte = crate::sha256_hex(jeton);
    let ligne: Option<(i64, i64)> = t.query_row("SELECT compte, expire FROM jetons_courriel WHERE empreinte = ?1 AND objet = ?2", params![empreinte, objet], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    let Some((compte, expire)) = ligne else { return Ok(None) };
    t.execute("DELETE FROM jetons_courriel WHERE empreinte = ?1", [empreinte])?;
    Ok((expire >= crate::maintenant() as i64).then_some(compte))
}

/// Nouveau mot de passe par le lien reçu : consomme le jeton, remplace le mot de passe, ferme
/// toutes les sessions du compte, révoque ses jetons d'apps et ses autres liens. Écrire le
/// mot de passe prouve aussi l'adresse. Compte suspendu ou jeton inutilisable : `None`.
pub fn reinitialiser_mot_de_passe(c: &Connection, jeton: &str, hachage: &str) -> rusqlite::Result<Option<i64>> {
    let t = c.unchecked_transaction()?;
    let Some(compte) = prendre_jeton_courriel(&t, jeton, OBJET_REINIT)? else {
        t.commit()?;
        return Ok(None);
    };
    if t.execute("UPDATE comptes SET hachage = ?1, echecs = 0, bloque_jusqua = 0, email_verifie = 1 WHERE id = ?2 AND suspendu = 0", params![hachage, compte])? != 1 {
        t.commit()?;
        return Ok(None);
    }
    t.execute("DELETE FROM sessions WHERE compte = ?1", [compte])?;
    t.execute("DELETE FROM jetons_courriel WHERE compte = ?1", [compte])?;
    revoquer_jetons_du_compte(&t, compte)?;
    t.commit()?;
    Ok(Some(compte))
}

/// Confirmation de l'adresse par le lien reçu (usage unique).
pub fn confirmer_email(c: &Connection, jeton: &str) -> rusqlite::Result<Option<i64>> {
    let t = c.unchecked_transaction()?;
    let compte = prendre_jeton_courriel(&t, jeton, OBJET_CONFIRMATION)?;
    if let Some(id) = compte {
        t.execute("UPDATE comptes SET email_verifie = 1 WHERE id = ?1", [id])?;
    }
    t.commit()?;
    Ok(compte)
}

/// Adresse prouvée par un fournisseur (Google) ou à la création.
pub fn marquer_email_verifie(c: &Connection, compte: i64) -> rusqlite::Result<()> {
    c.execute("UPDATE comptes SET email_verifie = 1 WHERE id = ?1", [compte])?;
    Ok(())
}

/// Cases « Recevoir les alertes » et « Recevoir le résumé hebdomadaire ».
pub fn regler_preferences(c: &Connection, compte: i64, alertes: bool, resume: bool) -> rusqlite::Result<()> {
    c.execute("UPDATE comptes SET alertes = ?1, resume = ?2 WHERE id = ?3", params![alertes, resume, compte])?;
    Ok(())
}

/// La proposition de notifications du navigateur a été présentée : on ne la refait pas.
pub fn marquer_notifications_proposees(c: &Connection, compte: i64) -> rusqlite::Result<()> {
    c.execute("UPDATE comptes SET notifications_proposees = 1 WHERE id = ?1", [compte])?;
    Ok(())
}

// ————— Jetons des apps (« Se connecter au cloud ») —————

/// Crée un jeton d'app pour `compte` ; seule son empreinte est gardée.
pub fn creer_jeton_appli(c: &Connection, compte: i64, appareil: &str) -> rusqlite::Result<String> {
    let jeton = format!("{PREFIXE_JETON_APPLI}{}", crate::aleatoire_hex(32));
    let t = crate::maintenant() as i64;
    let appareil: String = appareil.chars().filter(|c| !c.is_control()).take(80).collect();
    c.execute("DELETE FROM jetons_applis WHERE expire < ?1", [t])?;
    c.execute("INSERT INTO jetons_applis (empreinte, compte, appareil, cree_le, expire, vu_le) VALUES (?1, ?2, ?3, ?4, ?5, ?4)", params![crate::sha256_hex(&jeton), compte, appareil, t, t + DUREE_JETON_APPLI as i64])?;
    Ok(jeton)
}

/// Compte d'un jeton d'app valable (ni expiré, ni compte suspendu).
pub fn compte_du_jeton(c: &Connection, jeton: &str) -> rusqlite::Result<Option<Compte>> {
    if !jeton.starts_with(PREFIXE_JETON_APPLI) {
        return Ok(None);
    }
    let t = crate::maintenant() as i64;
    let empreinte = crate::sha256_hex(jeton);
    let Some(id) = c.query_row("SELECT compte FROM jetons_applis WHERE empreinte = ?1 AND expire >= ?2", params![empreinte, t], |r| r.get::<_, i64>(0)).optional()? else { return Ok(None) };
    // Date de dernière utilisation, à l'heure près (pas une écriture par requête).
    c.execute("UPDATE jetons_applis SET vu_le = ?2 WHERE empreinte = ?1 AND vu_le < ?2 - 3600", params![empreinte, t])?;
    Ok(compte_par_id(c, id)?.filter(|cp| !cp.suspendu))
}

pub fn revoquer_jeton_appli(c: &Connection, jeton: &str) -> rusqlite::Result<()> {
    c.execute("DELETE FROM jetons_applis WHERE empreinte = ?1", [crate::sha256_hex(jeton)])?;
    Ok(())
}

/// Apps connectées d'un compte : (appareil, créé le, vu le).
pub fn jetons_du_compte(c: &Connection, compte: i64) -> rusqlite::Result<Vec<(String, u64, u64)>> {
    let mut q = c.prepare("SELECT appareil, cree_le, vu_le FROM jetons_applis WHERE compte = ?1 AND expire >= ?2 ORDER BY vu_le DESC")?;
    q.query_map(params![compte, crate::maintenant() as i64], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64, r.get::<_, i64>(2)? as u64)))?.collect()
}

pub fn revoquer_jetons_du_compte(c: &Connection, compte: i64) -> rusqlite::Result<()> {
    c.execute("DELETE FROM jetons_applis WHERE compte = ?1", [compte])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comptes_sessions_blocage() {
        let d = std::env::temp_dir().join(format!("eko-cloud-base-{}", crate::aleatoire_hex(6)));
        let c = ouvrir(&d).unwrap();
        let id = creer_compte(&c, "A@exemple.fr", "Atelier", Some(&hacher("phrase-de-passe"))).unwrap().unwrap();
        assert!(creer_compte(&c, "a@EXEMPLE.fr", "Doublon", None).unwrap().is_none(), "e-mail unique sans tenir compte de la casse");
        assert!(matches!(connecter(&c, "a@exemple.fr", "phrase-de-passe").unwrap(), Connexion::Ok(i) if i == id));
        for _ in 0..5 {
            assert!(matches!(connecter(&c, "a@exemple.fr", "faux").unwrap(), Connexion::Refusee));
        }
        assert!(matches!(connecter(&c, "a@exemple.fr", "phrase-de-passe").unwrap(), Connexion::Bloquee));
        assert!(matches!(connecter(&c, "inconnu@exemple.fr", "x").unwrap(), Connexion::Refusee));
        let (jeton, csrf) = ouvrir_session(&c, id).unwrap();
        let (cp, c2) = session(&c, &jeton).unwrap().unwrap();
        assert_eq!((cp.id, c2), (id, csrf));
        assert!(session(&c, "autre").unwrap().is_none());
        suspendre(&c, id, true).unwrap();
        assert!(session(&c, &jeton).unwrap().is_none(), "suspendu : sessions fermées");
        assert!(email_valide("x@y.fr") && !email_valide("x@y") && !email_valide("a b@c.fr"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn reprise_par_email_verifie() {
        let d = std::env::temp_dir().join(format!("eko-cloud-base-{}", crate::aleatoire_hex(6)));
        let c = ouvrir(&d).unwrap();
        // Un tiers inscrit l'e-mail du titulaire avec son propre mot de passe.
        let id = creer_compte(&c, "titulaire@exemple.fr", "Tiers", Some(&hacher("phrase-du-tiers"))).unwrap().unwrap();
        assert!(lier_fournisseur(&c, id, "facebook", "fb-tiers").unwrap());
        let (jeton, _) = ouvrir_session(&c, id).unwrap();
        // Le titulaire arrive par Google (e-mail vérifié) : il reprend le compte.
        assert!(reprendre_par_fournisseur(&c, id, "google", "g-titulaire").unwrap());
        assert!(matches!(connecter(&c, "titulaire@exemple.fr", "phrase-du-tiers").unwrap(), Connexion::Refusee), "mot de passe du tiers retiré");
        assert!(session(&c, &jeton).unwrap().is_none(), "sessions du tiers fermées");
        assert!(compte_par_fournisseur(&c, "facebook", "fb-tiers").unwrap().is_none(), "autre fournisseur retiré");
        assert_eq!(compte_par_fournisseur(&c, "google", "g-titulaire").unwrap().unwrap().id, id);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn jetons_courriel_usage_unique_et_expiration() {
        let d = std::env::temp_dir().join(format!("eko-cloud-base-{}", crate::aleatoire_hex(6)));
        let c = ouvrir(&d).unwrap();
        let id = creer_compte(&c, "a@exemple.fr", "Atelier", Some(&hacher("ancien-mot-de-passe"))).unwrap().unwrap();
        let (session_ouverte, _) = ouvrir_session(&c, id).unwrap();
        let app = creer_jeton_appli(&c, id, "Mac").unwrap();
        let jeton = creer_jeton_courriel(&c, id, OBJET_REINIT, DUREE_JETON_REINIT).unwrap();
        assert_eq!(jeton.len(), 43, "32 octets en base64url");
        let stocke: String = c.query_row("SELECT empreinte FROM jetons_courriel", [], |r| r.get(0)).unwrap();
        assert_eq!(stocke, crate::sha256_hex(&jeton));
        assert_ne!(stocke, jeton, "seule l'empreinte est gardée");
        // Un jeton de confirmation ne sert pas à réinitialiser.
        let confirmation = creer_jeton_courriel(&c, id, OBJET_CONFIRMATION, DUREE_JETON_CONFIRMATION).unwrap();
        assert!(reinitialiser_mot_de_passe(&c, &confirmation, &hacher("nouveau-mot-de-passe")).unwrap().is_none());
        assert_eq!(jeton_courriel_valide(&c, &jeton, OBJET_REINIT).unwrap(), Some(id));
        // Un nouveau jeton remplace le précédent.
        let autre = creer_jeton_courriel(&c, id, OBJET_REINIT, DUREE_JETON_REINIT).unwrap();
        assert_eq!(jeton_courriel_valide(&c, &jeton, OBJET_REINIT).unwrap(), None);
        // Réinitialisation : mot de passe changé, sessions et apps fermées, usage unique.
        assert_eq!(reinitialiser_mot_de_passe(&c, &autre, &hacher("nouveau-mot-de-passe")).unwrap(), Some(id));
        assert!(reinitialiser_mot_de_passe(&c, &autre, &hacher("encore-un-autre-mdp")).unwrap().is_none(), "usage unique");
        assert!(session(&c, &session_ouverte).unwrap().is_none());
        assert!(compte_du_jeton(&c, &app).unwrap().is_none());
        assert!(matches!(connecter(&c, "a@exemple.fr", "nouveau-mot-de-passe").unwrap(), Connexion::Ok(i) if i == id));
        assert!(matches!(connecter(&c, "a@exemple.fr", "ancien-mot-de-passe").unwrap(), Connexion::Refusee));
        // Expiration.
        let vieux = creer_jeton_courriel(&c, id, OBJET_REINIT, DUREE_JETON_REINIT).unwrap();
        c.execute("UPDATE jetons_courriel SET expire = 1", []).unwrap();
        assert_eq!(jeton_courriel_valide(&c, &vieux, OBJET_REINIT).unwrap(), None);
        assert!(reinitialiser_mot_de_passe(&c, &vieux, &hacher("troisieme-mot-de-passe")).unwrap().is_none());
        // Compte suspendu : pas de réinitialisation.
        let j = creer_jeton_courriel(&c, id, OBJET_REINIT, DUREE_JETON_REINIT).unwrap();
        suspendre(&c, id, true).unwrap();
        assert!(reinitialiser_mot_de_passe(&c, &j, &hacher("quatrieme-mot-de-passe")).unwrap().is_none());
        // Confirmation de l'adresse.
        suspendre(&c, id, false).unwrap();
        let conf = creer_jeton_courriel(&c, id, OBJET_CONFIRMATION, 3600).unwrap();
        assert!(confirmer_email(&c, "n'importe quoi").unwrap().is_none());
        assert_eq!(confirmer_email(&c, &conf).unwrap(), Some(id));
        assert!(confirmer_email(&c, &conf).unwrap().is_none(), "usage unique");
        assert!(compte_par_id(&c, id).unwrap().unwrap().email_verifie);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn migration_douce_des_comptes_existants() {
        let d = std::env::temp_dir().join(format!("eko-cloud-base-{}", crate::aleatoire_hex(6)));
        std::fs::create_dir_all(&d).unwrap();
        {
            // Base d'avant les e-mails.
            let c = Connection::open(d.join("portail.sqlite")).unwrap();
            c.execute_batch("CREATE TABLE comptes (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE COLLATE NOCASE, nom TEXT NOT NULL, hachage TEXT, google TEXT UNIQUE, facebook TEXT UNIQUE, super_admin INTEGER NOT NULL DEFAULT 0, suspendu INTEGER NOT NULL DEFAULT 0, cree_le INTEGER NOT NULL, echecs INTEGER NOT NULL DEFAULT 0, bloque_jusqua INTEGER NOT NULL DEFAULT 0); INSERT INTO comptes (email, nom, cree_le) VALUES ('ancien@exemple.fr', 'Ancien', 1);").unwrap();
        }
        let c = ouvrir(&d).unwrap();
        let ancien = compte_par_email(&c, "ancien@exemple.fr").unwrap().unwrap();
        assert!(ancien.email_verifie && ancien.alertes && ancien.resume, "comptes existants : vérifiés, alertes et résumé activés");
        let nouveau = creer_compte(&c, "neuf@exemple.fr", "Neuf", None).unwrap().unwrap();
        assert!(!compte_par_id(&c, nouveau).unwrap().unwrap().email_verifie, "nouveau compte : à confirmer");
        drop(c);
        let c = ouvrir(&d).unwrap();
        assert!(!compte_par_email(&c, "neuf@exemple.fr").unwrap().unwrap().email_verifie, "la migration ne rejoue pas");
        let _ = std::fs::remove_dir_all(&d);
    }
}
