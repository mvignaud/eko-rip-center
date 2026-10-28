//! Épreuve TIFF après RIP (roadmap 9.13) : les plaques rendues d'une page réunies en un
//! seul TIFF, en valeurs machine (CMJN et tons directs tels qu'ils partent à la machine)
//! ou colorimétrique (RVB, converti par Ghostscript avec le profil du support). Les tons
//! directs y sont gardés en canaux (relus comme tons directs par le RIP et par Photoshop),
//! aplatis dans la couleur, ou supprimés.
//!
//! Un plafond de taille réduit la définition (moyenne de k × k points) ; la résolution
//! écrite garde la taille physique exacte de la page, pour qu'on puisse certifier les
//! cotes d'un lé sur l'épreuve. Tout se fait en flux, ligne par ligne.

use crate::blanc::LecteurTiff;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Couleur {
    /// CMJN (et tons directs) en valeurs machine, avec le profil du support si le travail
    /// en a un.
    #[default]
    Machine,
    /// RVB converti par le profil du support (intention colorimétrique relative).
    Icc,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tons {
    /// Un canal par ton direct.
    #[default]
    Canaux,
    /// Mêlés à la couleur d'après leur couleur d'affichage.
    Aplatis,
    Supprimes,
}

/// Plafond le plus haut : un TIFF classique (non BigTIFF), lisible partout, s'arrête à
/// 4 Go.
pub const MAX_MO: u64 = 3500;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Options {
    #[serde(default)]
    pub couleur: Couleur,
    #[serde(default)]
    pub tons: Tons,
    /// Plafond de taille de l'épreuve non compressée, en Mo.
    #[serde(default = "max_defaut")]
    pub max_mo: u64,
}

fn max_defaut() -> u64 {
    1024
}

impl Default for Options {
    fn default() -> Self {
        Options { couleur: Couleur::Machine, tons: Tons::Canaux, max_mo: max_defaut() }
    }
}

impl Options {
    pub fn verifier(&self) -> Result<(), &'static str> {
        if (1..=MAX_MO).contains(&self.max_mo) { Ok(()) } else { Err("epreuve_invalide") }
    }
}

/// Plaques d'une page : C, M, J, N (absente = sans encre), puis les tons directs.
pub struct Source {
    pub cmjn: [PathBuf; 4],
    pub tons: Vec<(String, PathBuf)>,
    /// Résolution des plaques.
    pub dpi: f64,
    /// Profil ICC du support (CMJN), s'il y en a un.
    pub profil: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
pub struct Epreuve {
    pub largeur: u32,
    pub hauteur: u32,
    /// k : un point de l'épreuve pour k × k points des plaques.
    pub reduction: u32,
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
    /// Canaux du TIFF, dans l'ordre.
    pub canaux: Vec<String>,
}

/// Couleur d'affichage CMJN (0..=1) d'un ton direct aplati.
fn affichage(nom: &str) -> [f64; 4] {
    match nom.to_lowercase().as_str() {
        "white" | "blanc" | "wh" => [0.0, 0.0, 0.0, 0.1],
        "silver" | "metallic" | "argent" | "ms" => [0.0, 0.0, 0.0, 0.35],
        "varnish" | "vernis" | "gloss" => [0.0, 0.0, 0.0, 0.05],
        _ => [0.0, 0.0, 0.0, 0.5],
    }
}

/// Lecture réduite des plaques : lignes moyennées par k × k, en encre (0 = rien,
/// 255 = pleine encre).
struct Reducteur {
    lecteurs: Vec<Option<LecteurTiff>>,
    k: usize,
    largeur: usize,
    hauteur: usize,
    l2: usize,
    y2: usize,
}

impl Reducteur {
    fn ouvrir(chemins: &[&Path], k: usize) -> Result<Self, String> {
        let lecteurs: Vec<Option<LecteurTiff>> = chemins.iter().map(|c| c.exists().then(|| LecteurTiff::ouvrir(c)).transpose()).collect::<Result<_, _>>()?;
        let mut tailles = lecteurs.iter().flatten().map(|l| (l.largeur, l.hauteur));
        let (largeur, hauteur) = tailles.next().ok_or("aucune plaque")?;
        if tailles.any(|t| t != (largeur, hauteur)) {
            return Err("plaques de tailles différentes".into());
        }
        Ok(Reducteur { lecteurs, k, largeur, hauteur, l2: largeur.div_ceil(k), y2: 0 })
    }

