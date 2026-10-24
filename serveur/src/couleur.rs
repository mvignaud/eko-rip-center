//! Gestion des couleurs : profils ICC du serveur et options Ghostscript d'un travail.
//!
//! Les profils sont rangés dans `<données>/profils/` (déposés depuis l'écran) ; aucun
//! profil tiers n'est livré avec le logiciel. Ghostscript convertit : profils d'entrée
//! par défaut (CMJN, RVB), profil de sortie (le support), intention de rendu.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Profil {
    pub fichier: String,
    /// Classe ICC : `prtr` (sortie), `scnr` (entrée), `mntr`, `spac`…
    pub classe: String,
    /// Espace de couleur des données : `CMYK`, `RGB`, `GRAY`…
    pub espace: String,
    pub description: String,
}

/// Réglages de couleur d'un travail ; `None` = défaut de Ghostscript.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Couleur {
    pub entree_cmjn: Option<String>,
    pub entree_rvb: Option<String>,
    pub sortie: Option<String>,
    /// 0 perceptuel, 1 colorimétrique relatif, 2 saturation, 3 colorimétrique absolu.
    pub intention: u8,
    /// Ignorer les profils embarqués dans le PDF au profit des profils d'entrée.
    pub forcer: bool,
}

/// Lit l'en-tête d'un profil ICC ; `None` si ce n'en est pas un.
pub fn lire_entete(octets: &[u8], fichier: &str) -> Option<Profil> {
    if octets.len() < 132 || &octets[36..40] != b"acsp" {
        return None;
    }
    let taille = u32::from_be_bytes(octets[0..4].try_into().ok()?) as usize;
    if taille < 132 || taille > octets.len() {
        return None;
    }
    let texte = |a: usize, b: usize| String::from_utf8_lossy(&octets[a..b]).trim().to_string();
    Some(Profil { fichier: fichier.to_string(), classe: texte(12, 16), espace: texte(16, 20), description: description(octets).unwrap_or_default() })
}

/// Balise `desc` : ICC v2 (`desc`, ASCII) ou v4 (`mluc`, UTF-16BE, premier texte).
fn description(o: &[u8]) -> Option<String> {
    let mot = |a: usize| -> Option<usize> { Some(u32::from_be_bytes(o.get(a..a + 4)?.try_into().ok()?) as usize) };
    let n = mot(128)?;
    for i in 0..n.min(200) {
        let e = 132 + 12 * i;
        if o.get(e..e + 4)? != b"desc" {
            continue;
        }
        let (pos, long) = (mot(e + 4)?, mot(e + 8)?);
        let d = o.get(pos..pos + long)?;
        let mot_d = |a: usize| -> Option<usize> { Some(u32::from_be_bytes(d.get(a..a + 4)?.try_into().ok()?) as usize) };
        return match d.get(0..4)? {
            b"desc" => {
                let l = mot_d(8)?;
                Some(String::from_utf8_lossy(d.get(12..12 + l)?).trim_end_matches('\0').trim().to_string())
            }
            b"mluc" => {
                let (l, p) = (mot_d(20)?, mot_d(24)?);
                let u: Vec<u16> = d.get(p..p + l)?.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
                Some(String::from_utf16_lossy(&u).trim_end_matches('\0').trim().to_string())
            }
            _ => None,
        };
    }
    None
}

/// Nom de fichier acceptable pour un profil déposé : lettres, chiffres, `.-_ `, en .icc/.icm.
pub fn nom_valide(nom: &str) -> bool {
    let bas = nom.to_lowercase();
    (bas.ends_with(".icc") || bas.ends_with(".icm"))
        && nom.len() <= 120
        && !nom.starts_with('.')
        && !nom.contains("..")
        && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
}

pub fn lister(dossier: &Path) -> Vec<Profil> {
    let Ok(entrees) = std::fs::read_dir(dossier) else { return vec![] };
    let mut v: Vec<Profil> = entrees
        .flatten()
        .filter_map(|e| {
            let nom = e.file_name().into_string().ok()?;
            nom_valide(&nom).then_some(())?;
            lire_entete(&std::fs::read(e.path()).ok()?, &nom)
        })
        .collect();
    v.sort_by(|a, b| a.fichier.cmp(&b.fichier));
    v
}

