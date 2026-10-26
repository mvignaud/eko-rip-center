//! Règles du dossier surveillé automatique (roadmap 9.1) : un fichier déposé dans
//! `<données>/entrees-auto/` prend le réglage de la première règle qui lui correspond, par
//! motif de nom (`BACHE_*`) et, pour un PDF, par format de la première page.
//!
//! Le format se compare par petit et grand côté, pour qu'une affiche en paysage et la même
//! en portrait suivent la même règle. Un fichier qu'aucune règle ne prend est rangé dans
//! `refuses/` avec sa raison, comme dans le dossier d'un réglage.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Regle {
    /// Motif du nom de fichier, sans tenir compte de la casse : `*` pour n'importe quelle
    /// suite de caractères, `?` pour un seul.
    pub motif: String,
    /// Petit côté de la première page (mm), bornes comprises ; absent : tout format.
    #[serde(default)]
    pub petit_cote_mm: Option<[f64; 2]>,
    /// Grand côté de la première page (mm), bornes comprises.
    #[serde(default)]
    pub grand_cote_mm: Option<[f64; 2]>,
    /// Nom du réglage enregistré appliqué.
    pub reglage: String,
}

impl Regle {
    /// `format` : petit et grand côté (mm), s'ils sont connus. Une règle de format ne prend
    /// pas un fichier de format inconnu.
    pub fn prend(&self, nom: &str, format: Option<[f64; 2]>) -> bool {
        let dans = |b: Option<[f64; 2]>, v: Option<f64>| b.is_none_or(|[min, max]| v.is_some_and(|v| v >= min && v <= max));
        motif(&self.motif, nom) && dans(self.petit_cote_mm, format.map(|f| f[0])) && dans(self.grand_cote_mm, format.map(|f| f[1]))
    }
}

/// Première règle qui prend le fichier.
pub fn choisir<'a>(regles: &'a [Regle], nom: &str, format: Option<[f64; 2]>) -> Option<&'a Regle> {
    regles.iter().find(|r| r.prend(nom, format))
}

/// Motif `*` / `?`, insensible à la casse, sur tout le nom.
pub fn motif(motif: &str, nom: &str) -> bool {
    let (m, n): (Vec<char>, Vec<char>) = (motif.to_lowercase().chars().collect(), nom.to_lowercase().chars().collect());
    // Retour en arrière sur la dernière étoile : linéaire en pratique, sans récursion.
    let (mut i, mut j, mut etoile, mut reprise) = (0, 0, None, 0);
    while j < n.len() {
        if i < m.len() && (m[i] == '?' || m[i] == n[j]) {
            i += 1;
            j += 1;
        } else if i < m.len() && m[i] == '*' {
            etoile = Some(i);
            reprise = j;
            i += 1;
        } else if let Some(e) = etoile {
            i = e + 1;
            reprise += 1;
            j = reprise;
        } else {
            return false;
        }
    }
    m[i..].iter().all(|&c| c == '*')
}

/// Vérifie la liste : motif non vide (200 caractères), bornes ordonnées entre 0 et
/// 100 m, réglage existant.
pub fn verifier(regles: &[Regle], reglages: &[String]) -> Result<(), &'static str> {
    let borne = |b: Option<[f64; 2]>| b.is_none_or(|[min, max]| min.is_finite() && max.is_finite() && 0.0 <= min && min <= max && max <= 100_000.0);
    let bon = regles.len() <= 200
        && regles.iter().all(|r| {
            !r.motif.trim().is_empty() && r.motif.chars().count() <= 200 && !r.motif.contains(['/', '\\']) && borne(r.petit_cote_mm) && borne(r.grand_cote_mm) && reglages.contains(&r.reglage)
        });
    if bon { Ok(()) } else { Err("regles_invalides") }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("regles.json")
}

/// Dossier surveillé des règles ; son nom ne peut pas être celui d'un réglage (`entrees/<nom>`).
pub fn dossier(donnees: &Path) -> PathBuf {
    donnees.join("entrees-auto")
}

pub fn lire(donnees: &Path) -> Vec<Regle> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, regles: &[Regle]) -> std::io::Result<()> {
    std::fs::create_dir_all(dossier(donnees))?;
    crate::etat::ecrire(&fichier(donnees), regles, false)
}

/// Au-delà, le format n'est pas lu (le PDF entier passerait en mémoire) : seules les règles
/// sans format peuvent prendre le fichier.
const FORMAT_MAX_OCTETS: u64 = 512 << 20;