    /// Ligne réduite suivante, une par plaque.
    fn ligne(&mut self) -> Result<Vec<Vec<u8>>, String> {
        let k = self.k;
        let rangs = k.min(self.hauteur - self.y2 * k);
        let mut sommes = vec![vec![0u32; self.l2]; self.lecteurs.len()];
        for _ in 0..rangs {
            for (lecteur, somme) in self.lecteurs.iter_mut().zip(&mut sommes) {
                let Some(l) = lecteur else { continue };
                for (x, v) in l.ligne()?.into_iter().enumerate() {
                    somme[x / k] += 255 - v as u32;
                }
            }
        }
        self.y2 += 1;
        Ok(sommes
            .into_iter()
            .map(|s| {
                s.into_iter()
                    .enumerate()
                    .map(|(x2, v)| {
                        let n = (k.min(self.largeur - x2 * k) * rangs) as u32;
                        ((v + n / 2) / n) as u8
                    })
                    .collect()
            })
            .collect())
    }
}

/// Plus petit k qui fait tenir l'épreuve (l / k × h / k × `canaux` octets) sous le plafond.
fn reduction(largeur: usize, hauteur: usize, canaux: usize, max_mo: u64) -> usize {
    (1..).find(|&k| (largeur.div_ceil(k) * hauteur.div_ceil(k) * canaux) as u64 <= max_mo << 20).unwrap()
}

/// Ressources Photoshop des canaux en plus : noms (1006) et affichage en ton direct (1007),
/// comme les écrit Photoshop pour un canal de ton direct.
fn ressources_photoshop(tons: &[String]) -> Vec<u8> {
    let bloc = |id: u16, d: &[u8]| {
        let mut o = b"8BIM".to_vec();
        o.extend_from_slice(&id.to_be_bytes());
        o.extend_from_slice(&[0, 0]);
        o.extend_from_slice(&(d.len() as u32).to_be_bytes());
        o.extend_from_slice(d);
        if d.len() % 2 == 1 {
            o.push(0);
        }
        o
    };
    let mut noms = vec![];
    let mut affiche = vec![];
    for t in tons {
        let n: Vec<u8> = t.bytes().take(255).collect();
        noms.push(n.len() as u8);
        noms.extend_from_slice(&n);
        // Espace CMJN (2), valeurs stockées inversées, opacité 100, nature 2 (ton direct).
        affiche.extend_from_slice(&2u16.to_be_bytes());
        for c in affichage(t) {
            affiche.extend_from_slice(&((65535.0 * (1.0 - c)).round() as u16).to_be_bytes());
        }
        affiche.extend_from_slice(&100u16.to_be_bytes());
        affiche.extend_from_slice(&[2, 0]);
    }
    [bloc(1006, &noms), bloc(1007, &affiche)].concat()
}

/// Description du TIFF à écrire.
struct Tiff<'a> {
    largeur: u32,
    hauteur: u32,
    /// Pixels par pouce, horizontal et vertical (taille physique exacte).
    ppi: (f64, f64),
    rvb: bool,
    tons: &'a [String],
    icc: Option<Vec<u8>>,
}

