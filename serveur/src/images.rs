//! Images reçues (roadmap 9.2) : TIFF, PNG et JPEG deviennent un PDF d'une page à leur taille
//! réelle (résolution du fichier), puis suivent la chaîne d'un PDF (rendu, plaques, profils,
//! blanc, découpe). Le PDF est écrit en flux, bande par bande : une affiche de plusieurs Go
//! ne passe jamais entière en mémoire.
//!
//! Gardés : CMJN, RVB ou gris, 8 ou 16 bits (1 bit en gris) ; la transparence devient un
//! masque (rien n'est posé où l'image est transparente) ; le profil ICC du fichier. Le JPEG
//! est repris tel quel (filtre DCT), sans décodage. Refusés avec un message : TIFF à palette,
//! YCbCr, Lab ou à plans séparés, PNG entrelacé. Canaux en plus (tons directs d'un TIFF) :
//! ignorés pour l'instant et signalés.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::Path;

/// Résolution supposée quand le fichier n'en donne pas (1 pixel = 1 point PDF), signalée.
pub const PPI_PAR_DEFAUT: f64 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Genre {
    Tiff,
    Png,
    Jpeg,
}

/// Genre d'image d'après les premiers octets.
pub fn genre(debut: &[u8]) -> Option<Genre> {
    if debut.starts_with(b"II*\0") || debut.starts_with(b"MM\0*") || debut.starts_with(b"II+\0") || debut.starts_with(b"MM\0+") {
        Some(Genre::Tiff)
    } else if debut.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Genre::Png)
    } else if debut.starts_with(b"\xFF\xD8\xFF") {
        Some(Genre::Jpeg)
    } else {
        None
    }
}

/// Ce que la conversion a dû supposer ou laisser de côté, pour les alertes du travail.
#[derive(Debug, Default, PartialEq)]
pub struct Remarques {
    /// Pas de résolution dans le fichier : `PPI_PAR_DEFAUT` supposée.
    pub sans_resolution: bool,
    /// Canaux en plus ignorés (tons directs d'un TIFF).
    pub canaux_ignores: usize,
    /// Taille de la page produite, en mm.
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
}

/// Refus : (code, détail).
pub type Refus = (&'static str, String);

/// Convertit l'image `src` en PDF d'une page dans `dst`.
pub fn vers_pdf(src: &Path, dst: &Path, g: Genre) -> Result<Remarques, Refus> {
    let r = match g {
        Genre::Tiff => tiff_vers_pdf(src, dst),
        Genre::Png => png_vers_pdf(src, dst),
        Genre::Jpeg => jpeg_vers_pdf(src, dst),
    };
    if r.is_err() {
        let _ = std::fs::remove_file(dst);
    }
    r
}

fn illisible(e: impl std::fmt::Display) -> Refus {
    ("image_illisible", e.to_string())
}

fn interne(e: std::io::Error) -> Refus {
    ("erreur_interne", e.to_string())
}

// --- Écriture du PDF ---

/// Description de l'image à écrire.
struct Image {
    largeur: u32,
    hauteur: u32,
    /// Pixels par pouce, horizontal et vertical.
    ppi: (f64, f64),
    /// `DeviceGray`, `DeviceRGB` ou `DeviceCMYK`.
    espace: &'static str,
    composantes: u32,
    bits: u32,
    icc: Option<Vec<u8>>,
    /// Entrée `/Decode` (CMJN inversé d'un JPEG Adobe).
    decode: Option<&'static str>,
    /// `Some(longueur)` : données déjà compressées en DCT (JPEG repris tel quel).
    dct: Option<u64>,
    alpha: bool,
}

/// Écrivain qui compte ses octets (positions de la table xref).
struct Compteur {
    w: BufWriter<File>,
    n: u64,
}

impl Write for Compteur {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let k = self.w.write(b)?;
        self.n += k as u64;
        Ok(k)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.w.flush()
    }
}

