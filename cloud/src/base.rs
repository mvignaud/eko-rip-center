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
    bloque_jusqua INTEGER NOT NULL DEFAULT 0
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
";

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
    Ok(c)
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
}

const COLONNES: &str = "id, email, nom, super_admin, suspendu, cree_le, hachage IS NOT NULL, google IS NOT NULL, facebook IS NOT NULL";

fn compte(r: &rusqlite::Row) -> rusqlite::Result<Compte> {
    Ok(Compte { id: r.get(0)?, email: r.get(1)?, nom: r.get(2)?, super_admin: r.get(3)?, suspendu: r.get(4)?, cree_le: r.get::<_, i64>(5)? as u64, a_mot_de_passe: r.get(6)?, google: r.get(7)?, facebook: r.get(8)? })
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

/// E-mail plausible (la vérification par lien viendra avec l'envoi d'e-mails).
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
    q.query_map([], |r| Ok((compte(r)?, r.get(9)?)))?.collect()
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
}