/// Écrit le TIFF (LZW, bandes d'environ 1 Mo) ; `ligne` remplit une ligne de pixels
/// entrelacés (couleur puis tons).
fn ecrire_tiff(sortie: &Path, t: &Tiff, mut ligne: impl FnMut(&mut Vec<u8>) -> Result<(), String>) -> Result<(), String> {
    use tiff::tags::Tag;
    let err = |e: tiff::TiffError| e.to_string();
    let canaux = (if t.rvb { 3 } else { 4 }) + t.tons.len();
    let octets_ligne = t.largeur as usize * canaux;
    let par_bande = ((1 << 20) / octets_ligne.max(1)).clamp(1, t.hauteur as usize) as u32;
    let f = std::io::BufWriter::new(std::fs::File::create(sortie).map_err(|e| e.to_string())?);
    let mut enc = tiff::encoder::TiffEncoder::new(f).map_err(err)?;
    let mut dir = enc.image_directory().map_err(err)?;
    let (mut positions, mut longueurs): (Vec<u32>, Vec<u32>) = (vec![], vec![]);
    let mut bande: Vec<u8> = Vec::with_capacity(par_bande as usize * octets_ligne);
    let mut l = Vec::with_capacity(octets_ligne);
    for y in 0..t.hauteur {
        l.clear();
        ligne(&mut l)?;
        if l.len() != octets_ligne {
            return Err(format!("ligne {y} : {} octets au lieu de {octets_ligne}", l.len()));
        }
        bande.extend_from_slice(&l);
        if (y + 1) % par_bande == 0 || y + 1 == t.hauteur {
            let lzw = weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8).encode(&bande).map_err(|e| e.to_string())?;
            let p = dir.write_data(&lzw[..]).map_err(err)?;
            positions.push(u32::try_from(p).map_err(|_| "épreuve de plus de 4 Go".to_string())?);
            longueurs.push(lzw.len() as u32);
            bande.clear();
        }
    }
    dir.write_tag(Tag::ImageWidth, t.largeur).map_err(err)?;
    dir.write_tag(Tag::ImageLength, t.hauteur).map_err(err)?;
    dir.write_tag(Tag::BitsPerSample, &vec![8u16; canaux][..]).map_err(err)?;
    dir.write_tag(Tag::Compression, 5u16).map_err(err)?;
    dir.write_tag(Tag::PhotometricInterpretation, if t.rvb { 2u16 } else { 5u16 }).map_err(err)?;
    dir.write_tag(Tag::SamplesPerPixel, canaux as u16).map_err(err)?;
    dir.write_tag(Tag::RowsPerStrip, par_bande).map_err(err)?;
    dir.write_tag(Tag::StripOffsets, &positions[..]).map_err(err)?;
    dir.write_tag(Tag::StripByteCounts, &longueurs[..]).map_err(err)?;
    dir.write_tag(Tag::PlanarConfiguration, 1u16).map_err(err)?;
    // Résolution en millièmes de point par pouce : la taille physique reste exacte à
    // 1/1000 de point près.
    let r = |ppi: f64| tiff::encoder::Rational { n: (ppi * 1000.0).round() as u32, d: 1000 };
    dir.write_tag(Tag::XResolution, r(t.ppi.0)).map_err(err)?;
    dir.write_tag(Tag::YResolution, r(t.ppi.1)).map_err(err)?;
    dir.write_tag(Tag::ResolutionUnit, 2u16).map_err(err)?;
    if !t.rvb {
        // InkSet 1 : CMJN.
        dir.write_tag(Tag::Unknown(332), 1u16).map_err(err)?;
    }
    if !t.tons.is_empty() {
        dir.write_tag(Tag::ExtraSamples, &vec![0u16; t.tons.len()][..]).map_err(err)?;
        dir.write_tag(Tag::Unknown(34377), &ressources_photoshop(t.tons)[..]).map_err(err)?;
    }
    if let Some(icc) = &t.icc {
        dir.write_tag(Tag::Unknown(34675), &icc[..]).map_err(err)?;
    }
    dir.finish().map_err(err)
}

/// Lecture ligne par ligne d'un TIFF RVB 8 bits (sortie de Ghostscript).
struct LecteurRvb {
    d: tiff::decoder::Decoder<std::io::BufReader<std::fs::File>>,
    largeur: usize,
    bande: u32,
    lignes: std::collections::VecDeque<Vec<u8>>,
}