/// Écrit le PDF : `couleur` écrit le flux de l'image, `masque` celui de la transparence
/// (appelé seulement si `img.alpha`). Les longueurs sont connues d'avance.
fn ecrire_pdf(dst: &Path, img: &Image, couleur: impl FnOnce(&mut dyn Write) -> Result<(), Refus>, masque: impl FnOnce(&mut dyn Write) -> Result<(), Refus>) -> Result<(), Refus> {
    let mut f = Compteur { w: BufWriter::new(File::create(dst).map_err(interne)?), n: 0 };
    let (lpt, hpt) = (img.largeur as f64 * 72.0 / img.ppi.0, img.hauteur as f64 * 72.0 / img.ppi.1);
    let ligne = |c: u32| (img.largeur as u64 * c as u64 * img.bits as u64).div_ceil(8);
    let longueur = img.dct.unwrap_or(ligne(img.composantes) * img.hauteur as u64);
    let mut positions: Vec<(usize, u64)> = vec![];
    f.write_all(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n").map_err(interne)?;
    let contenu = format!("q {lpt:.4} 0 0 {hpt:.4} 0 0 cm /Im Do Q");
    for (n, corps) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {lpt:.4} {hpt:.4}] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>")),
        (4, format!("<< /Length {} >>\nstream\n{contenu}\nendstream", contenu.len())),
    ] {
        positions.push((n, f.n));
        write!(f, "{n} 0 obj\n{corps}\nendobj\n").map_err(interne)?;
    }
    let espace = if img.icc.is_some() { "[/ICCBased 7 0 R]".to_string() } else { format!("/{}", img.espace) };
    let mut dico = format!("<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {espace} /BitsPerComponent {} /Length {longueur}", img.largeur, img.hauteur, img.bits);
    if img.dct.is_some() {
        dico += " /Filter /DCTDecode";
    }
    if let Some(d) = img.decode {
        dico += &format!(" /Decode {d}");
    }
    if img.alpha {
        dico += " /SMask 6 0 R";
    }
    positions.push((5, f.n));
    write!(f, "5 0 obj\n{dico} >>\nstream\n").map_err(interne)?;
    let avant = f.n;
    couleur(&mut f)?;
    if f.n - avant != longueur {
        return Err(illisible(format!("image tronquée : {} octets au lieu de {longueur}", f.n - avant)));
    }
    f.write_all(b"\nendstream\nendobj\n").map_err(interne)?;
    if img.alpha {
        let la = ligne(1) * img.hauteur as u64;
        positions.push((6, f.n));
        write!(f, "6 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent {} /Length {la} >>\nstream\n", img.largeur, img.hauteur, img.bits).map_err(interne)?;
        let avant = f.n;
        masque(&mut f)?;
        if f.n - avant != la {
            return Err(illisible("masque de transparence tronqué"));
        }
        f.write_all(b"\nendstream\nendobj\n").map_err(interne)?;
    }
    if let Some(icc) = &img.icc {
        positions.push((7, f.n));
        write!(f, "7 0 obj\n<< /N {} /Alternate /{} /Length {} >>\nstream\n", img.composantes, img.espace, icc.len()).map_err(interne)?;
        f.write_all(icc).map_err(interne)?;
        f.write_all(b"\nendstream\nendobj\n").map_err(interne)?;
    }
    let taille = positions.iter().map(|(n, _)| *n).max().unwrap_or(0) + 1;
    let xref = f.n;
    write!(f, "xref\n0 {taille}\n0000000000 65535 f \n").map_err(interne)?;
    for n in 1..taille {
        match positions.iter().find(|(m, _)| *m == n) {
            Some((_, p)) => writeln!(f, "{p:010} 00000 n "),
            None => writeln!(f, "0000000000 65535 f "),
        }
        .map_err(interne)?;
    }
    write!(f, "trailer\n<< /Size {taille} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").map_err(interne)?;
    f.flush().map_err(interne)
}

/// Une ligne de pixels entrelacés (`canaux` échantillons de `octets` octets) → la couleur
/// (les `couleur` premiers) dans `c` et l'alpha (`alpha`, s'il y en a un) dans `a`.
fn separer(ligne: &[u8], canaux: usize, couleur: usize, alpha: Option<usize>, octets: usize, c: &mut Vec<u8>, a: &mut Vec<u8>) {
    c.clear();
    a.clear();
    for px in ligne.chunks_exact(canaux * octets) {
        c.extend_from_slice(&px[..couleur * octets]);
        if let Some(i) = alpha {
            a.extend_from_slice(&px[i * octets..(i + 1) * octets]);
        }
    }
}

