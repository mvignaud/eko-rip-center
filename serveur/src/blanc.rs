//! Blanc automatique : plaque `White` générée après le rendu, sous l'encre de l'image
//! (sous-couche) ou sur toute la page (aplat), avec une rétraction (le blanc recule de
//! quelques dixièmes de millimètre pour ne pas dépasser de la couleur).
//!
//! Les plaques de production d'une bâche font plusieurs Go : elles sont lues et écrites
//! en flux, bande par bande ; la rétraction (érosion carrée de rayon r) ne garde en
//! mémoire que 2r + 1 lignes.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

pub const PLAQUE: &str = "White";

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Aucun,
    /// Sous l'encre de l'image (toute plaque d'encre au-delà du seuil).
    SousCouche,
    /// Toute la page (support transparent ou coloré).
    Aplat,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Blanc {
    #[serde(default)]
    pub mode: Mode,
    /// Recul du blanc depuis le bord de l'encre, en mm.
    #[serde(default = "retraction_defaut")]
    pub retraction_mm: f64,
    /// Encre minimale (en %) qui reçoit du blanc dessous.
    #[serde(default = "seuil_defaut")]
    pub seuil: f64,
}

fn retraction_defaut() -> f64 {
    0.2
}

fn seuil_defaut() -> f64 {
    5.0
}

impl Default for Blanc {
    fn default() -> Self {
        Blanc { mode: Mode::Aucun, retraction_mm: retraction_defaut(), seuil: seuil_defaut() }
    }
}

impl Blanc {
    pub fn verifier(&self) -> Result<(), &'static str> {
        if (0.0..=5.0).contains(&self.retraction_mm) && (1.0..=100.0).contains(&self.seuil) { Ok(()) } else { Err("blanc_invalide") }
    }

    fn rayon(&self, dpi: f64) -> usize {
        (self.retraction_mm / 25.4 * dpi).round() as usize
    }

    /// Valeur de plaque (0 = pleine encre) à partir de laquelle un pixel est « encré ».
    fn limite(&self) -> u8 {
        (255.0 * (1.0 - self.seuil / 100.0)).floor() as u8
    }
}

/// Érosion carrée de rayon `r` d'un masque binaire fourni ligne par ligne : un pixel reste
/// encré si tout son carré (2r + 1)² l'est ; hors de l'image compte comme non encré.
/// Les lignes sortent avec `r` lignes de retard ; `finir` vide la fin.
struct Erosion {
    r: usize,
    largeur: usize,
    /// Lignes déjà érodées horizontalement (vrai = encré), au plus 2r + 1.
    fenetre: VecDeque<Vec<bool>>,
    /// Par colonne, nombre de lignes non encrées dans la fenêtre.
    trous: Vec<u32>,
    lignes_recues: usize,
    lignes_sorties: usize,
    hauteur: usize,
}

impl Erosion {
    fn new(r: usize, largeur: usize, hauteur: usize) -> Self {
        // La fenêtre démarre pleine de lignes « hors image » (non encrées) au-dessus.
        let fenetre = (0..r).map(|_| vec![false; largeur]).collect();
        Erosion { r, largeur, fenetre, trous: vec![r as u32; largeur], lignes_recues: 0, lignes_sorties: 0, hauteur }
    }

    fn horizontale(&self, masque: &[bool]) -> Vec<bool> {
        if self.r == 0 {
            return masque.to_vec();
        }
        // Somme préfixe des trous ; hors de la ligne = trou.
        let mut prefixe = Vec::with_capacity(masque.len() + 1);
        prefixe.push(0u32);
        for &m in masque {
            prefixe.push(prefixe.last().unwrap() + u32::from(!m));
        }
        (0..self.largeur)
            .map(|x| x >= self.r && x + self.r < self.largeur && prefixe[x + self.r + 1] == prefixe[x - self.r])
            .collect()
    }

    /// Ajoute une ligne de masque ; rend la ligne érodée prête, s'il y en a une.
    fn pousser(&mut self, masque: &[bool], sortie: &mut impl FnMut(&[bool]) -> std::io::Result<()>) -> std::io::Result<()> {
        let h = self.horizontale(masque);
        self.lignes_recues += 1;
        self.entrer(h, sortie)
    }