impl LecteurRvb {
    fn ouvrir(chemin: &Path) -> Result<Self, String> {
        let f = std::io::BufReader::new(std::fs::File::open(chemin).map_err(|e| e.to_string())?);
        let mut d = tiff::decoder::Decoder::new(f).map_err(|e| e.to_string())?.with_limits(tiff::decoder::Limits::unlimited());
        if d.colortype().map_err(|e| e.to_string())? != tiff::ColorType::RGB(8) {
            return Err("épreuve RVB : TIFF 8 bits RVB attendu de Ghostscript".into());
        }
        let largeur = d.dimensions().map_err(|e| e.to_string())?.0 as usize;
        Ok(LecteurRvb { d, largeur, bande: 0, lignes: Default::default() })
    }

    /// Ligne suivante ; `None` après la dernière.
    fn ligne(&mut self) -> Result<Option<Vec<u8>>, String> {
        while self.lignes.is_empty() {
            if self.bande >= self.d.strip_count().map_err(|e| e.to_string())? {
                return Ok(None);
            }
            let (_, rangs) = self.d.chunk_data_dimensions(self.bande);
            let v = match self.d.read_chunk(self.bande).map_err(|e| e.to_string())? {
                tiff::decoder::DecodingResult::U8(v) => v,
                _ => return Err("épreuve RVB : bande non 8 bits".into()),
            };
            self.bande += 1;
            for l in v.chunks_exact(self.largeur * 3).take(rangs as usize) {
                self.lignes.push_back(l.to_vec());
            }
        }
        Ok(self.lignes.pop_front())
    }
}