/// Vérifie les choix d'un travail contre les profils présents et leur espace.
pub fn verifier(c: &Couleur, profils: &[Profil]) -> Result<(), &'static str> {
    let espace = |f: &Option<String>| f.as_ref().map(|f| profils.iter().find(|p| &p.fichier == f).map(|p| p.espace.as_str()));
    let attendus = [(espace(&c.entree_cmjn), "CMYK"), (espace(&c.entree_rvb), "RGB"), (espace(&c.sortie), "CMYK")];
    for (trouve, attendu) in attendus {
        match trouve {
            None => {}
            Some(None) => return Err("profil_inconnu"),
            Some(Some(e)) if e != attendu => return Err("profil_inadapte"),
            Some(Some(_)) => {}
        }
    }
    if c.intention > 3 {
        return Err("profil_inadapte");
    }
    Ok(())
}

/// Options Ghostscript correspondant aux réglages (chemins absolus vers les profils).
pub fn options_ghostscript(c: &Couleur, dossier: &Path) -> Vec<String> {
    let chemin = |f: &str| dossier.join(f).display().to_string();
    let mut o = vec![format!("-dRenderIntent={}", c.intention)];
    if let Some(f) = &c.entree_cmjn {
        o.push(format!("-sDefaultCMYKProfile={}", chemin(f)));
    }
    if let Some(f) = &c.entree_rvb {
        o.push(format!("-sDefaultRGBProfile={}", chemin(f)));
    }
    if let Some(f) = &c.sortie {
        o.push(format!("-sOutputICCProfile={}", chemin(f)));
    }
    if c.forcer {
        o.push("-dOverrideICC".into());
    }
    o
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Dossier de profils livrés avec Ghostscript (Homebrew ou Debian/Ubuntu).
    pub fn profils_ghostscript() -> std::path::PathBuf {
        ["/opt/homebrew/share/ghostscript/iccprofiles", "/usr/share/color/icc/ghostscript", "/usr/share/ghostscript/iccprofiles"]
            .iter()
            .map(std::path::PathBuf::from)
            .find(|d| d.join("ps_cmyk.icc").exists())
            .expect("profils ICC de Ghostscript introuvables")
    }

    #[test]
    fn entete_des_profils_de_ghostscript() {
        let d = profils_ghostscript();
        let cmjn = lire_entete(&std::fs::read(d.join("ps_cmyk.icc")).unwrap(), "ps_cmyk.icc").unwrap();
        assert_eq!((cmjn.classe.as_str(), cmjn.espace.as_str()), ("prtr", "CMYK"));
        let rvb = lire_entete(&std::fs::read(d.join("default_rgb.icc")).unwrap(), "default_rgb.icc").unwrap();
        assert_eq!(rvb.espace, "RGB");
        assert!(!cmjn.description.is_empty(), "description lue");
        assert!(lire_entete(b"%PDF-1.4 pas un profil", "x.icc").is_none());
        assert!(lire_entete(&[0u8; 200], "x.icc").is_none());
    }

    #[test]
    fn noms_de_fichiers() {
        assert!(nom_valide("EpsonWideCMYK_Ver2.icc"));
        assert!(nom_valide("Vinyle brillant 720.ICM"));
        assert!(!nom_valide("../etc/passwd.icc"));
        assert!(!nom_valide("profil.pdf"));
        assert!(!nom_valide(".cache.icc"));
    }

    #[test]
    fn choix_controles() {
        let p = |f: &str, e: &str| Profil { fichier: f.into(), classe: "prtr".into(), espace: e.into(), description: String::new() };
        let profils = [p("cmjn.icc", "CMYK"), p("rvb.icc", "RGB")];
        let ok = Couleur { entree_cmjn: Some("cmjn.icc".into()), entree_rvb: Some("rvb.icc".into()), sortie: Some("cmjn.icc".into()), intention: 1, forcer: true };
        assert_eq!(verifier(&ok, &profils), Ok(()));
        let inconnu = Couleur { sortie: Some("absent.icc".into()), ..Default::default() };
        assert_eq!(verifier(&inconnu, &profils), Err("profil_inconnu"));
        let rvb_en_sortie = Couleur { sortie: Some("rvb.icc".into()), ..Default::default() };
        assert_eq!(verifier(&rvb_en_sortie, &profils), Err("profil_inadapte"));
        let o = options_ghostscript(&ok, Path::new("/p"));
        assert_eq!(o, ["-dRenderIntent=1", "-sDefaultCMYKProfile=/p/cmjn.icc", "-sDefaultRGBProfile=/p/rvb.icc", "-sOutputICCProfile=/p/cmjn.icc", "-dOverrideICC"]);
    }
}
