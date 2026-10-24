//! Couverture d'encre et coût estimé. La couverture d'une plaque est mesurée sur son
//! aperçu (50 dpi) : la moyenne d'encre d'une surface ne change pas avec la définition, et
//! une plaque de production de bâche ferait plusieurs Go en mémoire. Les tarifs (ml/m² à
//! 100 % et prix au litre par encre, prix du support au m²) sont gardés dans
//! `<données>/tarifs.json`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const M2_PAR_POUCE2: f64 = 0.0254 * 0.0254;

/// Surface et format de la page, part d'encre de chaque plaque (0 à 1).
#[derive(Debug, PartialEq, Default)]
pub struct Mesure {
    pub surface_m2: f64,
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
    pub couverture: BTreeMap<String, f64>,
}

/// Mesure les plaques d'une page sur leurs aperçus `apercus[i]` (rendus à `dpi`).
/// Convention des plaques : 0 = pleine encre, 255 = pas d'encre.
pub fn mesurer(apercus: &[(String, PathBuf)], dpi: f64) -> Result<Mesure, String> {
    let mut m = Mesure::default();
    for (nom, chemin) in apercus {
        let img = image::open(chemin).map_err(|e| format!("aperçu de {nom} : {e}"))?.to_luma8();
        let pixels = img.as_raw();
        let encre: u64 = pixels.iter().map(|&v| 255 - v as u64).sum();
        m.couverture.insert(nom.clone(), if pixels.is_empty() { 0.0 } else { encre as f64 / (255.0 * pixels.len() as f64) });
        m.surface_m2 = img.width() as f64 * img.height() as f64 / (dpi * dpi) * M2_PAR_POUCE2;
        (m.largeur_mm, m.hauteur_mm) = (img.width() as f64 / dpi * 25.4, img.height() as f64 / dpi * 25.4);
    }
    Ok(m)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Encre {
    /// Encre déposée à 100 % de couverture, en ml par m².
    pub ml_m2: f64,
    pub prix_litre: f64,
    /// Contenance et prix de la cartouche : s'ils sont donnés, ils fixent le prix au litre.
    #[serde(default)]
    pub cartouche: Option<Cartouche>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cartouche {
    pub contenance_ml: f64,
    pub prix: f64,
}

impl Encre {
    pub fn prix_litre(&self) -> f64 {
        self.cartouche.filter(|c| c.contenance_ml > 0.0).map_or(self.prix_litre, |c| c.prix / c.contenance_ml * 1000.0)
    }
}

/// Support (rouleau) du catalogue : prix au m² donné, ou calculé depuis le rouleau acheté.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Support {
    pub nom: String,
    #[serde(default)]
    pub prix_m2: Option<f64>,
    #[serde(default)]
    pub rouleau: Option<Rouleau>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rouleau {
    pub largeur_mm: f64,
    pub longueur_m: f64,
    /// Prix payé pour le rouleau entier.
    pub prix: f64,
}

impl Support {
    pub fn prix_m2(&self) -> f64 {
        match (self.prix_m2, self.rouleau) {
            (Some(p), _) => p,
            (None, Some(r)) if r.largeur_mm > 0.0 && r.longueur_m > 0.0 => r.prix / (r.largeur_mm / 1000.0 * r.longueur_m),
            _ => 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tarifs {
    /// Par nom de plaque (`Cyan`, `White`…) ; les autres prennent `encre_defaut`.
    #[serde(default)]
    pub encres: BTreeMap<String, Encre>,
    pub encre_defaut: Encre,
    /// Prix au m² d'un travail sans support choisi.
    #[serde(default)]
    pub support_m2: f64,
    /// Catalogue des supports, choisis par travail.
    #[serde(default)]
    pub supports: Vec<Support>,
    /// Prix saisis toutes taxes comprises (sinon hors taxe).
    #[serde(default = "vrai")]
    pub saisie_ttc: bool,
    /// Taux de TVA en %, pour passer de l'un à l'autre.
    #[serde(default = "tva_defaut")]
    pub tva_pct: f64,
    /// Prix de revient affiché toutes taxes comprises (sinon hors taxe).
    #[serde(default)]
    pub affichage_ttc: bool,
}

fn vrai() -> bool {
    true
}

fn tva_defaut() -> f64 {
    20.0
}

impl Tarifs {
    /// Facteur qui passe des prix saisis aux prix affichés (HT ↔ TTC).
    pub fn conversion(&self) -> f64 {
        let t = 1.0 + self.tva_pct / 100.0;
        match (self.saisie_ttc, self.affichage_ttc) {
            (true, false) => 1.0 / t,
            (false, true) => t,
            _ => 1.0,
        }
    }
}

impl Default for Tarifs {
    // Cartouche Epson donnée par l'atelier le 24/10/2026 : 700 ml à 123,90 € TTC (177 €/l).
    // ponytail: 10 ml/m² à 100 % reste une valeur de départ, à mesurer à l'atelier.
    fn default() -> Self {
        let cartouche = Some(Cartouche { contenance_ml: 700.0, prix: 123.90 });
        // Premier support du catalogue, donné le même jour : HEXIS V3000WG à 1,97 €/m², rouleau
        // de 1600 mm × 45 m (141,84 €).
        let hexis = Support { nom: "HEXIS V3000WG".into(), prix_m2: Some(1.97), rouleau: Some(Rouleau { largeur_mm: 1600.0, longueur_m: 45.0, prix: 141.84 }) };
        // Prix saisis TTC (comme donnés), prix de revient affiché HT.
        Tarifs { encres: BTreeMap::new(), encre_defaut: Encre { ml_m2: 10.0, prix_litre: 177.0, cartouche }, support_m2: 0.0, supports: vec![hexis], saisie_ttc: true, tva_pct: 20.0, affichage_ttc: false }
    }
}

impl Tarifs {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = |v: f64| v.is_finite() && (0.0..=100_000.0).contains(&v);
        let encres_bonnes = self.encres.iter().chain([(&String::new(), &self.encre_defaut)]).all(|(n, e)| {
            n.len() <= 60 && bon(e.ml_m2) && bon(e.prix_litre) && e.cartouche.is_none_or(|c| bon(c.contenance_ml) && c.contenance_ml > 0.0 && bon(c.prix))
        });
        let supports_bons = self.supports.iter().enumerate().all(|(i, s)| {
            !s.nom.trim().is_empty()
                && s.nom.chars().count() <= 60
                && !self.supports[..i].iter().any(|x| x.nom == s.nom)
                && s.prix_m2.is_none_or(bon)
                && s.rouleau.is_none_or(|r| bon(r.largeur_mm) && r.largeur_mm > 0.0 && bon(r.longueur_m) && r.longueur_m > 0.0 && bon(r.prix))
        });
        if encres_bonnes && supports_bons && bon(self.support_m2) && (0.0..=100.0).contains(&self.tva_pct) { Ok(()) } else { Err("tarifs_invalides") }
    }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("tarifs.json")
}

pub fn lire(donnees: &Path) -> Tarifs {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, t: &Tarifs) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), t, false)
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
    /// Coûts toutes taxes comprises (sinon hors taxe).
    pub ttc: bool,
    pub surface_m2: f64,
    /// Support compté (catalogue) et surface de support consommée (m²).
    pub support: Option<String>,
    pub surface_support_m2: f64,
    pub encres: Vec<LigneEncre>,
    pub ml: f64,
    pub cout_encres: f64,
    pub cout_support: f64,
    pub total: f64,
}

/// Encre et coût d'une page ; les plaques `exclues` (tons de découpe) ne comptent pas.
/// Avec un support du catalogue de largeur connue, la matière consommée est toute la
/// largeur du rouleau sur la hauteur de la page (sens de l'avance) ; sinon la surface de
/// la page au prix par défaut.
// ponytail: la page est supposée posée dans le sens du rouleau (sa largeur en travers).
pub fn estimer(surface_m2: f64, hauteur_mm: f64, couverture: &BTreeMap<String, f64>, exclues: &[String], t: &Tarifs, support: Option<&str>) -> Estimation {
    let encres: Vec<LigneEncre> = couverture
        .iter()
        .filter(|(nom, _)| !exclues.contains(nom))
        .map(|(nom, &c)| {
            let e = t.encres.get(nom).copied().unwrap_or(t.encre_defaut);
            let ml = c * surface_m2 * e.ml_m2;
            LigneEncre { nom: nom.clone(), couverture: c, ml, cout: ml / 1000.0 * e.prix_litre() * t.conversion() }
        })
        .collect();
    let (ml, cout_encres) = encres.iter().fold((0.0, 0.0), |(m, c), l| (m + l.ml, c + l.cout));
    let s = support.and_then(|n| t.supports.iter().find(|s| s.nom == n));
    let (surface_support_m2, prix_m2) = match s {
        Some(s) => (s.rouleau.filter(|_| hauteur_mm > 0.0 && surface_m2 > 0.0).map_or(surface_m2, |r| r.largeur_mm / 1000.0 * hauteur_mm / 1000.0), s.prix_m2()),
        None => (surface_m2, t.support_m2),
    };
    let cout_support = surface_support_m2 * prix_m2 * t.conversion();
    Estimation { ttc: t.affichage_ttc, surface_m2, support: s.map(|s| s.nom.clone()), surface_support_m2, encres, ml, cout_encres, cout_support, total: cout_encres + cout_support }
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
        let mut t = Tarifs { support_m2: 5.0, affichage_ttc: true, ..Default::default() };
        t.encres.insert("White".into(), Encre { ml_m2: 20.0, prix_litre: 200.0, cartouche: None });
        let couverture = BTreeMap::from([("Cyan".into(), 0.5), ("White".into(), 1.0), ("CutContour".into(), 0.01)]);
        let e = estimer(2.0, 0.0, &couverture, &["CutContour".into()], &t, None);
        assert_eq!(e.encres.iter().map(|l| l.nom.as_str()).collect::<Vec<_>>(), ["Cyan", "White"], "tons de découpe exclus");
        // Cyan : 0,5 × 2 m² × 10 ml = 10 ml à 177 €/l (cartouche 700 ml à 123,90 €) = 1,77 € ;
        // blanc : 40 ml à 200 €/l = 8 €.
        assert!((e.encres[0].ml - 10.0).abs() < 1e-9 && (e.encres[0].cout - 1.77).abs() < 1e-9);
        assert!((e.encres[1].ml - 40.0).abs() < 1e-9 && (e.encres[1].cout - 8.0).abs() < 1e-9);
        assert!((e.cout_support - 10.0).abs() < 1e-9 && (e.total - 19.77).abs() < 1e-9 && (e.ml - 50.0).abs() < 1e-9);
        assert!(Tarifs { support_m2: f64::NAN, ..Default::default() }.verifier().is_err());
        assert!(Tarifs::default().verifier().is_ok());
    }

    #[test]
    fn hors_taxe_ou_toutes_taxes() {
        // 1 m² de bâche à 12 € TTC, sans encre : 10 € HT à 20 % de TVA.
        let base = Tarifs { supports: vec![Support { nom: "B".into(), prix_m2: Some(12.0), rouleau: None }], ..Default::default() };
        let cout = |t: &Tarifs| estimer(1.0, 0.0, &BTreeMap::new(), &[], t, Some("B"));
        let ht = cout(&base);
        assert!(!ht.ttc && (ht.total - 10.0).abs() < 1e-9, "saisi TTC, affiché HT par défaut");
        assert!((cout(&Tarifs { affichage_ttc: true, ..base.clone() }).total - 12.0).abs() < 1e-9);
        // Saisi HT, affiché TTC : 12 × 1,2 = 14,40.
        let t = Tarifs { saisie_ttc: false, affichage_ttc: true, ..base.clone() };
        assert!(cout(&t).ttc && (cout(&t).total - 14.4).abs() < 1e-9);
        assert!(Tarifs { tva_pct: 150.0, ..base }.verifier().is_err());
    }

    #[test]
    fn valeurs_livrees_de_l_atelier() {
        let t = Tarifs::default();
        assert!((t.encre_defaut.prix_litre() - 177.0).abs() < 1e-9, "700 ml à 123,90 €");
        let h = &t.supports[0];
        assert_eq!(h.nom, "HEXIS V3000WG");
        assert!((h.prix_m2() - 1.97).abs() < 1e-9);
        let r = h.rouleau.unwrap();
        assert!((r.prix - r.largeur_mm / 1000.0 * r.longueur_m * 1.97).abs() < 0.005, "rouleau cohérent avec le prix au m²");
        assert!(t.verifier().is_ok());
    }

    #[test]
    fn cartouche_et_rouleau_donnent_le_prix_de_revient() {
        // Cartouche de 700 ml à 98 € : 140 €/l, quel que soit le prix au litre saisi.
        let cyan = Encre { ml_m2: 10.0, prix_litre: 1.0, cartouche: Some(Cartouche { contenance_ml: 700.0, prix: 98.0 }) };
        assert!((cyan.prix_litre() - 140.0).abs() < 1e-9);
        // Rouleau de 1600 mm × 50 m payé 240 € : 3 €/m².
        let vinyle = Support { nom: "Vinyle".into(), prix_m2: None, rouleau: Some(Rouleau { largeur_mm: 1600.0, longueur_m: 50.0, prix: 240.0 }) };
        assert!((vinyle.prix_m2() - 3.0).abs() < 1e-9);
        let mut t = Tarifs { supports: vec![vinyle, Support { nom: "Bâche".into(), prix_m2: Some(4.5), rouleau: None }], affichage_ttc: true, ..Default::default() };
        t.encres.insert("Cyan".into(), cyan);
        // Page de 1000 × 500 mm (0,5 m²), cyan à 50 % : 0,5 × 0,5 × 10 = 2,5 ml → 0,35 €.
        // Vinyle : toute la laize (1,6 m) sur 0,5 m = 0,8 m² à 3 € = 2,40 €.
        let couv = BTreeMap::from([("Cyan".to_string(), 0.5)]);
        let e = estimer(0.5, 500.0, &couv, &[], &t, Some("Vinyle"));
        assert_eq!(e.support.as_deref(), Some("Vinyle"));
        assert!((e.surface_support_m2 - 0.8).abs() < 1e-9 && (e.cout_support - 2.4).abs() < 1e-9 && (e.cout_encres - 0.35).abs() < 1e-9 && (e.total - 2.75).abs() < 1e-9);
        // Bâche au m² sans largeur : la surface de la page.
        let e = estimer(0.5, 500.0, &couv, &[], &t, Some("Bâche"));
        assert!((e.cout_support - 2.25).abs() < 1e-9);
        // Support inconnu (supprimé depuis) : prix par défaut, sans erreur.
        assert_eq!(estimer(0.5, 500.0, &couv, &[], &t, Some("Ancien")).support, None);
        // Deux supports du même nom, ou un rouleau de longueur nulle : refusés.
        let double = Tarifs { supports: vec![t.supports[1].clone(), t.supports[1].clone()], ..Default::default() };
        assert!(double.verifier().is_err());
        let vide = Tarifs { supports: vec![Support { nom: "x".into(), prix_m2: None, rouleau: Some(Rouleau { largeur_mm: 1600.0, longueur_m: 0.0, prix: 10.0 }) }], ..Default::default() };
        assert!(vide.verifier().is_err());
    }
}