// --- TIFF ---

struct InfosTiff {
    largeur: u32,
    hauteur: u32,
    espace: &'static str,
    couleur: usize,
    canaux: usize,
    alpha: Option<usize>,
    bits: u32,
    ppi: Option<(f64, f64)>,
    icc: Option<Vec<u8>>,
}

type Decodeur = tiff::decoder::Decoder<BufReader<File>>;

fn ouvrir_tiff(src: &Path) -> Result<Decodeur, Refus> {
    let f = File::open(src).map_err(illisible)?;
    Ok(tiff::decoder::Decoder::new(BufReader::new(f)).map_err(illisible)?.with_limits(tiff::decoder::Limits::unlimited()))
}

fn infos_tiff(d: &mut Decodeur) -> Result<InfosTiff, Refus> {
    use tiff::ColorType as C;
    use tiff::decoder::ifd::Value;
    use tiff::tags::Tag;
    let (largeur, hauteur) = d.dimensions().map_err(illisible)?;
    if d.find_tag_unsigned::<u16>(Tag::PlanarConfiguration).ok().flatten().unwrap_or(1) != 1 {
        return Err(("image_non_prise_en_charge", "TIFF à plans séparés : l'enregistrer avec les pixels entrelacés".into()));
    }
    let canaux = d.find_tag_unsigned::<u16>(Tag::SamplesPerPixel).ok().flatten().unwrap_or(1) as usize;
    let ct = d.colortype().map_err(illisible)?;
    let bits = ct.bit_depth() as u32;
    let (espace, couleur, alpha) = match ct {
        C::Gray(_) | C::Multiband { .. } => ("DeviceGray", 1, None),
        C::GrayA(_) => ("DeviceGray", 1, Some(1)),
        C::RGB(_) => ("DeviceRGB", 3, None),
        C::RGBA(_) => ("DeviceRGB", 3, Some(3)),
        C::CMYK(_) => ("DeviceCMYK", 4, None),
        C::CMYKA(_) => ("DeviceCMYK", 4, Some(4)),
        autre => return Err(("image_non_prise_en_charge", format!("TIFF {autre:?} : l'enregistrer en CMJN, RVB ou niveaux de gris"))),
    };
    if !(bits == 8 || bits == 16 || (bits == 1 && canaux == 1)) {
        return Err(("image_non_prise_en_charge", format!("TIFF {bits} bits : 8 ou 16 bits attendus")));
    }
    if canaux < couleur + usize::from(alpha.is_some()) {
        return Err(illisible("nombre de canaux incohérent"));
    }
    let reel = |v: Value| match v {
        Value::Rational(n, dd) if dd > 0 => Some(n as f64 / dd as f64),
        Value::Double(x) => Some(x),
        Value::Float(x) => Some(x as f64),
        _ => None,
    };
    let unite = match d.find_tag_unsigned::<u16>(Tag::ResolutionUnit).ok().flatten().unwrap_or(2) {
        3 => 2.54,
        2 => 1.0,
        _ => 0.0,
    };
    let x = d.find_tag(Tag::XResolution).ok().flatten().and_then(reel);
    let y = d.find_tag(Tag::YResolution).ok().flatten().and_then(reel);
    let ppi = match (x, y) {
        (Some(x), y) if x > 0.0 && unite > 0.0 => Some((x * unite, y.filter(|y| *y > 0.0).unwrap_or(x) * unite)),
        // Photoshop peut écrire « 1 sans unité » dans l'en-tête TIFF et garder la vraie
        // résolution dans ses propres données (relevé le 25/10 sur une affiche de l'atelier).
        _ => d.find_tag(Tag::Unknown(34377)).ok().flatten().and_then(|v| v.into_u8_vec().ok()).and_then(|o| resolution_photoshop(&o)),
    };
    let icc = d.find_tag(Tag::Unknown(34675)).ok().flatten().and_then(|v| v.into_u8_vec().ok()).filter(|p| p.len() > 128);
    Ok(InfosTiff { largeur, hauteur, espace, couleur, canaux, alpha, bits, ppi, icc })
}