    fn entrer(&mut self, h: Vec<bool>, sortie: &mut impl FnMut(&[bool]) -> std::io::Result<()>) -> std::io::Result<()> {
        for (t, &v) in self.trous.iter_mut().zip(&h) {
            *t += u32::from(!v);
        }
        self.fenetre.push_back(h);
        if self.fenetre.len() == 2 * self.r + 1 {
            if self.lignes_sorties < self.hauteur {
                let ligne: Vec<bool> = self.trous.iter().map(|&t| t == 0).collect();
                sortie(&ligne)?;
                self.lignes_sorties += 1;
            }
            let vieille = self.fenetre.pop_front().unwrap();
            for (t, &v) in self.trous.iter_mut().zip(&vieille) {
                *t -= u32::from(!v);
            }
        }
        Ok(())
    }

    /// Complète par `r` lignes non encrées sous l'image et sort les dernières lignes.
    fn finir(&mut self, sortie: &mut impl FnMut(&[bool]) -> std::io::Result<()>) -> std::io::Result<()> {
        while self.lignes_sorties < self.hauteur {
            self.entrer(vec![false; self.largeur], sortie)?;
        }
        Ok(())
    }
}

/// Lecture d'une plaque TIFF 8 bits ligne par ligne, en valeurs « 0 = pleine encre ».
pub(crate) struct LecteurTiff {
    decodeur: tiff::decoder::Decoder<BufReader<std::fs::File>>,
    pub(crate) largeur: usize,
    pub(crate) hauteur: usize,
    bande: u32,
    lignes: VecDeque<Vec<u8>>,
    inverser: bool,
}

impl LecteurTiff {
    pub(crate) fn ouvrir(chemin: &Path) -> Result<Self, String> {
        let f = std::fs::File::open(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
        let mut decodeur = tiff::decoder::Decoder::new(BufReader::new(f)).map_err(|e| format!("{} : {e}", chemin.display()))?;
        if decodeur.colortype().map_err(|e| e.to_string())? != tiff::ColorType::Gray(8) {
            return Err(format!("{} : plaque attendue en niveaux de gris 8 bits", chemin.display()));
        }
        let (l, h) = decodeur.dimensions().map_err(|e| e.to_string())?;
        // WhiteIsZero (0) : la valeur brute est l'inverse de la luminance.
        let photometrie = decodeur.get_tag_u32(tiff::tags::Tag::PhotometricInterpretation).unwrap_or(1);
        Ok(LecteurTiff { decodeur, largeur: l as usize, hauteur: h as usize, bande: 0, lignes: VecDeque::new(), inverser: photometrie == 0 })
    }

    pub(crate) fn ligne(&mut self) -> Result<Vec<u8>, String> {
        while self.lignes.is_empty() {
            let (_, rangs) = self.decodeur.chunk_data_dimensions(self.bande);
            let donnees = match self.decodeur.read_chunk(self.bande).map_err(|e| e.to_string())? {
                tiff::decoder::DecodingResult::U8(v) => v,
                _ => return Err("plaque : bande non 8 bits".into()),
            };
            self.bande += 1;
            for rang in donnees.chunks_exact(self.largeur).take(rangs as usize) {
                self.lignes.push_back(if self.inverser { rang.iter().map(|v| 255 - v).collect() } else { rang.to_vec() });
            }
        }
        Ok(self.lignes.pop_front().unwrap())
    }
}

/// Génère la plaque `White` d'une page dans `sortie`, à partir des plaques `sources`
/// (encres de l'image) et du blanc déjà dessiné dans le PDF (`existant`, gardé : le blanc
/// final est leur union). Les plaques sont des TIFF 8 bits rendus à `dpi`.
pub fn generer_tiff(sources: &[PathBuf], existant: Option<&Path>, sortie: &Path, b: &Blanc, dpi: f64) -> Result<(), String> {
    let mut lecteurs = sources.iter().map(|p| LecteurTiff::ouvrir(p)).collect::<Result<Vec<_>, _>>()?;
    let mut deja = existant.map(LecteurTiff::ouvrir).transpose()?;
    let (largeur, hauteur) = match (lecteurs.first(), &deja) {
        (Some(l), _) => (l.largeur, l.hauteur),
        (None, Some(d)) => (d.largeur, d.hauteur),
        (None, None) => return Err("aucune plaque pour le blanc".into()),
    };
    if lecteurs.iter().chain(deja.as_ref()).any(|l| (l.largeur, l.hauteur) != (largeur, hauteur)) {
        return Err("plaques de tailles différentes".into());
    }

    let tmp = sortie.with_extension("tif.tmp");
    let mut tiff = EcrivainTiff::creer(&tmp, largeur as u32, hauteur as u32, dpi.round() as u32).map_err(|e| e.to_string())?;
    let mut bande: Vec<u8> = Vec::with_capacity(RANGS_PAR_BANDE * largeur);
    let mut lignes_ecrites = 0usize;
    // Le blanc déjà dessiné, lu au même rythme que les lignes érodées qui sortent.
    let mut ecrire = |ligne: &[bool]| -> std::io::Result<()> {
        let existante = match deja.as_mut() {
            Some(d) => Some(d.ligne().map_err(std::io::Error::other)?),
            None => None,
        };
        for (x, &encre) in ligne.iter().enumerate() {
            let v = if encre { 0 } else { 255 };
            bande.push(existante.as_ref().map_or(v, |e| e[x].min(v)));
        }
        lignes_ecrites += 1;
        if bande.len() == RANGS_PAR_BANDE * largeur || lignes_ecrites == hauteur {
            tiff.bande(&bande)?;
            bande.clear();
        }
        Ok(())
    };

    let limite = b.limite();
    let mut erosion = Erosion::new(b.rayon(dpi), largeur, hauteur);
    let mut masque = vec![false; largeur];
    let resultat = (|| -> Result<(), String> {
        for _ in 0..hauteur {
            match b.mode {
                Mode::Aplat => masque.fill(true),
                _ => {
                    masque.fill(false);
                    for l in lecteurs.iter_mut() {
                        for (m, v) in masque.iter_mut().zip(l.ligne()?) {
                            *m |= v <= limite;
                        }
                    }
                }
            }
            erosion.pousser(&masque, &mut ecrire).map_err(|e| e.to_string())?;
        }
        erosion.finir(&mut ecrire).map_err(|e| e.to_string())
    })();
    let resultat = resultat.and_then(|()| tiff.finir().map_err(|e| e.to_string()));
    if let Err(e) = resultat {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, sortie).map_err(|e| e.to_string())
}

pub(crate) const RANGS_PAR_BANDE: usize = 64;

/// TIFF de base, gris 8 bits (0 = noir = pleine encre), bandes LZW écrites l'une après
/// l'autre. Écrit à la main : l'encodeur de la crate `tiff` (0.11) n'applique pas la
/// compression annoncée quand on lui donne les bandes une à une.
pub(crate) struct EcrivainTiff {
    f: BufWriter<std::fs::File>,
    position: u64,
    largeur: u32,
    hauteur: u32,
    dpi: u32,
    decalages: Vec<u32>,
    tailles: Vec<u32>,
}

impl EcrivainTiff {
    pub(crate) fn creer(chemin: &Path, largeur: u32, hauteur: u32, dpi: u32) -> std::io::Result<Self> {
        use std::io::Write;
        let mut f = BufWriter::new(std::fs::File::create(chemin)?);
        // En-tête petit-boutiste ; le décalage du répertoire est écrit à la fin.
        f.write_all(b"II*\0\0\0\0\0")?;
        Ok(EcrivainTiff { f, position: 8, largeur, hauteur, dpi, decalages: vec![], tailles: vec![] })
    }