/// Épreuve colorimétrique : PDF de l'image CMJN réduite (profil du support, tons aplatis
/// en surimpression si demandé), rendu en RVB par Ghostscript. Rend le TIFF de Ghostscript.
#[allow(clippy::too_many_arguments)]
async fn icc_par_ghostscript(gs: &str, chemins: &[PathBuf], tons: &[(String, PathBuf)], k: usize, (l2, h2): (usize, usize), ppi: f64, icc: Option<Vec<u8>>, aplatir: bool, tmp: &Path, delai: Duration) -> Result<PathBuf, String> {
    let pdf = tmp.join("epreuve.pdf");
    let affiches: Vec<crate::images::TonDirect> = if aplatir { tons.iter().map(|(n, _)| (n.clone(), affichage(n))).collect() } else { vec![] };
    let (chemins, pdf2) = (chemins.to_vec(), pdf.clone());
    tokio::task::spawn_blocking(move || {
        crate::images::pdf_cmjn(&pdf2, l2 as u32, h2 as u32, ppi, icc, affiches, |w, flux| {
            let refs: Vec<&Path> = chemins.iter().map(PathBuf::as_path).collect();
            let mut r = Reducteur::ouvrir(&refs, k)?;
            let mut l = Vec::with_capacity(l2 * 4);
            for _ in 0..h2 {
                let p = r.ligne()?;
                l.clear();
                match flux {
                    None => (0..l2).for_each(|x| l.extend_from_slice(&[p[0][x], p[1][x], p[2][x], p[3][x]])),
                    Some(t) => l.extend_from_slice(&p[4 + t]),
                }
                w.write_all(&l).map_err(|e| e.to_string())?;
            }
            Ok(())
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let rvb = tmp.join("rvb.tif");
    // Colorimétrique relative ; la dernière résolution donnée l'emporte sur celle du rendu.
    let options = ["-dRenderIntent=1".to_string(), "-sCompression=lzw".to_string(), format!("-r{ppi:.4}")];
    crate::render::ghostscript(gs, &pdf, "tiff24nc", ppi.round() as u32, &options, &rvb, delai).await?;
    Ok(rvb)
}

/// Produit l'épreuve de `src` dans `sortie`. `gs` et `delai` servent à l'épreuve
/// colorimétrique.
pub async fn produire(gs: &str, src: &Source, o: Options, sortie: &Path, delai: Duration) -> Result<Epreuve, String> {
    let chemins: Vec<PathBuf> = src.cmjn.iter().cloned().chain(src.tons.iter().map(|(_, c)| c.clone())).collect();
    let (largeur, hauteur) = {
        let r = Reducteur::ouvrir(&chemins.iter().map(PathBuf::as_path).collect::<Vec<_>>(), 1)?;
        (r.largeur, r.hauteur)
    };
    let rvb = o.couleur == Couleur::Icc;
    let noms: Vec<String> = if o.tons == Tons::Canaux { src.tons.iter().map(|(n, _)| n.clone()).collect() } else { vec![] };
    let k = reduction(largeur, hauteur, (if rvb { 3 } else { 4 }) + noms.len(), o.max_mo);
    let (l2, h2) = (largeur.div_ceil(k), hauteur.div_ceil(k));
    // Taille physique de la page entière, gardée exacte quelle que soit la réduction.
    let (pouces_l, pouces_h) = (largeur as f64 / src.dpi, hauteur as f64 / src.dpi);
    let ppi = (l2 as f64 / pouces_l, h2 as f64 / pouces_h);
    let icc = src.profil.as_ref().map(std::fs::read).transpose().map_err(|e| e.to_string())?;
    let canaux: Vec<String> = if rvb { ["Red", "Green", "Blue"].map(String::from).to_vec() } else { crate::epson::PLAQUES_CMJN.map(String::from).to_vec() }.into_iter().chain(noms.iter().cloned()).collect();
    let sortie = sortie.to_path_buf();
    let mode = o.tons;
    let affiches: Vec<[f64; 4]> = src.tons.iter().map(|(n, _)| affichage(n)).collect();
    if !rvb {
        tokio::task::spawn_blocking(move || {
            let mut r = Reducteur::ouvrir(&chemins.iter().map(PathBuf::as_path).collect::<Vec<_>>(), k)?;
            let t = Tiff { largeur: l2 as u32, hauteur: h2 as u32, ppi, rvb: false, tons: &noms, icc };
            ecrire_tiff(&sortie, &t, |l| {
                let p = r.ligne()?;
                for x in 0..l2 {
                    let mut c = [p[0][x], p[1][x], p[2][x], p[3][x]];
                    if mode == Tons::Aplatis {
                        // Ton posé en surimpression : chaque encre de sa couleur d'affichage
                        // s'ajoute à celle de dessous.
                        for (s, a) in p[4..].iter().zip(&affiches) {
                            let s = s[x] as f64 / 255.0;
                            for (v, a) in c.iter_mut().zip(a) {
                                *v = (255.0 * (1.0 - (1.0 - *v as f64 / 255.0) * (1.0 - s * a))).round() as u8;
                            }
                        }
                    }
                    l.extend_from_slice(&c);
                    if mode == Tons::Canaux {
                        l.extend(p[4..].iter().map(|s| s[x]));
                    }
                }
                Ok(())
            })
        })
        .await
        .map_err(|e| e.to_string())??;
    } else {
        let tmp = sortie.with_extension("tmp");
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let r = async {
            let rendu = icc_par_ghostscript(gs, &chemins, &src.tons, k, (l2, h2), ppi.0, icc, mode == Tons::Aplatis, &tmp, delai).await?;
            tokio::task::spawn_blocking(move || {
                let mut g = LecteurRvb::ouvrir(&rendu)?;
                // Les tons gardés en canaux sont relus des plaques, à la même réduction.
                let mut r = if noms.is_empty() { None } else { Some(Reducteur::ouvrir(&chemins.iter().map(PathBuf::as_path).collect::<Vec<_>>(), k)?) };
                let t = Tiff { largeur: l2 as u32, hauteur: h2 as u32, ppi, rvb: true, tons: &noms, icc: None };
                ecrire_tiff(&sortie, &t, |l| {
                    // Ghostscript peut arrondir la page d'un point : on coupe ou on complète
                    // en blanc.
                    let mut c = g.ligne()?.unwrap_or_default();
                    c.resize(l2 * 3, 255);
                    let tons = match r.as_mut() {
                        Some(r) => r.ligne()?.split_off(4),
                        None => vec![],
                    };
                    for x in 0..l2 {
                        l.extend_from_slice(&c[x * 3..x * 3 + 3]);
                        // Canaux en plus d'un TIFF RVB : stockés inversés (0 = pleine encre),
                        // comme Photoshop.
                        l.extend(tons.iter().map(|s| 255 - s[x]));
                    }
                    Ok(())
                })
            })
            .await
            .map_err(|e| e.to_string())?
        }
        .await;
        let _ = std::fs::remove_dir_all(&tmp);
        r?;
    }
    Ok(Epreuve { largeur: l2 as u32, hauteur: h2 as u32, reduction: k as u32, largeur_mm: pouces_l * 25.4, hauteur_mm: pouces_h * 25.4, canaux })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier() -> PathBuf {
        let d = std::env::temp_dir().join(format!("eko-rip-epreuve-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Page de 250 × 120 points à 100 dpi (63,5 × 30,48 mm) : cyan plein à gauche, blanc
    /// plein à droite, rien ailleurs.
    fn source(d: &Path) -> Source {
        let plaque = |nom: &str, encre: &dyn Fn(u32, u32) -> bool| {
            let c = d.join(format!("{nom}.tif"));
            image::GrayImage::from_fn(250, 120, |x, y| image::Luma([if encre(x, y) { 0 } else { 255 }])).save(&c).unwrap();
            c
        };
        let rien = |_: u32, _: u32| false;
        Source {
            cmjn: [plaque("Cyan", &|x, _| x < 100), plaque("Magenta", &rien), plaque("Yellow", &rien), plaque("Black", &rien)],
            tons: vec![("White".into(), plaque("White", &|x, _| x >= 150))],
            dpi: 100.0,
            profil: None,
        }
    }

    /// Dimensions physiques (mm) et nombre de canaux d'un TIFF.
    fn lire(tif: &Path) -> ((f64, f64), u16, Vec<u8>) {
        use tiff::decoder::ifd::Value;
        let mut d = tiff::decoder::Decoder::new(std::fs::File::open(tif).unwrap()).unwrap();
        let (l, h) = d.dimensions().unwrap();
        let mut res = |t| match d.get_tag(t).unwrap() {
            Value::Rational(n, dd) => n as f64 / dd as f64,
            v => panic!("{v:?}"),
        };
        let (rx, ry) = (res(tiff::tags::Tag::XResolution), res(tiff::tags::Tag::YResolution));
        let n = d.get_tag_u32(tiff::tags::Tag::SamplesPerPixel).unwrap() as u16;
        // Bandes LZW décodées à la main : le décodeur ne rend pas les canaux en plus d'un CMJN.
        let o = std::fs::read(tif).unwrap();
        let (pos, lon) = (d.get_tag_u64_vec(tiff::tags::Tag::StripOffsets).unwrap(), d.get_tag_u64_vec(tiff::tags::Tag::StripByteCounts).unwrap());
        let px: Vec<u8> = pos.iter().zip(&lon).flat_map(|(&p, &l)| weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8).decode(&o[p as usize..(p + l) as usize]).unwrap()).collect();
        ((l as f64 / rx * 25.4, h as f64 / ry * 25.4), n, px)
    }

    /// Roadmap 9.13 : valeurs machine, tons en canaux : taille physique ± 0,5 mm, canal blanc
    /// présent, relu comme ton direct `White` par le RIP lui-même.
    #[tokio::test]
    async fn machine_canaux_cotes_et_canal_blanc() {
        let d = dossier();
        let tif = d.join("e.tif");
        let e = produire("gs", &source(&d), Options::default(), &tif, Duration::from_secs(60)).await.unwrap();
        assert_eq!((e.largeur, e.hauteur, e.reduction), (250, 120, 1));
        assert_eq!(e.canaux, ["Cyan", "Magenta", "Yellow", "Black", "White"]);
        let ((l, h), n, px) = lire(&tif);
        assert!((l - 63.5).abs() <= 0.5 && (h - 30.48).abs() <= 0.5, "{l} × {h} mm");
        assert_eq!(n, 5);
        assert_eq!(&px[..5], &[255, 0, 0, 0, 0], "cyan plein, pas de blanc à gauche");
        assert_eq!(&px[(200 * 5)..(200 * 5 + 5)], &[0, 0, 0, 0, 255], "blanc plein à droite");
        let r = crate::images::vers_pdf(&tif, &d.join("e.pdf"), crate::images::Genre::Tiff, None).unwrap();
        assert_eq!(r.tons_directs, ["White"]);
        assert!((r.largeur_mm - 63.5).abs() <= 0.5);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Plafond de taille : la définition baisse, la taille physique reste ± 0,5 mm ; tons
    /// aplatis (le blanc grise un peu le noir) ou supprimés : 4 canaux.
    #[tokio::test]
    async fn plafond_aplatis_supprimes() {
        let d = dossier();
        let src = source(&d);
        let (a, s) = (d.join("a.tif"), d.join("s.tif"));
        // 250 × 120 × 4 octets ≈ 117 Ko : 1 Mo passe, on force la réduction par le calcul.
        assert_eq!(reduction(250, 120, 4, 1), 1);
        assert_eq!(reduction(25_000, 12_000, 4, 100), 4);
        let e = produire("gs", &src, Options { tons: Tons::Aplatis, ..Default::default() }, &a, Duration::from_secs(60)).await.unwrap();
        assert_eq!(e.canaux.len(), 4);
        let (_, n, px) = lire(&a);
        assert_eq!(n, 4);
        assert_eq!(&px[200 * 4..200 * 4 + 4], &[0, 0, 0, 25], "blanc aplati : 10 % de noir");
        produire("gs", &src, Options { tons: Tons::Supprimes, ..Default::default() }, &s, Duration::from_secs(60)).await.unwrap();
        assert_eq!(&lire(&s).2[200 * 4..200 * 4 + 4], &[0, 0, 0, 0]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Réduction k = 3 sur une page de 250 points : 84 points, cotes gardées.
    #[test]
    fn reduction_garde_les_cotes() {
        let d = dossier();
        let src = source(&d);
        let chemins: Vec<&Path> = src.cmjn.iter().map(PathBuf::as_path).collect();
        let mut r = Reducteur::ouvrir(&chemins, 3).unwrap();
        assert_eq!((r.l2, r.hauteur.div_ceil(3)), (84, 40));
        let l = r.ligne().unwrap();
        assert_eq!((l[0][0], l[0][33], l[0][83]), (255, 85, 0), "cyan : plein, bord moyenné, rien");
        let tif = d.join("k.tif");
        let t = Tiff { largeur: 84, hauteur: 40, ppi: (84.0 / 2.5, 40.0 / 1.2), rvb: false, tons: &[], icc: None };
        ecrire_tiff(&tif, &t, |l| {
            l.extend(std::iter::repeat_n(0, 84 * 4));
            Ok(())
        })
        .unwrap();
        let ((l, h), _, _) = lire(&tif);
        assert!((l - 63.5).abs() <= 0.5 && (h - 30.48).abs() <= 0.5, "{l} × {h} mm");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Épreuve colorimétrique : RVB par Ghostscript, cyan à gauche, papier à droite, blanc
    /// gardé en canal (inversé, comme Photoshop).
    #[tokio::test]
    async fn icc_rvb_et_canal() {
        let d = dossier();
        let tif = d.join("i.tif");
        let e = produire("gs", &source(&d), Options { couleur: Couleur::Icc, ..Default::default() }, &tif, Duration::from_secs(60)).await.unwrap();
        assert_eq!(e.canaux, ["Red", "Green", "Blue", "White"]);
        let ((l, h), n, px) = lire(&tif);
        assert_eq!(n, 4);
        assert!((l - 63.5).abs() <= 0.5 && (h - 30.48).abs() <= 0.5, "{l} × {h} mm");
        let p = |x: usize| &px[(60 * 250 + x) * 4..(60 * 250 + x) * 4 + 4];
        assert!(p(10)[0] < 80 && p(10)[2] > 180, "cyan : {:?}", p(10));
        assert!(p(120)[..3].iter().all(|&v| v > 240), "papier : {:?}", p(120));
        assert_eq!((p(10)[3], p(200)[3]), (255, 0), "canal blanc : vide à gauche, plein à droite");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