/// Résolution des ressources d'image Photoshop (bloc `8BIM` n° 1005, ResolutionInfo) :
/// virgule fixe 16.16, unité 1 = pixels par pouce, 2 = pixels par cm.
fn resolution_photoshop(o: &[u8]) -> Option<(f64, f64)> {
    let mut i = 0;
    while i + 12 <= o.len() {
        if &o[i..i + 4] != b"8BIM" {
            i += 1;
            continue;
        }
        let id = u16::from_be_bytes([o[i + 4], o[i + 5]]);
        let nom = 1 + *o.get(i + 6)? as usize;
        let j = i + 6 + nom + nom % 2;
        let taille = u32::from_be_bytes(o.get(j..j + 4)?.try_into().ok()?) as usize;
        let donnees = o.get(j + 4..j + 4 + taille)?;
        if id == 1005 && donnees.len() >= 16 {
            let fixe = |k: usize| u32::from_be_bytes(donnees[k..k + 4].try_into().unwrap()) as f64 / 65536.0;
            let unite = |k: usize| if u16::from_be_bytes([donnees[k], donnees[k + 1]]) == 2 { 2.54 } else { 1.0 };
            let (h, v) = (fixe(0) * unite(4), fixe(8) * unite(12));
            return (h > 0.0).then_some((h, if v > 0.0 { v } else { h }));
        }
        i = j + 4 + taille + taille % 2;
    }
    None
}

/// Lit le TIFF ligne par ligne (bandes ou tuiles), en octets gros-boutistes.
fn lignes_tiff(src: &Path, mut sur_ligne: impl FnMut(&[u8]) -> Result<(), Refus>) -> Result<(), Refus> {
    let mut d = ouvrir_tiff(src)?;
    let i = infos_tiff(&mut d)?;
    let (cl, ch) = d.chunk_dimensions();
    let (cl, ch) = (cl.max(1), ch.max(1));
    let travers = i.largeur.div_ceil(cl);
    let octets = (i.bits as usize).div_ceil(8);
    let largeur_ligne = |l: u32| if i.bits == 1 { (l as usize).div_ceil(8) } else { l as usize * i.canaux * octets };
    if i.bits == 1 && travers > 1 {
        return Err(("image_non_prise_en_charge", "TIFF 1 bit en tuiles".into()));
    }
    let mut ligne = vec![0u8; largeur_ligne(i.largeur)];
    for rang in 0..i.hauteur.div_ceil(ch) {
        let mut bandes = vec![];
        for k in 0..travers {
            let n = rang * travers + k;
            let (l, h) = d.chunk_data_dimensions(n);
            let donnees = match d.read_chunk(n).map_err(illisible)? {
                tiff::decoder::DecodingResult::U8(v) => v,
                tiff::decoder::DecodingResult::U16(v) => v.iter().flat_map(|x| x.to_be_bytes()).collect(),
                _ => return Err(("image_non_prise_en_charge", "TIFF à échantillons flottants ou signés".into())),
            };
            bandes.push((l, h, donnees));
        }
        for y in 0..bandes[0].1 as usize {
            let mut x = 0;
            for (l, _, donnees) in &bandes {
                let lg = largeur_ligne(*l);
                let morceau = donnees.get(y * lg..(y + 1) * lg).ok_or_else(|| illisible("bande trop courte"))?;
                ligne[x..x + lg].copy_from_slice(morceau);
                x += lg;
            }
            sur_ligne(&ligne)?;
        }
    }
    Ok(())
}