/// Petit et grand côté (mm) de la première page d'un PDF : CropBox, sinon MediaBox, héritées
/// des nœuds parents comme le veut le format.
pub fn format_pdf(chemin: &Path) -> Option<[f64; 2]> {
    if std::fs::metadata(chemin).ok()?.len() > FORMAT_MAX_OCTETS {
        return None;
    }
    let doc = lopdf::Document::load(chemin).ok()?;
    let (_, id) = doc.get_pages().into_iter().next()?;
    let mut noeud = doc.get_dictionary(id).ok()?;
    let boite = loop {
        if let Some(b) = [b"CropBox".as_slice(), b"MediaBox"].iter().find_map(|k| noeud.get(k).ok()) {
            break doc.dereference(b).ok()?.1.as_array().ok()?.clone();
        }
        noeud = doc.get_dictionary(noeud.get(b"Parent").ok()?.as_reference().ok()?).ok()?;
    };
    let v: Vec<f64> = boite.iter().filter_map(|o| doc.dereference(o).ok()?.1.as_float().ok().map(f64::from)).collect();
    let [x0, y0, x1, y1] = v[..] else { return None };
    let mm = |pt: f64| pt.abs() * 25.4 / 72.0;
    let (l, h) = (mm(x1 - x0), mm(y1 - y0));
    Some([l.min(h), l.max(h)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regle(motif: &str, reglage: &str) -> Regle {
        Regle { motif: motif.into(), petit_cote_mm: None, grand_cote_mm: None, reglage: reglage.into() }
    }

    #[test]
    fn motifs() {
        assert!(motif("BACHE_*", "BACHE_client-12.pdf"));
        assert!(motif("bache_*", "Bache_Client.PDF"), "casse ignorée");
        assert!(!motif("BACHE_*", "affiche_BACHE_1.pdf"), "tout le nom, pas une partie");
        assert!(motif("*_A?.pdf", "Plan_A0.pdf") && !motif("*_A?.pdf", "Plan_A10.pdf"));
        assert!(motif("*", "") && motif("**a*", "xxa") && !motif("a*b", "acbx"));
        assert!(motif("*.tif*", "photo.tiff") && motif("Vitrine*Été*", "vitrine 2026 été.pdf"));
    }

    #[test]
    fn premiere_regle_qui_prend() {
        let mut grande = regle("*", "Bâche XXL");
        grande.grand_cote_mm = Some([2000.0, 100_000.0]);
        let regles = vec![regle("BACHE_*", "Bâche"), grande, regle("*.pdf", "Vinyle")];
        assert_eq!(choisir(&regles, "BACHE_toto.pdf", Some([1000.0, 3000.0])).unwrap().reglage, "Bâche");
        assert_eq!(choisir(&regles, "affiche.pdf", Some([1000.0, 3000.0])).unwrap().reglage, "Bâche XXL");
        assert_eq!(choisir(&regles, "affiche.pdf", Some([297.0, 420.0])).unwrap().reglage, "Vinyle");
        assert_eq!(choisir(&regles, "affiche.pdf", None).unwrap().reglage, "Vinyle", "format inconnu : la règle de format ne prend pas");
        assert!(choisir(&regles, "photo.jpg", None).is_none());
        let noms = ["Bâche".to_string(), "Bâche XXL".into(), "Vinyle".into()];
        assert_eq!(verifier(&regles, &noms), Ok(()));
        for casse in [regle(" ", "Vinyle"), regle("a/b", "Vinyle"), regle("*", "Inconnu"), Regle { petit_cote_mm: Some([10.0, 5.0]), ..regle("*", "Vinyle") }, Regle { grand_cote_mm: Some([f64::NAN, 5.0]), ..regle("*", "Vinyle") }] {
            assert_eq!(verifier(std::slice::from_ref(&casse), &noms), Err("regles_invalides"), "{casse:?}");
        }
    }

    #[test]
    fn format_de_la_premiere_page() {
        use lopdf::{Document, Object, dictionary};
        let d = std::env::temp_dir().join(format!("eko-regles-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        // MediaBox héritée du nœud Pages, page en paysage : A3 → [297, 420].
        let mut doc = Document::with_version("1.7");
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1, "MediaBox" => vec![0.into(), 0.into(), Object::Real(1190.55), Object::Real(841.89)] }));
        let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalogue);
        let chemin = d.join("a3.pdf");
        doc.save(&chemin).unwrap();
        let [p, g] = format_pdf(&chemin).unwrap();
        assert!((p - 297.0).abs() < 0.1 && (g - 420.0).abs() < 0.1, "{p} {g}");
        std::fs::write(d.join("faux.pdf"), b"%PDF-1.7 rien").unwrap();
        assert_eq!(format_pdf(&d.join("faux.pdf")), None);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