    fn ecrire(&mut self, o: &[u8]) -> std::io::Result<u32> {
        use std::io::Write;
        let ici = u32::try_from(self.position).map_err(|_| std::io::Error::other("plaque de blanc de plus de 4 Go"))?;
        self.f.write_all(o)?;
        self.position += o.len() as u64;
        Ok(ici)
    }

    pub(crate) fn bande(&mut self, pixels: &[u8]) -> std::io::Result<()> {
        let lzw = weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8).encode(pixels).map_err(std::io::Error::other)?;
        let ici = self.ecrire(&lzw)?;
        self.decalages.push(ici);
        self.tailles.push(lzw.len() as u32);
        Ok(())
    }

    pub(crate) fn finir(mut self) -> std::io::Result<()> {
        use std::io::{Seek, Write};
        let octets = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
        // Valeurs de plus de 4 octets : hors du répertoire.
        let (dec, tai) = (self.decalages.clone(), self.tailles.clone());
        let (n, une) = (dec.len() as u32, dec.len() == 1);
        let ou_dec = if une { dec[0] } else { self.ecrire(&octets(&dec))? };
        let ou_tai = if une { tai[0] } else { self.ecrire(&octets(&tai))? };
        let ou_res = self.ecrire(&octets(&[self.dpi, 1]))?;
        if self.position % 2 == 1 {
            self.ecrire(&[0])?;
        }
        // (étiquette, type, nombre, valeur) ; types : 3 = SHORT, 4 = LONG, 5 = RATIONAL.
        let entrees: [(u16, u16, u32, u32); 12] = [
            (256, 4, 1, self.largeur),
            (257, 4, 1, self.hauteur),
            (258, 3, 1, 8),
            (259, 3, 1, 5), // LZW
            (262, 3, 1, 1), // BlackIsZero
            (273, 4, n, ou_dec),
            (277, 3, 1, 1),
            (278, 4, 1, RANGS_PAR_BANDE as u32),
            (279, 4, n, ou_tai),
            (282, 5, 1, ou_res),
            (283, 5, 1, ou_res),
            (296, 3, 1, 2), // pouce
        ];
        let mut repertoire = (entrees.len() as u16).to_le_bytes().to_vec();
        for (etiquette, genre, nombre, valeur) in entrees {
            repertoire.extend(etiquette.to_le_bytes());
            repertoire.extend(genre.to_le_bytes());
            repertoire.extend(nombre.to_le_bytes());
            // Un SHORT seul se range dans les 2 premiers octets du champ.
            repertoire.extend(if genre == 3 { (valeur as u16 as u32).to_le_bytes() } else { valeur.to_le_bytes() });
        }
        repertoire.extend(0u32.to_le_bytes());
        let ou_rep = self.ecrire(&repertoire)?;
        self.f.seek(std::io::SeekFrom::Start(4))?;
        self.f.write_all(&ou_rep.to_le_bytes())?;
        self.f.flush()
    }
}