fn tiff_vers_pdf(src: &Path, dst: &Path) -> Result<Remarques, Refus> {
    let i = infos_tiff(&mut ouvrir_tiff(src)?)?;
    let ignores = i.canaux - i.couleur - usize::from(i.alpha.is_some());
    let ppi = i.ppi.unwrap_or((PPI_PAR_DEFAUT, PPI_PAR_DEFAUT));
    let octets = (i.bits as usize).div_ceil(8);
    // Un alpha partout opaque ne devient pas un masque : la transparence ralentit beaucoup
    // le rendu (Ghostscript passe alors par son moteur de transparence).
    let mut opaque = true;
    if let Some(a) = i.alpha {
        lignes_tiff(src, |l| {
            opaque &= l.chunks_exact(i.canaux * octets).all(|px| px[a * octets..(a + 1) * octets].iter().all(|&o| o == 0xFF));
            Ok(())
        })?;
    }
    let i = InfosTiff { alpha: i.alpha.filter(|_| !opaque), ..i };
    let img = Image { largeur: i.largeur, hauteur: i.hauteur, ppi, espace: i.espace, composantes: i.couleur as u32, bits: i.bits, icc: i.icc.clone(), decode: None, dct: None, alpha: i.alpha.is_some() };
    let passe = |w: &mut dyn Write, garder_alpha: bool| -> Result<(), Refus> {
        let (mut c, mut a) = (vec![], vec![]);
        lignes_tiff(src, |l| {
            if i.bits == 1 || i.canaux == i.couleur {
                return w.write_all(l).map_err(interne);
            }
            separer(l, i.canaux, i.couleur, i.alpha, octets, &mut c, &mut a);
            w.write_all(if garder_alpha { &a } else { &c }).map_err(interne)
        })
    };
    ecrire_pdf(dst, &img, |w| passe(w, false), |w| passe(w, true))?;
    Ok(remarques(&img, i.ppi.is_none(), ignores))
}

fn remarques(img: &Image, sans_resolution: bool, canaux_ignores: usize) -> Remarques {
    Remarques { sans_resolution, canaux_ignores, largeur_mm: img.largeur as f64 * 25.4 / img.ppi.0, hauteur_mm: img.hauteur as f64 * 25.4 / img.ppi.1 }
}

// --- PNG ---

fn lecteur_png(src: &Path) -> Result<png::Reader<BufReader<File>>, Refus> {
    let mut d = png::Decoder::new(BufReader::new(File::open(src).map_err(illisible)?));
    // Palette et gris < 8 bits dépliés ; 16 bits gardés (gros-boutiste, comme le PDF).
    d.set_transformations(png::Transformations::EXPAND);
    d.read_info().map_err(illisible)
}

fn png_vers_pdf(src: &Path, dst: &Path) -> Result<Remarques, Refus> {
    let r = lecteur_png(src)?;
    let info = r.info();
    if info.interlaced {
        return Err(("image_non_prise_en_charge", "PNG entrelacé : l'enregistrer sans entrelacement".into()));
    }
    let (largeur, hauteur) = (info.width, info.height);
    let ppi = info.pixel_dims.filter(|p| p.unit == png::Unit::Meter && p.xppu > 0).map(|p| (p.xppu as f64 * 0.0254, p.yppu.max(1) as f64 * 0.0254));
    let icc = info.icc_profile.as_ref().map(|p| p.to_vec()).filter(|p| p.len() > 128);
    let (ct, prof) = r.output_color_type();
    let bits = if prof == png::BitDepth::Sixteen { 16 } else { 8 };
    let (espace, couleur, canaux) = match ct {
        png::ColorType::Grayscale => ("DeviceGray", 1, 1),
        png::ColorType::GrayscaleAlpha => ("DeviceGray", 1, 2),
        png::ColorType::Rgb => ("DeviceRGB", 3, 3),
        png::ColorType::Rgba => ("DeviceRGB", 3, 4),
        png::ColorType::Indexed => return Err(illisible("palette non dépliée")),
    };
    let octets = bits as usize / 8;
    drop(r);
    // Alpha partout opaque : pas de masque (voir le TIFF).
    let mut alpha = (canaux > couleur).then_some(couleur);
    if let Some(a) = alpha {
        let mut r = lecteur_png(src)?;
        let mut opaque = true;
        while let Some(l) = r.next_row().map_err(illisible)? {
            opaque &= l.data().chunks_exact(canaux * octets).all(|px| px[a * octets..(a + 1) * octets].iter().all(|&o| o == 0xFF));
            if !opaque {
                break;
            }
        }
        alpha = alpha.filter(|_| !opaque);
    }
    let img = Image { largeur, hauteur, ppi: ppi.unwrap_or((PPI_PAR_DEFAUT, PPI_PAR_DEFAUT)), espace, composantes: couleur as u32, bits, icc, decode: None, dct: None, alpha: alpha.is_some() };
    let passe = |w: &mut dyn Write, garder_alpha: bool| -> Result<(), Refus> {
        let mut r = lecteur_png(src)?;
        let (mut c, mut a) = (vec![], vec![]);
        while let Some(l) = r.next_row().map_err(illisible)? {
            separer(l.data(), canaux, couleur, alpha, octets, &mut c, &mut a);
            w.write_all(if garder_alpha { &a } else { &c }).map_err(interne)?;
        }
        Ok(())
    };
    ecrire_pdf(dst, &img, |w| passe(w, false), |w| passe(w, true))?;
    Ok(remarques(&img, ppi.is_none(), 0))
}

