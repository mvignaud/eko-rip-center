//! Filtres couleur d'un travail (roadmap 3.E2) et remplacement de couleur (3.E3).
//!
//! Espace : les encres de la machine, après la conversion ICC. Le filtre se pose sur les
//! plaques CMJN que rend Ghostscript (`tiffsep`), avant le blanc automatique : il agit sur ce
//! que l'imprimante recevra, quel que soit l'espace du fichier (RVB, CMJN, gris), et le
//! résultat se mesure au pixel près dans les plaques. Les tons directs (blanc, argent,
//! découpe) ne sont jamais touchés. Une fonction de transfert Ghostscript (`settransfer`)
//! aurait agi aussi sur l'aperçu RVB, mais pas sur les mêmes canaux, et ne sait pas faire la
//! saturation ni le remplacement, qui mélangent les canaux.
//!
//! Calcul d'un pixel, en quantité d'encre (0 = rien, 255 = pleine) : si la valeur d'origine
//! tombe dans un remplacement actif (le premier de la liste), elle devient sa cible, telle
//! quelle ; sinon courbe de son canal, puis luminosité, puis saturation.

use crate::blanc::{EcrivainTiff, LecteurTiff, RANGS_PAR_BANDE};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Plaques d'encre de quadrichromie, dans l'ordre C, M, J, N.
pub const ENCRES: [&str; 4] = ["Cyan", "Magenta", "Yellow", "Black"];

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Filtre {
    /// Courbe de chaque encre (C, M, J, N) : points (entrée %, sortie %), segments droits
    /// entre eux ; (0, 0) et (100, 100) sont ajoutés s'ils manquent. Vide : pas de courbe.
    /// Deux points font des niveaux (ex. [[0, 0], [100, 85]] : 85 % d'encre au plus).
    #[serde(default)]
    pub courbes: [Vec<[f64; 2]>; 4],
    /// En % (−100 à 100) : positif éclaircit (moins d'encre sur les quatre canaux), négatif
    /// fonce (jusqu'au double). Le blanc du support reste blanc.
    #[serde(default)]
    pub luminosite: f64,
    /// En % (−100 à 100) : écart des C, M, J à leur part grise (le plus petit des trois),
    /// agrandi ou réduit ; −100 = gris. Le noir n'est pas touché.
    #[serde(default)]
    pub saturation: f64,
    #[serde(default)]
    pub remplacements: Vec<Remplacement>,
}

/// Couleur d'origine (CMJN en %) → couleur voulue, sur une plage : chaque canal à au plus
/// `tolerance` % de `de`.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Remplacement {
    pub de: [f64; 4],
    pub vers: [f64; 4],
    #[serde(default)]
    pub tolerance: f64,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn vrai() -> bool {
    true
}

impl Filtre {
    pub fn neutre(&self) -> bool {
        self.courbes.iter().all(Vec::is_empty) && self.luminosite == 0.0 && self.saturation == 0.0 && !self.remplacements.iter().any(|r| r.actif)
    }

    pub fn verifier(&self) -> Result<(), &'static str> {
        let pc = |v: &f64| (0.0..=100.0).contains(v);
        let courbes = self.courbes.iter().all(|c| c.len() <= 16 && c.iter().flatten().all(pc) && c.windows(2).all(|p| p[0][0] < p[1][0]));
        let bon = courbes
            && (-100.0..=100.0).contains(&self.luminosite)
            && (-100.0..=100.0).contains(&self.saturation)
            && self.remplacements.len() <= 20
            && self.remplacements.iter().all(|r| r.de.iter().chain(&r.vers).all(pc) && (0.0..=50.0).contains(&r.tolerance));
        if bon { Ok(()) } else { Err("filtre_invalide") }
    }

    fn calcul(&self) -> Calcul {
        let unite = |v: f64| (v * 2.55).round().clamp(0.0, 255.0) as u8;
        let k = 1.0 - self.luminosite / 100.0;
        let tables = std::array::from_fn(|c| {
            let mut pts = self.courbes[c].clone();
            if !pts.is_empty() {
                if pts[0][0] > 0.0 {
                    pts.insert(0, [0.0, 0.0]);
                }
                if pts[pts.len() - 1][0] < 100.0 {
                    pts.push([100.0, 100.0]);
                }
            }
            std::array::from_fn(|v| {
                let x = v as f64 / 2.55;
                let y = match pts.windows(2).find(|p| x <= p[1][0]) {
                    Some(p) => p[0][1] + (p[1][1] - p[0][1]) * (x - p[0][0]) / (p[1][0] - p[0][0]),
                    None => x,
                };
                (y * 2.55 * k).round().clamp(0.0, 255.0) as u8
            })
        });
        let remplacements = self.remplacements.iter().filter(|r| r.actif).map(|r| (r.de.map(unite), r.vers.map(unite), unite(r.tolerance))).collect();
        Calcul { tables, saturation: 1.0 + self.saturation as f32 / 100.0, remplacements }
    }
}