/// Même calcul sur les aperçus PNG (petits : lus en entier).
pub fn generer_apercu(sources: &[PathBuf], existant: Option<&Path>, sortie: &Path, b: &Blanc, dpi: f64) -> Result<(), String> {
    let lire = |p: &Path| image::open(p).map(|i| i.to_luma8()).map_err(|e| format!("{} : {e}", p.display()));
    let plaques = sources.iter().map(|p| lire(p)).collect::<Result<Vec<_>, _>>()?;
    let deja = existant.map(lire).transpose()?;
    let (l, h) = plaques.first().or(deja.as_ref()).map(|i| (i.width() as usize, i.height() as usize)).ok_or("aucune plaque pour le blanc")?;
    let limite = b.limite();
    let mut pixels = Vec::with_capacity(l * h);
    let mut y = 0usize;
    let mut ecrire = |ligne: &[bool]| -> std::io::Result<()> {
        for (x, &encre) in ligne.iter().enumerate() {
            let v = if encre { 0 } else { 255 };
            pixels.push(deja.as_ref().map_or(v, |d| d.get_pixel(x as u32, y as u32)[0].min(v)));
        }
        y += 1;
        Ok(())
    };
    let mut erosion = Erosion::new(b.rayon(dpi), l, h);
    for ry in 0..h {
        let masque: Vec<bool> = (0..l).map(|x| b.mode == Mode::Aplat || plaques.iter().any(|p| p.get_pixel(x as u32, ry as u32)[0] <= limite)).collect();
        erosion.pousser(&masque, &mut ecrire).map_err(|e| e.to_string())?;
    }
    erosion.finir(&mut ecrire).map_err(|e| e.to_string())?;
    image::GrayImage::from_raw(l as u32, h as u32, pixels).ok_or("aperçu du blanc")?.save(sortie).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eroder(masque: &[Vec<bool>], r: usize) -> Vec<Vec<bool>> {
        let (l, h) = (masque[0].len(), masque.len());
        let mut sortie = vec![];
        let mut e = Erosion::new(r, l, h);
        let mut ecrire = |ligne: &[bool]| {
            sortie.push(ligne.to_vec());
            Ok(())
        };
        for ligne in masque {
            e.pousser(ligne, &mut ecrire).unwrap();
        }
        e.finir(&mut ecrire).unwrap();
        sortie
    }

    /// Référence naïve : tout le carré doit être encré, hors image = non encré.
    fn reference(masque: &[Vec<bool>], r: usize) -> Vec<Vec<bool>> {
        let (l, h) = (masque[0].len() as i64, masque.len() as i64);
        let r = r as i64;
        (0..h)
            .map(|y| (0..l).map(|x| (-r..=r).all(|dy| (-r..=r).all(|dx| { let (xx, yy) = (x + dx, y + dy); xx >= 0 && yy >= 0 && xx < l && yy < h && masque[yy as usize][xx as usize] }))).collect())
            .collect()
    }

    #[test]
    fn erosion_identique_a_la_reference() {
        // Motif pseudo-aléatoire, plus un grand bloc plein.
        let mut graine = 7u32;
        let masque: Vec<Vec<bool>> = (0..23)
            .map(|y| (0..31).map(|x| { graine = graine.wrapping_mul(1_103_515_245).wrapping_add(12_345); (5..20).contains(&y) && (4..26).contains(&x) || graine >> 28 > 3 }).collect())
            .collect();
        for r in [0, 1, 2, 3, 12] {
            assert_eq!(eroder(&masque, r), reference(&masque, r), "rayon {r}");
        }
    }

    #[test]
    fn reglages_bornes() {
        assert!(Blanc::default().verifier().is_ok());
        assert_eq!(Blanc { retraction_mm: 6.0, ..Default::default() }.verifier(), Err("blanc_invalide"));
        assert_eq!(Blanc { seuil: 0.0, ..Default::default() }.verifier(), Err("blanc_invalide"));
        // 0,2 mm à 720 dpi ≈ 5,7 px → 6 ; seuil 5 % → valeur ≤ 242.
        assert_eq!((Blanc::default().rayon(720.0), Blanc::default().limite()), (6, 242));
    }

    /// Vrai rendu Ghostscript : carré cyan de 1 pouce au centre d'une page de 2 pouces,
    /// à 100 dpi. Sous-couche avec 1 mm de rétraction (4 px) : le blanc couvre le carré
    /// moins 4 px de chaque côté, en TIFF de production comme en aperçu.
    #[test]
    fn sous_couche_d_un_vrai_rendu() {
        let d = std::env::temp_dir().join(format!("eko-rip-blanc-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("p.ps"), "%!PS\n<< /PageSize [144 144] >> setpagedevice\n1 0 0 0 setcmykcolor 36 36 72 72 rectfill\nshowpage\n").unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r100"])
            .arg(format!("-sOutputFile={}", d.join("r.tif").display()))
            .arg(d.join("p.ps"))
            .status()
            .unwrap();
        assert!(st.success());
        let sources: Vec<PathBuf> = ["Cyan", "Magenta", "Yellow", "Black"].iter().map(|n| d.join(format!("r({n}).tif"))).collect();
        let b = Blanc { mode: Mode::SousCouche, retraction_mm: 1.0, seuil: 5.0 };
        generer_tiff(&sources, None, &d.join("w.tif"), &b, 100.0).unwrap();
        let w = image::open(d.join("w.tif")).unwrap().to_luma8();
        let cyan = image::open(&sources[0]).unwrap().to_luma8();
        assert_eq!(w.dimensions(), (200, 200));
        assert_eq!(cyan.get_pixel(100, 100)[0], 0, "convention : 0 = pleine encre");
        // Carré cyan : x et y de 50 à 149 ; blanc attendu de 54 à 145.
        let blanc = |x, y| w.get_pixel(x, y)[0] == 0;
        assert!(blanc(54, 54) && blanc(145, 145) && blanc(100, 100));
        assert!(!blanc(53, 100) && !blanc(146, 100) && !blanc(100, 53) && !blanc(100, 146), "rétraction de 4 px");
        assert!(!blanc(10, 10), "pas de blanc hors de l'encre");

        // Union avec un blanc déjà dessiné : le coin (10, 10) devient blanc.
        let mut existant = image::GrayImage::from_pixel(200, 200, image::Luma([255]));
        existant.put_pixel(10, 10, image::Luma([0]));
        existant.save(d.join("e.tif")).unwrap();
        generer_tiff(&sources, Some(&d.join("e.tif")), &d.join("w2.tif"), &b, 100.0).unwrap();
        let w2 = image::open(d.join("w2.tif")).unwrap().to_luma8();
        assert_eq!((w2.get_pixel(10, 10)[0], w2.get_pixel(100, 100)[0], w2.get_pixel(30, 30)[0]), (0, 0, 255));

        // Aperçu PNG : même résultat.
        for (i, s) in sources.iter().enumerate() {
            image::open(s).unwrap().to_luma8().save(d.join(format!("a{i}.png"))).unwrap();
        }
        let apercus: Vec<PathBuf> = (0..4).map(|i| d.join(format!("a{i}.png"))).collect();
        generer_apercu(&apercus, None, &d.join("wa.png"), &b, 100.0).unwrap();
        assert_eq!(image::open(d.join("wa.png")).unwrap().to_luma8(), w, "aperçu identique à la production");

        // Aplat : toute la page, moins la rétraction au bord.
        generer_tiff(&sources, None, &d.join("w3.tif"), &Blanc { mode: Mode::Aplat, ..b }, 100.0).unwrap();
        let w3 = image::open(d.join("w3.tif")).unwrap().to_luma8();
        assert_eq!((w3.get_pixel(4, 4)[0], w3.get_pixel(3, 100)[0], w3.get_pixel(195, 195)[0]), (0, 255, 0));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
