//! Préférences retenues entre deux lancements : un petit fichier par valeur dans
//! `~/.config/e-ko-rip-center/` (adresse du serveur, dernier réglage d'envoi).

use std::path::PathBuf;

pub const PORT_PAR_DEFAUT: u16 = 8640;

pub fn dossier() -> PathBuf {
    gtk::glib::user_config_dir().join("e-ko-rip-center")
}

pub fn lire(nom: &str) -> Option<String> {
    let v = std::fs::read_to_string(dossier().join(nom)).ok()?;
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

pub fn ecrire(nom: &str, valeur: &str) {
    let d = dossier();
    if std::fs::create_dir_all(&d).is_ok() {
        let _ = std::fs::write(d.join(nom), format!("{valeur}\n"));
    }
}

pub fn supprimer(nom: &str) {
    let _ = std::fs::remove_file(dossier().join(nom));
}

/// Adresse saisie (`hôte`, `hôte:8640`, `http://hôte:8640/`) ramenée à `http://hôte:port`.
/// Sans schéma ni port, le port 8640 du serveur est pris.
pub fn normaliser_adresse(saisie: &str) -> Option<String> {
    let s = saisie.trim().trim_end_matches('/');
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    let (schema, reste) = match s.split_once("://") {
        Some((sch, r)) if sch == "http" || sch == "https" => (sch, r),
        Some(_) => return None,
        None => ("http", s),
    };
    if reste.is_empty() || reste.contains('/') {
        return None;
    }
    // Un port est présent après le dernier « : » (hors adresse IPv6 entre crochets).
    let a_un_port = reste.rsplit_once(':').is_some_and(|(h, p)| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && !h.ends_with(':'));
    if a_un_port || schema == "https" {
        Some(format!("{schema}://{reste}"))
    } else {
        Some(format!("{schema}://{reste}:{PORT_PAR_DEFAUT}"))
    }
}

#[cfg(test)]
mod tests {
    use super::normaliser_adresse;

    #[test]
    fn adresses() {
        assert_eq!(normaliser_adresse("192.168.10.109").as_deref(), Some("http://192.168.10.109:8640"));
        assert_eq!(normaliser_adresse(" rip.local:9000/ ").as_deref(), Some("http://rip.local:9000"));
        assert_eq!(normaliser_adresse("http://rip:8640").as_deref(), Some("http://rip:8640"));
        assert_eq!(normaliser_adresse("https://rip.e-ko.fr").as_deref(), Some("https://rip.e-ko.fr"));
        assert_eq!(normaliser_adresse("https://k3x9ab.platform.e-ko.rip").as_deref(), Some("https://k3x9ab.platform.e-ko.rip"));
        assert_eq!(normaliser_adresse("https://platform.e-ko.rip/s/k3x9ab"), None);
        assert_eq!(normaliser_adresse(""), None);
        assert_eq!(normaliser_adresse("ftp://x"), None);
        assert_eq!(normaliser_adresse("a b"), None);
    }
}