// --- JPEG ---

/// En-têtes d'un JPEG : largeur, hauteur, composantes (SOF), densité (JFIF), marqueur Adobe
/// (CMJN inversé), profil ICC (APP2, en morceaux numérotés).
type EntetesJpeg = (u32, u32, u32, Option<(f64, f64)>, bool, Option<Vec<u8>>);

fn entetes_jpeg(o: &[u8]) -> Result<EntetesJpeg, Refus> {
    let (mut dims, mut ppi, mut adobe, mut icc) = (None, None, false, Vec::<(u8, Vec<u8>)>::new());
    let mut i = 2;
    while i + 4 <= o.len() {
        if o[i] != 0xFF {
            return Err(illisible("marqueur JPEG attendu"));
        }
        let m = o[i + 1];
        if m == 0xFF {
            i += 1;
            continue;
        }
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) {
            i += 2;
            continue;
        }
        let l = u16::from_be_bytes([o[i + 2], o[i + 3]]) as usize;
        let seg = o.get(i + 4..i + 2 + l).ok_or_else(|| illisible("segment JPEG tronqué"))?;
        match m {
            0xE0 if seg.starts_with(b"JFIF\0") && seg.len() >= 12 => {
                let (u, x, y) = (seg[7], u16::from_be_bytes([seg[8], seg[9]]) as f64, u16::from_be_bytes([seg[10], seg[11]]) as f64);
                let f = match u {
                    1 => 1.0,
                    2 => 2.54,
                    _ => 0.0,
                };
                if f > 0.0 && x > 0.0 {
                    ppi = Some((x * f, if y > 0.0 { y } else { x } * f));
                }
            }
            0xE2 if seg.starts_with(b"ICC_PROFILE\0") && seg.len() > 14 => icc.push((seg[12], seg[14..].to_vec())),
            0xEE if seg.starts_with(b"Adobe") => adobe = true,
            0xC0..=0xCF if !matches!(m, 0xC4 | 0xC8 | 0xCC) && seg.len() >= 6 => {
                dims = Some((u16::from_be_bytes([seg[3], seg[4]]) as u32, u16::from_be_bytes([seg[1], seg[2]]) as u32, seg[5] as u32));
            }
            0xDA => break,
            _ => {}
        }
        i += 2 + l;
    }
    let (l, h, n) = dims.ok_or_else(|| illisible("dimensions JPEG absentes"))?;
    icc.sort_by_key(|(k, _)| *k);
    let icc: Vec<u8> = icc.into_iter().flat_map(|(_, p)| p).collect();
    Ok((l, h, n, ppi, adobe, (icc.len() > 128).then_some(icc)))
}

