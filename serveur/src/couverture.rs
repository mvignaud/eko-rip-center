//! Couverture d'encre et coût estimé. La couverture d'une plaque est mesurée sur son
//! aperçu (50 dpi) : la moyenne d'encre d'une surface ne change pas avec la définition, et
//! une plaque de production de bâche ferait plusieurs Go en mémoire. Les tarifs (ml/m² à
//! 100 % et prix au litre par encre, prix du support au m²) sont gardés dans
//! `<données>/tarifs.json`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const M2_PAR_POUCE2: f64 = 0.0254 * 0.0254;

/// Surface de la page (m²) et part d'encre de chaque plaque (0 à 1).
#[derive(Debug, PartialEq)]
pub struct Mesure {
    pub surface_m2: f64,
    pub couverture: BTreeMap<String, f64>,
}

/// Mesure les plaques d'une page sur leurs aperçus `apercus[i]` (rendus à `dpi`).
/// Convention des plaques : 0 = pleine encre, 255 = pas d'encre.
pub fn mesurer(apercus: &[(String, PathBuf)], dpi: f64) -> Result<Mesure, String> {
    let mut surface_m2 = 0.0;
    let mut couverture = BTreeMap::new();
    for (nom, chemin) in apercus {
        let img = image::open(chemin).map_err(|e| format!("aperçu de {nom} : {e}"))?.to_luma8();
        let pixels = img.as_raw();
        let encre: u64 = pixels.iter().map(|&v| 255 - v as u64).sum();
        couverture.insert(nom.clone(), if pixels.is_empty() { 0.0 } else { encre as f64 / (255.0 * pixels.len() as f64) });
        surface_m2 = img.width() as f64 * img.height() as f64 / (dpi * dpi) * M2_PAR_POUCE2;
    }
    Ok(Mesure { surface_m2, couverture })
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Encre {
    /// Encre déposée à 100 % de couverture, en ml par m².
    pub ml_m2: f64,
    pub prix_litre: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tarifs {
    /// Par nom de plaque (`Cyan`, `White`…) ; les autres prennent `encre_defaut`.
    #[serde(default)]
    pub encres: BTreeMap<String, Encre>,
    pub encre_defaut: Encre,
    /// Prix du support au m².
    #[serde(default)]
    pub support_m2: f64,
}

impl Default for Tarifs {
    // ponytail: valeurs de départ à remplacer par les mesures de l'atelier (pesée des
    // cartouches, factures) ; rien n'est juste tant qu'elles ne sont pas réglées.
    fn default() -> Self {
        Tarifs { encres: BTreeMap::new(), encre_defaut: Encre { ml_m2: 10.0, prix_litre: 140.0 }, support_m2: 0.0 }
    }
}

impl Tarifs {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = |v: f64| v.is_finite() && (0.0..=100_000.0).contains(&v);
        let encres_bonnes = self.encres.iter().chain([(&String::new(), &self.encre_defaut)]).all(|(n, e)| n.len() <= 60 && bon(e.ml_m2) && bon(e.prix_litre));
        if encres_bonnes && bon(self.support_m2) { Ok(()) } else { Err("tarifs_invalides") }
    }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("tarifs.json")
}

pub fn lire(donnees: &Path) -> Tarifs {
    std::fs::read(fichier(donnees)).ok().and_then(|o| serde_json::from_slice(&o).ok()).unwrap_or_default()
}

pub fn ecrire(donnees: &Path, t: &Tarifs) -> std::io::Result<()> {
    let tmp = fichier(donnees).with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(t).unwrap())?;
    std::fs::rename(tmp, fichier(donnees))
}

#[derive(Debug, PartialEq, Serialize)]
pub struct LigneEncre {
    pub nom: String,
    pub couverture: f64,
    pub ml: f64,
    pub cout: f64,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Estimation {
    pub surface_m2: f64,
    pub encres: Vec<LigneEncre>,
    pub ml: f64,
    pub cout_encres: f64,
    pub cout_support: f64,
    pub total: f64,
}

/// Encre et coût d'une page ; les plaques `exclues` (tons de découpe) ne comptent pas.
// ponytail: le support compté est la surface de la page, pas la longueur de rouleau
// consommée sur toute la laize ; à revoir avec la sortie réelle de l'Epson.
pub fn estimer(surface_m2: f64, couverture: &BTreeMap<String, f64>, exclues: &[String], t: &Tarifs) -> Estimation {
    let encres: Vec<LigneEncre> = couverture
        .iter()
        .filter(|(nom, _)| !exclues.contains(nom))
        .map(|(nom, &c)| {
            let e = t.encres.get(nom).copied().unwrap_or(t.encre_defaut);
            let ml = c * surface_m2 * e.ml_m2;
            LigneEncre { nom: nom.clone(), couverture: c, ml, cout: ml / 1000.0 * e.prix_litre }
        })
        .collect();
    let (ml, cout_encres) = encres.iter().fold((0.0, 0.0), |(m, c), l| (m + l.ml, c + l.cout));
    let cout_support = surface_m2 * t.support_m2;
    Estimation { surface_m2, encres, ml, cout_encres, cout_support, total: cout_encres + cout_support }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couverture_mesuree_sur_l_apercu() {
        let d = std::env::temp_dir().join(format!("eko-rip-couverture-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        // 100 × 50 px à 50 dpi : 2 × 1 pouce. Moitié gauche pleine encre (0), droite vide.
        let img = image::GrayImage::from_fn(100, 50, |x, _| image::Luma([if x < 50 { 0 } else { 255 }]));
        img.save(d.join("c.png")).unwrap();
        image::GrayImage::from_pixel(100, 50, image::Luma([255])).save(d.join("v.png")).unwrap();
        let m = mesurer(&[("Cyan".into(), d.join("c.png")), ("White".into(), d.join("v.png"))], 50.0).unwrap();
        assert!((m.couverture["Cyan"] - 0.5).abs() < 1e-9);
        assert_eq!(m.couverture["White"], 0.0);
        assert!((m.surface_m2 - 2.0 * M2_PAR_POUCE2).abs() < 1e-12);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn cout_par_encre_et_support() {
        let mut t = Tarifs { support_m2: 5.0, ..Default::default() };
        t.encres.insert("White".into(), Encre { ml_m2: 20.0, prix_litre: 200.0 });
        let couverture = BTreeMap::from([("Cyan".into(), 0.5), ("White".into(), 1.0), ("CutContour".into(), 0.01)]);
        let e = estimer(2.0, &couverture, &["CutContour".into()], &t);
        assert_eq!(e.encres.iter().map(|l| l.nom.as_str()).collect::<Vec<_>>(), ["Cyan", "White"], "tons de découpe exclus");
        // Cyan : 0,5 × 2 m² × 10 ml = 10 ml à 140 €/l = 1,40 € ; blanc : 40 ml à 200 €/l = 8 €.
        assert!((e.encres[0].ml - 10.0).abs() < 1e-9 && (e.encres[0].cout - 1.4).abs() < 1e-9);
        assert!((e.encres[1].ml - 40.0).abs() < 1e-9 && (e.encres[1].cout - 8.0).abs() < 1e-9);
        assert!((e.cout_support - 10.0).abs() < 1e-9 && (e.total - 19.4).abs() < 1e-9 && (e.ml - 50.0).abs() < 1e-9);
        assert!(Tarifs { support_m2: f64::NAN, ..Default::default() }.verifier().is_err());
        assert!(Tarifs::default().verifier().is_ok());
    }
}