struct Calcul {
    tables: [[u8; 256]; 4],
    saturation: f32,
    remplacements: Vec<([u8; 4], [u8; 4], u8)>,
}

impl Calcul {
    /// Un pixel, en quantité d'encre (255 = pleine).
    fn pixel(&self, o: [u8; 4]) -> [u8; 4] {
        if let Some((_, vers, _)) = self.remplacements.iter().find(|(de, _, t)| (0..4).all(|i| o[i].abs_diff(de[i]) <= *t)) {
            return *vers;
        }
        let mut e = std::array::from_fn(|i| self.tables[i][o[i] as usize]);
        if self.saturation != 1.0 {
            let gris = e[0].min(e[1]).min(e[2]) as f32;
            for v in &mut e[..3] {
                *v = (gris + (*v as f32 - gris) * self.saturation).round().clamp(0.0, 255.0) as u8;
            }
        }
        e
    }

    /// Lignes de plaques (0 = pleine encre), filtrées sur place.
    fn lignes(&self, l: &mut [Vec<u8>; 4]) {
        for x in 0..l[0].len() {
            let s = self.pixel(std::array::from_fn(|i| 255 - l[i][x]));
            for i in 0..4 {
                l[i][x] = 255 - s[i];
            }
        }
    }
}

/// Plaques de production d'une page, filtrées : lues et réécrites bande par bande (une
/// plaque de bâche fait plusieurs Go), puis remplacées ensemble.
pub fn filtrer_tiff(plaques: &[PathBuf; 4], f: &Filtre, dpi: u32) -> Result<(), String> {
    let calcul = f.calcul();
    let mut lecteurs = plaques.iter().map(|p| LecteurTiff::ouvrir(p)).collect::<Result<Vec<_>, _>>()?;
    let (largeur, hauteur) = (lecteurs[0].largeur, lecteurs[0].hauteur);
    if lecteurs.iter().any(|l| (l.largeur, l.hauteur) != (largeur, hauteur)) {
        return Err("plaques de tailles différentes".into());
    }
    let tmp: Vec<PathBuf> = plaques.iter().map(|p| p.with_extension("tif.filtre")).collect();
    let resultat = (|| -> Result<(), String> {
        let mut ecrivains = tmp.iter().map(|p| EcrivainTiff::creer(p, largeur as u32, hauteur as u32, dpi)).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        let mut bandes: [Vec<u8>; 4] = Default::default();
        for y in 0..hauteur {
            let mut l: [Vec<u8>; 4] = [vec![], vec![], vec![], vec![]];
            for (i, lecteur) in lecteurs.iter_mut().enumerate() {
                l[i] = lecteur.ligne()?;
            }
            calcul.lignes(&mut l);
            for i in 0..4 {
                bandes[i].extend_from_slice(&l[i]);
            }
            if bandes[0].len() == RANGS_PAR_BANDE * largeur || y + 1 == hauteur {
                for (e, b) in ecrivains.iter_mut().zip(bandes.iter_mut()) {
                    e.bande(b).map_err(|e| e.to_string())?;
                    b.clear();
                }
            }
        }
        for e in ecrivains {
            e.finir().map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = resultat {
        for t in &tmp {
            let _ = std::fs::remove_file(t);
        }
        return Err(e);
    }
    for (t, p) in tmp.iter().zip(plaques) {
        std::fs::rename(t, p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Aperçu d'une page : plaques PNG filtrées ; le composite d'origine est gardé dans `avant`
/// (vue scindée avant/après) et le composite filtré l'approche : chaque pixel de l'épreuve
/// écran est multiplié, par canal R, V, B, par le rapport entre les couleurs « naïves »
/// (sans profil) des plaques filtrées et d'origine. L'épreuve du support reste ainsi la base ;
/// seul l'effet du filtre est estimé.
pub fn filtrer_apercu(plaques: &[PathBuf; 4], composite: &Path, avant: &Path, f: &Filtre) -> Result<(), String> {
    let lire = |p: &Path| image::open(p).map(|i| i.to_luma8()).map_err(|e| format!("{} : {e}", p.display()));
    let mut images = plaques.iter().map(|p| lire(p)).collect::<Result<Vec<_>, _>>()?;
    let mut rvb = image::open(composite).map_err(|e| format!("{} : {e}", composite.display()))?.to_rgb8();
    let dims = rvb.dimensions();
    if images.iter().any(|i| i.dimensions() != dims) {
        return Err("aperçu : plaques et composite de tailles différentes".into());
    }
    rvb.save(avant).map_err(|e| e.to_string())?;
    let calcul = f.calcul();
    let naif = |e: [u8; 4]| -> [f32; 3] {
        let blanc = |c: u8| (255 - c) as f32 * (255 - e[3]) as f32 / 255.0;
        [blanc(e[0]), blanc(e[1]), blanc(e[2])]
    };
    for (n, px) in rvb.pixels_mut().enumerate() {
        let (x, y) = (n as u32 % dims.0, n as u32 / dims.0);
        let o: [u8; 4] = std::array::from_fn(|i| 255 - images[i].get_pixel(x, y)[0]);
        let s = calcul.pixel(o);
        let (a, b) = (naif(o), naif(s));
        for c in 0..3 {
            px[c] = if a[c] < 1.0 { b[c] } else { (px[c] as f32 * b[c] / a[c]).min(255.0) }.round() as u8;
        }
        for i in 0..4 {
            images[i].put_pixel(x, y, image::Luma([255 - s[i]]));
        }
    }
    for (i, p) in images.iter().zip(plaques) {
        i.save(p).map_err(|e| e.to_string())?;
    }
    rvb.save(composite).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courbes_luminosite_saturation_remplacement() {
        // Identité : aucun pixel ne bouge.
        let c = Filtre::default().calcul();
        for v in [0u8, 1, 51, 128, 254, 255] {
            assert_eq!(c.pixel([v, v / 2, 255 - v, 7]), [v, v / 2, 255 - v, 7]);
        }
        // Niveaux sur le cyan : 100 % → 80 % ; 50 % → 40 %.
        let niveaux = Filtre { courbes: [vec![[100.0, 80.0]], vec![], vec![], vec![]], ..Default::default() }.calcul();
        assert_eq!(niveaux.pixel([255, 255, 0, 0]), [204, 255, 0, 0]);
        assert_eq!(niveaux.pixel([128, 0, 0, 0])[0], 102);
        // Courbe à trois points : 50 % → 70 %.
        let courbe = Filtre { courbes: [vec![], vec![[0.0, 0.0], [50.0, 70.0], [100.0, 100.0]], vec![], vec![]], ..Default::default() }.calcul();
        assert_eq!(courbe.pixel([0, 128, 0, 0])[1], 179);
        // Luminosité +20 : 80 % de l'encre ; le support reste vierge.
        let clair = Filtre { luminosite: 20.0, ..Default::default() }.calcul();
        assert_eq!(clair.pixel([255, 100, 0, 50]), [204, 80, 0, 40]);
        // Saturation −100 : C, M, J ramenés à leur part grise ; le noir ne bouge pas.
        let gris = Filtre { saturation: -100.0, ..Default::default() }.calcul();
        assert_eq!(gris.pixel([200, 100, 50, 30]), [50, 50, 50, 30]);
        let vif = Filtre { saturation: 50.0, ..Default::default() }.calcul();
        assert_eq!(vif.pixel([150, 100, 50, 0]), [200, 125, 50, 0]);
        // Remplacement avec plage de 2 % (5 unités) ; prioritaire sur le reste du filtre.
        let r = Filtre { luminosite: 50.0, remplacements: vec![Remplacement { de: [100.0, 50.0, 0.0, 0.0], vers: [0.0, 100.0, 100.0, 0.0], tolerance: 2.0, actif: true }], ..Default::default() }.calcul();
        assert_eq!(r.pixel([255, 128, 0, 0]), [0, 255, 255, 0]);
        assert_eq!(r.pixel([250, 123, 5, 5]), [0, 255, 255, 0], "dans la plage");
        assert_eq!(r.pixel([249, 128, 0, 0]), [125, 64, 0, 0], "hors plage : luminosité seule");
        let inactif = Filtre { remplacements: vec![Remplacement { actif: false, ..Remplacement { de: [0.0; 4], vers: [100.0; 4], tolerance: 0.0, actif: true } }], ..Default::default() };
        assert!(inactif.neutre());
    }

    #[test]
    fn bornes() {
        assert!(Filtre::default().verifier().is_ok());
        let mauvais = [
            Filtre { luminosite: 101.0, ..Default::default() },
            Filtre { courbes: [vec![[50.0, 10.0], [40.0, 20.0]], vec![], vec![], vec![]], ..Default::default() },
            Filtre { courbes: [vec![[50.0, 120.0]], vec![], vec![], vec![]], ..Default::default() },
            Filtre { remplacements: vec![Remplacement { de: [0.0; 4], vers: [0.0; 4], tolerance: 60.0, actif: true }], ..Default::default() },
        ];
        for f in mauvais {
            assert_eq!(f.verifier(), Err("filtre_invalide"), "{f:?}");
        }
    }
}