fn jpeg_vers_pdf(src: &Path, dst: &Path) -> Result<Remarques, Refus> {
    let mut f = File::open(src).map_err(illisible)?;
    let taille = f.metadata().map_err(illisible)?.len();
    // Les en-têtes sont au début ; 1 Mo couvre aussi un profil ICC en plusieurs morceaux.
    let mut debut = vec![0u8; taille.min(1 << 20) as usize];
    f.read_exact(&mut debut).map_err(illisible)?;
    let (largeur, hauteur, n, ppi, adobe, icc) = entetes_jpeg(&debut)?;
    let espace = match n {
        1 => "DeviceGray",
        3 => "DeviceRGB",
        4 => "DeviceCMYK",
        _ => return Err(("image_non_prise_en_charge", format!("JPEG à {n} composantes"))),
    };
    // Photoshop écrit le CMJN d'un JPEG inversé (marqueur Adobe) : le PDF le redresse.
    let decode = (n == 4 && adobe).then_some("[1 0 1 0 1 0 1 0]");
    let img = Image { largeur, hauteur, ppi: ppi.unwrap_or((PPI_PAR_DEFAUT, PPI_PAR_DEFAUT)), espace, composantes: n, bits: 8, icc, decode, dct: Some(taille), alpha: false };
    ecrire_pdf(
        dst,
        &img,
        |w| {
            f.rewind().map_err(illisible)?;
            std::io::copy(&mut f, w).map(|_| ()).map_err(interne)
        },
        |_| Ok(()),
    )?;
    Ok(remarques(&img, ppi.is_none(), 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("eko-rip-images-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Ghostscript lit le PDF produit : valeur (0 = pleine encre) de chaque plaque CMJN au
    /// point (x, y) en pixels à `dpi`.
    fn plaques(pdf: &Path, d: &Path, dpi: u32, x: u32, y: u32) -> Vec<u8> {
        let sortie = std::process::Command::new("gs").args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep"]).arg(format!("-r{dpi}")).arg(format!("-sOutputFile={}", d.join("p.tif").display())).arg(pdf).output().unwrap();
        assert!(sortie.status.success(), "{}", String::from_utf8_lossy(&sortie.stderr));
        ["Cyan", "Magenta", "Yellow", "Black"].iter().map(|n| image::open(d.join(format!("p({n}).tif"))).unwrap().to_luma8().get_pixel(x, y)[0]).collect()
    }

    #[test]
    fn reconnaissance() {
        assert_eq!(genre(b"II*\0abc"), Some(Genre::Tiff));
        assert_eq!(genre(b"MM\0*abc"), Some(Genre::Tiff));
        assert_eq!(genre(b"\x89PNG\r\n\x1a\nxx"), Some(Genre::Png));
        assert_eq!(genre(b"\xFF\xD8\xFF\xE0"), Some(Genre::Jpeg));
        assert_eq!(genre(b"%PDF-1.7"), None);
    }

    /// TIFF CMJN 8 bits, 300 ppi, LZW : taille réelle et cyan pur sur la seule plaque Cyan.
    #[test]
    fn tiff_cmjn_taille_reelle() {
        let d = dossier();
        let src = d.join("a.tif");
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap().with_compression(tiff::encoder::Compression::Lzw);
        let mut im = e.new_image::<tiff::encoder::colortype::CMYK8>(600, 300).unwrap();
        im.resolution(tiff::tags::ResolutionUnit::Inch, tiff::encoder::Rational { n: 300, d: 1 });
        im.write_data(&[255u8, 0, 0, 0].repeat(600 * 300)).unwrap();
        let pdf = d.join("a.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff).unwrap();
        assert_eq!((r.largeur_mm.round(), r.hauteur_mm.round(), r.sans_resolution), (51.0, 25.0, false));
        let t = crate::imbrication::taille_page(&std::fs::read(&pdf).unwrap(), 1).unwrap();
        assert_eq!((t.largeur.round(), t.hauteur.round()), (144.0, 72.0), "2 × 1 pouces");
        assert_eq!(plaques(&pdf, &d, 10, 5, 5), [0, 255, 255, 255]);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// TIFF RVB 16 bits avec alpha : le masque laisse le transparent sans encre.
    #[test]
    fn tiff_rvb16_alpha() {
        let d = dossier();
        let src = d.join("b.tif");
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        // Rouge opaque à gauche, rouge transparent à droite.
        let px: Vec<u16> = (0..20 * 10).flat_map(|i| if i % 20 < 10 { [65535, 0, 0, 65535] } else { [65535, 0, 0, 0] }).collect();
        e.write_image::<tiff::encoder::colortype::RGBA16>(20, 10, &px).unwrap();
        let pdf = d.join("b.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff).unwrap();
        assert!(r.sans_resolution, "pas de résolution : 72 ppi supposés");
        assert!(plaques(&pdf, &d, 72, 3, 5)[1] < 60, "rouge opaque : magenta posé");
        assert_eq!(plaques(&pdf, &d, 72, 16, 5), [255, 255, 255, 255], "transparent : rien");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Résolution Photoshop (8BIM 1005) quand l'en-tête TIFF est « 1 sans unité » ; un alpha
    /// partout opaque ne devient pas un masque.
    #[test]
    fn tiff_resolution_photoshop_et_alpha_opaque() {
        let d = dossier();
        let src = d.join("ps.tif");
        let mut res = b"8BIM\x03\xed\x00\x00\x00\x00\x00\x10".to_vec();
        res.extend_from_slice(&(300u32 << 16).to_be_bytes());
        res.extend_from_slice(&[0, 1, 0, 1]);
        res.extend_from_slice(&(300u32 << 16).to_be_bytes());
        res.extend_from_slice(&[0, 1, 0, 1]);
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        let mut im = e.new_image::<tiff::encoder::colortype::RGBA8>(600, 300).unwrap();
        im.resolution(tiff::tags::ResolutionUnit::None, tiff::encoder::Rational { n: 1, d: 1 });
        im.encoder().write_tag(tiff::tags::Tag::Unknown(34377), &res[..]).unwrap();
        im.write_data(&[255u8, 0, 0, 255].repeat(600 * 300)).unwrap();
        let pdf = d.join("ps.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff).unwrap();
        assert_eq!((r.largeur_mm.round(), r.hauteur_mm.round(), r.sans_resolution), (51.0, 25.0, false), "300 ppi de Photoshop");
        let o = std::fs::read(&pdf).unwrap();
        assert!(!o.windows(6).any(|w| w == b"/SMask"), "alpha opaque : pas de masque");
        assert!(plaques(&pdf, &d, 10, 5, 5)[1] < 60, "rouge : magenta posé");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// PNG RVBA 8 bits avec pHYs (300 ppi) : taille réelle, bleu en cyan.
    #[test]
    fn png_rvba() {
        let d = dossier();
        let src = d.join("c.png");
        let mut e = png::Encoder::new(BufWriter::new(File::create(&src).unwrap()), 300, 150);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.set_pixel_dims(Some(png::PixelDimensions { xppu: 11811, yppu: 11811, unit: png::Unit::Meter }));
        let mut w = e.write_header().unwrap();
        w.write_image_data(&[0u8, 0, 255, 255].repeat(300 * 150)).unwrap();
        w.finish().unwrap();
        let pdf = d.join("c.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Png).unwrap();
        assert_eq!((r.largeur_mm.round(), r.hauteur_mm.round()), (25.0, 13.0));
        let p = plaques(&pdf, &d, 10, 3, 2);
        assert!(p[0] < 80 && p[2] > 200, "bleu : cyan posé, pas de jaune : {p:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// JPEG repris tel quel : en-têtes lus, densité JFIF, Ghostscript le rend.
    #[test]
    fn jpeg_repris_tel_quel() {
        let d = dossier();
        let src = d.join("d.jpg");
        let mut o = vec![];
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut o, 90).encode(&[0u8, 200, 0].repeat(64 * 32), 64, 32, image::ExtendedColorType::Rgb8).unwrap();
        // Densité JFIF 200 dpi.
        let p = o.windows(5).position(|w| w == b"JFIF\0").unwrap();
        o[p + 7] = 1;
        o[p + 8..p + 10].copy_from_slice(&200u16.to_be_bytes());
        o[p + 10..p + 12].copy_from_slice(&200u16.to_be_bytes());
        std::fs::write(&src, &o).unwrap();
        let pdf = d.join("d.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Jpeg).unwrap();
        assert_eq!((r.largeur_mm * 10.0).round() / 10.0, 8.1);
        assert!(std::fs::read(&pdf).unwrap().windows(10).any(|w| w == b"/DCTDecode"));
        let p = plaques(&pdf, &d, 72, 10, 5);
        assert!(p[2] < 120 && p[1] > 150, "vert : jaune posé : {p:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
