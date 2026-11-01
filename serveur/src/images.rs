//! Images reçues (roadmap 9.2) : TIFF, PNG et JPEG deviennent un PDF d'une page à leur taille
//! réelle (résolution du fichier), puis suivent la chaîne d'un PDF (rendu, plaques, profils,
//! blanc, découpe). Le PDF est écrit en flux, bande par bande : une affiche de plusieurs Go
//! ne passe jamais entière en mémoire.
//!
//! Gardés : CMJN, RVB ou gris, 8 ou 16 bits (1 bit en gris) ; la transparence devient un
//! masque (rien n'est posé où l'image est transparente) ; le profil ICC du fichier. Le JPEG
//! est repris tel quel (filtre DCT), sans décodage. Refusés avec un message : TIFF à palette,
//! YCbCr, Lab ou à plans séparés, PNG entrelacé.
//!
//! Canaux en plus d'un TIFF (roadmap 9.2) : chaque ton direct devient une image en espace
//! `Separation` posée en surimpression sur l'image couleur, donc sa propre plaque au rendu.
//! Le nom vient des ressources Photoshop (noms des canaux, 8BIM 1045 ou 1006) ; sans nom,
//! `Spot1`, `Spot2`… Un nom de blanc (`White`, `Blanc`, `WH`, `Spot1`, casse et espaces
//! ignorés) donne la plaque `White`. Les canaux de sélection de Photoshop (masques, 8BIM
//! 1077 ou 1007) sont ignorés et signalés. Sens des valeurs : celui des canaux couleur du
//! fichier (encre posée en CMJN, lumière en RVB ou gris), hypothèse à confirmer sur un
//! vrai TIFF Photoshop à canal de ton direct.

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
    /// Canaux en plus ignorés (canaux de sélection d'un TIFF Photoshop).
    pub canaux_ignores: usize,
    /// Tons directs repris d'un TIFF, nom de leur plaque, dans l'ordre des canaux.
    pub tons_directs: Vec<String>,
    /// Plaque tirée du canal alpha (roadmap 9.12), quand le réglage la demande et que
    /// l'image a un canal alpha.
    pub plaque_alpha: Option<&'static str>,
    /// Taille de la page produite, en mm.
    pub largeur_mm: f64,
    pub hauteur_mm: f64,
}

/// Refus : (code, détail).
pub type Refus = (&'static str, String);

/// Convertit l'image `src` en PDF d'une page dans `dst`. `plaque_alpha` : nom de la plaque
/// (blanc ou vernis) qui reprend le canal alpha d'un PNG ou d'un TIFF, posée en
/// surimpression après les tons directs ; un ton direct du même nom déjà dans le fichier
/// passe avant (pas de seconde plaque).
pub fn vers_pdf(src: &Path, dst: &Path, g: Genre, plaque_alpha: Option<&'static str>) -> Result<Remarques, Refus> {
    let r = match g {
        Genre::Tiff => tiff_vers_pdf(src, dst, plaque_alpha),
        Genre::Png => png_vers_pdf(src, dst, plaque_alpha),
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
    /// Tons directs posés en surimpression : nom de plaque, couleur d'affichage CMJN (0..=1).
    spots: Vec<TonDirect>,
    /// Plaque qui reprend le canal alpha tel quel (sans masque), posée en dernier.
    plaque_alpha: Option<&'static str>,
}

/// Flux à écrire : l'image couleur, le masque de transparence, le ton direct n° k, ou le
/// canal alpha de la plaque blanc ou vernis.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Flux {
    Couleur,
    Masque,
    Spot(usize),
    Alpha,
}

/// Nom PDF : caractères hors ASCII visible et délimiteurs en `#xx`.
fn nom_pdf(nom: &str) -> String {
    nom.bytes().map(|b| if (0x21..=0x7e).contains(&b) && !b"#/()<>[]{}%".contains(&b) { (b as char).to_string() } else { format!("#{b:02X}") }).collect()
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

/// Écrit le PDF : `flux` écrit l'image couleur, puis le masque de transparence (si
/// `img.alpha`), puis chaque ton direct, puis la plaque alpha. Les longueurs sont connues
/// d'avance.
fn ecrire_pdf(dst: &Path, img: &Image, mut flux: impl FnMut(&mut dyn Write, Flux) -> Result<(), Refus>) -> Result<(), Refus> {
    let mut f = Compteur { w: BufWriter::new(File::create(dst).map_err(interne)?), n: 0 };
    let (lpt, hpt) = (img.largeur as f64 * 72.0 / img.ppi.0, img.hauteur as f64 * 72.0 / img.ppi.1);
    let ligne = |c: u32| (img.largeur as u64 * c as u64 * img.bits as u64).div_ceil(8);
    let longueur = img.dct.unwrap_or(ligne(img.composantes) * img.hauteur as u64);
    let mut positions: Vec<(usize, u64)> = vec![];
    f.write_all(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n").map_err(interne)?;
    // Tons directs : objets 8, 9…, posés après l'image en surimpression (les autres encres
    // restent dessous) ; la plaque alpha suit, objet 8 + nombre de tons.
    let n_alpha = 8 + img.spots.len();
    let mut spots: String = (0..img.spots.len()).map(|k| format!(" /S{k} Do")).collect();
    let mut xobjets: String = (0..img.spots.len()).map(|k| format!(" /S{k} {} 0 R", 8 + k)).collect();
    if img.plaque_alpha.is_some() {
        spots += " /A Do";
        xobjets += &format!(" /A {n_alpha} 0 R");
    }
    let contenu = if spots.is_empty() { format!("q {lpt:.4} 0 0 {hpt:.4} 0 0 cm /Im Do Q") } else { format!("q {lpt:.4} 0 0 {hpt:.4} 0 0 cm /Im Do /Sur gs{spots} Q") };
    let etats = if spots.is_empty() { "" } else { " /ExtGState << /Sur << /OP true /op true /OPM 1 >> >>" };
    for (n, corps) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {lpt:.4} {hpt:.4}] /Resources << /XObject << /Im 5 0 R{xobjets} >>{etats} >> /Contents 4 0 R >>")),
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
    flux(&mut f, Flux::Couleur)?;
    if f.n - avant != longueur {
        return Err(illisible(format!("image tronquée : {} octets au lieu de {longueur}", f.n - avant)));
    }
    f.write_all(b"\nendstream\nendobj\n").map_err(interne)?;
    if img.alpha {
        let la = ligne(1) * img.hauteur as u64;
        positions.push((6, f.n));
        write!(f, "6 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent {} /Length {la} >>\nstream\n", img.largeur, img.hauteur, img.bits).map_err(interne)?;
        let avant = f.n;
        flux(&mut f, Flux::Masque)?;
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
    let lb = ligne(1) * img.hauteur as u64;
    for (k, (nom, [c, m, j, n])) in img.spots.iter().enumerate() {
        positions.push((8 + k, f.n));
        let masque = if img.alpha { " /SMask 6 0 R" } else { "" };
        write!(f, "{} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace [/Separation /{} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [{c:.3} {m:.3} {j:.3} {n:.3}] /N 1 >>] /BitsPerComponent {} /Length {lb}{masque} >>\nstream\n", 8 + k, img.largeur, img.hauteur, nom_pdf(nom), img.bits).map_err(interne)?;
        let avant = f.n;
        flux(&mut f, Flux::Spot(k))?;
        if f.n - avant != lb {
            return Err(illisible(format!("canal « {nom} » tronqué")));
        }
        f.write_all(b"\nendstream\nendobj\n").map_err(interne)?;
    }
    if let Some(nom) = img.plaque_alpha {
        // Pas de masque : la plaque est l'alpha lui-même (bord adouci compris).
        positions.push((n_alpha, f.n));
        write!(f, "{n_alpha} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace [/Separation /{} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 0.1] /N 1 >>] /BitsPerComponent {} /Length {lb} >>\nstream\n", img.largeur, img.hauteur, nom_pdf(nom), img.bits).map_err(interne)?;
        let avant = f.n;
        flux(&mut f, Flux::Alpha)?;
        if f.n - avant != lb {
            return Err(illisible("canal alpha tronqué"));
        }
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

/// PDF d'une page d'image CMJN 8 bits (épreuve TIFF, roadmap 9.13) : `flux(w, None)` écrit
/// les pixels CMJN entrelacés (255 = pleine encre), puis `flux(w, Some(k))` le ton direct
/// n° k de `tons` (255 = pleine encre), posé en surimpression.
pub(crate) fn pdf_cmjn(dst: &Path, largeur: u32, hauteur: u32, ppi: f64, icc: Option<Vec<u8>>, tons: Vec<TonDirect>, mut flux: impl FnMut(&mut dyn Write, Option<usize>) -> Result<(), String>) -> Result<(), String> {
    let img = Image { largeur, hauteur, ppi: (ppi, ppi), espace: "DeviceCMYK", composantes: 4, bits: 8, icc, decode: None, dct: None, alpha: false, spots: tons, plaque_alpha: None };
    ecrire_pdf(dst, &img, |w, f| match f {
        Flux::Couleur => flux(w, None).map_err(|e| ("erreur_interne", e)),
        Flux::Spot(k) => flux(w, Some(k)).map_err(|e| ("erreur_interne", e)),
        Flux::Masque | Flux::Alpha => Ok(()),
    })
    .map_err(|(_, e)| e)
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
    /// Canaux en plus après la couleur et l'alpha : (indice dans le pixel, ton direct
    /// repris ou `None` pour un canal de sélection ignoré).
    extras: Vec<(usize, Option<TonDirect>)>,
}

/// Ton direct d'un canal en plus : nom de plaque, couleur d'affichage CMJN (0..=1).
pub(crate) type TonDirect = (String, [f64; 4]);

type Decodeur = tiff::decoder::Decoder<Retouche<BufReader<File>>>;

fn ouvrir_tiff(src: &Path) -> Result<Decodeur, Refus> {
    ouvrir_tiff_avec(src, None)
}

fn ouvrir_tiff_avec(src: &Path, retouche: Option<(u64, [u8; 2])>) -> Result<Decodeur, Refus> {
    let f = Retouche { r: BufReader::new(File::open(src).map_err(illisible)?), pos: 0, retouche };
    Ok(tiff::decoder::Decoder::new(f).map_err(illisible)?.with_limits(tiff::decoder::Limits::unlimited()))
}

/// Lecteur qui remplace deux octets à une position du fichier, à la lecture seulement.
struct Retouche<R> {
    r: R,
    pos: u64,
    retouche: Option<(u64, [u8; 2])>,
}

impl<R: Read> Read for Retouche<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.r.read(buf)?;
        if let Some((a, o)) = self.retouche {
            for (k, b) in o.iter().enumerate() {
                if let Some(i) = (a + k as u64).checked_sub(self.pos).filter(|i| *i < n as u64) {
                    buf[i as usize] = *b;
                }
            }
        }
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Seek> Seek for Retouche<R> {
    fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
        self.pos = self.r.seek(p)?;
        Ok(self.pos)
    }
}

/// Le décodeur TIFF ne rend que les échantillons de la couleur (les canaux en plus d'un
/// CMJN ou d'un RVB sont perdus). Pour tous les lire, l'interprétation photométrique de la
/// première image est lue « niveaux de gris » (1) : le décodeur rend alors chaque pixel
/// entier, tous canaux. Position et octets de cette valeur dans le fichier.
fn retouche_photometrie(src: &Path) -> Result<(u64, [u8; 2]), Refus> {
    let mut f = BufReader::new(File::open(src).map_err(illisible)?);
    let mut entete = [0u8; 16];
    f.read_exact(&mut entete[..8]).map_err(illisible)?;
    let gros = &entete[..2] == b"MM";
    let u16_ = |o: &[u8]| if gros { u16::from_be_bytes([o[0], o[1]]) } else { u16::from_le_bytes([o[0], o[1]]) };
    let u32_ = |o: &[u8]| { let a: [u8; 4] = o[..4].try_into().unwrap(); if gros { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) } };
    let u64_ = |o: &[u8]| { let a: [u8; 8] = o[..8].try_into().unwrap(); if gros { u64::from_be_bytes(a) } else { u64::from_le_bytes(a) } };
    let grand = u16_(&entete[2..4]) == 43;
    let (ifd, taille_entree, valeur) = if grand {
        f.read_exact(&mut entete[8..16]).map_err(illisible)?;
        (u64_(&entete[8..16]), 20u64, 12u64)
    } else {
        (u32_(&entete[4..8]) as u64, 12, 8)
    };
    f.seek(std::io::SeekFrom::Start(ifd)).map_err(illisible)?;
    let mut n = [0u8; 8];
    let nombre = if grand {
        f.read_exact(&mut n).map_err(illisible)?;
        u64_(&n)
    } else {
        f.read_exact(&mut n[..2]).map_err(illisible)?;
        u16_(&n) as u64
    };
    let debut = ifd + if grand { 8 } else { 2 };
    let mut e = vec![0u8; taille_entree as usize];
    for k in 0..nombre.min(4096) {
        f.read_exact(&mut e).map_err(illisible)?;
        if u16_(&e[..2]) == 262 {
            let un = if gros { [0, 1] } else { [1, 0] };
            return Ok((debut + k * taille_entree + valeur, un));
        }
    }
    Err(illisible("interprétation photométrique absente"))
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
    let photoshop = d.find_tag(Tag::Unknown(34377)).ok().flatten().and_then(|v| v.into_u8_vec().ok()).unwrap_or_default();
    let ressources = ressources_photoshop(&photoshop);
    let ppi = match (x, y) {
        (Some(x), y) if x > 0.0 && unite > 0.0 => Some((x * unite, y.filter(|y| *y > 0.0).unwrap_or(x) * unite)),
        // Photoshop peut écrire « 1 sans unité » dans l'en-tête TIFF et garder la vraie
        // résolution dans ses propres données (relevé le 25/10 sur une affiche de l'atelier).
        _ => resolution_photoshop(&ressources),
    };
    let icc = d.find_tag(Tag::Unknown(34675)).ok().flatten().and_then(|v| v.into_u8_vec().ok()).filter(|p| p.len() > 128);
    let premier = couleur + usize::from(alpha.is_some());
    let extras = canaux_photoshop(&ressources, canaux.saturating_sub(premier)).into_iter().enumerate().map(|(k, c)| (premier + k, c)).collect();
    Ok(InfosTiff { largeur, hauteur, espace, couleur, canaux, alpha, bits, ppi, icc, extras })
}

/// Blocs des ressources d'image Photoshop (`8BIM`, n°, nom Pascal, taille, données).
fn ressources_photoshop(o: &[u8]) -> Vec<(u16, &[u8])> {
    let mut v = vec![];
    let mut i = 0;
    while i + 12 <= o.len() {
        if &o[i..i + 4] != b"8BIM" {
            i += 1;
            continue;
        }
        let id = u16::from_be_bytes([o[i + 4], o[i + 5]]);
        let nom = 1 + o[i + 6] as usize;
        let j = i + 6 + nom + nom % 2;
        let Some(taille) = o.get(j..j + 4).map(|t| u32::from_be_bytes(t.try_into().unwrap()) as usize) else { break };
        let Some(donnees) = o.get(j + 4..j + 4 + taille) else { break };
        v.push((id, donnees));
        i = j + 4 + taille + taille % 2;
    }
    v
}

/// Résolution (bloc 1005, ResolutionInfo) : virgule fixe 16.16, unité 1 = pixels par
/// pouce, 2 = pixels par cm.
fn resolution_photoshop(r: &[(u16, &[u8])]) -> Option<(f64, f64)> {
    let (_, donnees) = r.iter().find(|(id, d)| *id == 1005 && d.len() >= 16)?;
    let fixe = |k: usize| u32::from_be_bytes(donnees[k..k + 4].try_into().unwrap()) as f64 / 65536.0;
    let unite = |k: usize| if u16::from_be_bytes([donnees[k], donnees[k + 1]]) == 2 { 2.54 } else { 1.0 };
    let (h, v) = (fixe(0) * unite(4), fixe(8) * unite(12));
    (h > 0.0).then_some((h, if v > 0.0 { v } else { h }))
}

/// Noms qui donnent la plaque `White` (casse, espaces, `_` et `-` ignorés). `Spot1` : nom
/// du canal de blanc dans les fichiers préparés pour les imprimantes à encre blanche.
const NOMS_BLANC: [&str; 4] = ["white", "blanc", "wh", "spot1"];

fn plaque_du_canal(nom: &str) -> String {
    let n: String = nom.chars().filter(|c| !matches!(c, ' ' | '_' | '-')).collect::<String>().to_lowercase();
    if NOMS_BLANC.contains(&n.as_str()) { crate::blanc::PLAQUE.to_string() } else { nom.trim().chars().take(60).collect() }
}

/// Les `n` canaux en plus : ton direct (nom de plaque, couleur d'affichage) ou `None` pour
/// un canal de sélection. Noms : bloc 1045 (Unicode), sinon 1006 (Pascal) ; nature et
/// couleur : bloc 1077 (version, puis 13 octets par canal), sinon 1007 (14 octets) ;
/// nature 2 = ton direct. Sans ces blocs : tons directs `Spot1`, `Spot2`…
fn canaux_photoshop(r: &[(u16, &[u8])], n: usize) -> Vec<Option<TonDirect>> {
    let bloc = |id: u16| r.iter().find(|(i, _)| *i == id).map(|(_, d)| *d);
    let mut noms: Vec<String> = vec![];
    if let Some(d) = bloc(1045) {
        let mut i = 0;
        while let Some(l) = d.get(i..i + 4).map(|o| u32::from_be_bytes(o.try_into().unwrap()) as usize) {
            let Some(u) = d.get(i + 4..i + 4 + 2 * l) else { break };
            let unites: Vec<u16> = u.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).take_while(|&c| c != 0).collect();
            noms.push(String::from_utf16_lossy(&unites));
            i += 4 + 2 * l;
        }
    } else if let Some(d) = bloc(1006) {
        let mut i = 0;
        while let Some(&l) = d.get(i) {
            let Some(o) = d.get(i + 1..i + 1 + l as usize) else { break };
            noms.push(o.iter().map(|&b| b as char).collect());
            i += 1 + l as usize;
        }
    }
    // (nature, couleur) par canal.
    let affichage: Vec<(u8, [f64; 4])> = match (bloc(1077), bloc(1007)) {
        (Some(d), _) if d.len() >= 4 => d[4..].as_chunks::<13>().0.iter().map(|c| (c[12], couleur_photoshop(c))).collect(),
        (_, Some(d)) => d.as_chunks::<14>().0.iter().map(|c| (c[12], couleur_photoshop(c))).collect(),
        _ => vec![],
    };
    let mut plaques: Vec<String> = vec![];
    (0..n)
        .map(|k| {
            let (nature, cmjn) = affichage.get(k).copied().unwrap_or((2, [0.0, 0.0, 0.0, 0.5]));
            if nature != 2 {
                return None;
            }
            let mut p = plaque_du_canal(noms.get(k).map(String::as_str).filter(|x| !x.trim().is_empty()).unwrap_or(&format!("Spot{}", k + 1)));
            // Deux canaux de même nom : deux plaques distinctes.
            if plaques.contains(&p) {
                p = (2..).map(|i| format!("{p} {i}")).find(|x| !plaques.contains(x)).unwrap();
            }
            plaques.push(p.clone());
            let cmjn = if p == crate::blanc::PLAQUE { [0.0, 0.0, 0.0, 0.1] } else { cmjn };
            Some((p, cmjn))
        })
        .collect()
}

/// Couleur d'affichage d'un canal (espace sur 2 octets, 4 valeurs sur 16 bits) en CMJN
/// 0..=1 : RVB converti simplement, CMJN de Photoshop (stocké inversé), sinon gris moyen.
fn couleur_photoshop(c: &[u8]) -> [f64; 4] {
    let v = |k: usize| u16::from_be_bytes([c[2 + 2 * k], c[3 + 2 * k]]) as f64 / 65535.0;
    match u16::from_be_bytes([c[0], c[1]]) {
        0 => {
            let (cy, ma, ja) = (1.0 - v(0), 1.0 - v(1), 1.0 - v(2));
            let k = cy.min(ma).min(ja);
            if k >= 1.0 { [0.0, 0.0, 0.0, 1.0] } else { [(cy - k) / (1.0 - k), (ma - k) / (1.0 - k), (ja - k) / (1.0 - k), k] }
        }
        2 => [1.0 - v(0), 1.0 - v(1), 1.0 - v(2), 1.0 - v(3)],
        _ => [0.0, 0.0, 0.0, 0.5],
    }
}

/// Lit le TIFF ligne par ligne (bandes ou tuiles), en octets gros-boutistes.
fn lignes_tiff(src: &Path, mut sur_ligne: impl FnMut(&[u8]) -> Result<(), Refus>) -> Result<(), Refus> {
    let i = infos_tiff(&mut ouvrir_tiff(src)?)?;
    // Canaux en plus : lus par la retouche (voir `retouche_photometrie`).
    let mut d = if i.extras.is_empty() { ouvrir_tiff(src)? } else { ouvrir_tiff_avec(src, Some(retouche_photometrie(src)?))? };
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

fn tiff_vers_pdf(src: &Path, dst: &Path, plaque_alpha: Option<&'static str>) -> Result<Remarques, Refus> {
    let i = infos_tiff(&mut ouvrir_tiff(src)?)?;
    let ignores = i.extras.iter().filter(|(_, s)| s.is_none()).count();
    let spots: Vec<(usize, TonDirect)> = i.extras.iter().filter_map(|(k, s)| Some((*k, s.clone()?))).collect();
    // Sens des valeurs : celui des canaux couleur (voir l'en-tête du module).
    let inverser = i.espace != "DeviceCMYK";
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
    // La plaque alpha reprend le canal même partout opaque (image rectangulaire).
    let alpha_brut = i.alpha;
    let plaque_alpha = plaque_alpha.filter(|p| alpha_brut.is_some() && !spots.iter().any(|(_, (n, _))| n == p));
    let i = InfosTiff { alpha: i.alpha.filter(|_| !opaque), ..i };
    let img = Image { largeur: i.largeur, hauteur: i.hauteur, ppi, espace: i.espace, composantes: i.couleur as u32, bits: i.bits, icc: i.icc.clone(), decode: None, dct: None, alpha: i.alpha.is_some(), spots: spots.iter().map(|(_, s)| s.clone()).collect(), plaque_alpha };
    let passe = |w: &mut dyn Write, f: Flux| -> Result<(), Refus> {
        let (mut c, mut a) = (vec![], vec![]);
        lignes_tiff(src, |l| {
            if f == Flux::Couleur && (i.bits == 1 || i.canaux == i.couleur) {
                return w.write_all(l).map_err(interne);
            }
            match f {
                Flux::Couleur | Flux::Masque => {
                    separer(l, i.canaux, i.couleur, i.alpha, octets, &mut c, &mut a);
                    w.write_all(if f == Flux::Masque { &a } else { &c }).map_err(interne)
                }
                Flux::Spot(k) => {
                    separer(l, i.canaux, 0, Some(spots[k].0), octets, &mut c, &mut a);
                    if inverser {
                        a.iter_mut().for_each(|o| *o = !*o);
                    }
                    w.write_all(&a).map_err(interne)
                }
                Flux::Alpha => {
                    separer(l, i.canaux, 0, alpha_brut, octets, &mut c, &mut a);
                    w.write_all(&a).map_err(interne)
                }
            }
        })
    };
    ecrire_pdf(dst, &img, passe)?;
    let mut r = remarques(&img, i.ppi.is_none(), ignores);
    r.plaque_alpha = img.plaque_alpha;
    r.tons_directs = img.spots.iter().map(|(n, _)| n.clone()).collect();
    Ok(r)
}

fn remarques(img: &Image, sans_resolution: bool, canaux_ignores: usize) -> Remarques {
    Remarques { sans_resolution, canaux_ignores, tons_directs: vec![], plaque_alpha: None, largeur_mm: img.largeur as f64 * 25.4 / img.ppi.0, hauteur_mm: img.hauteur as f64 * 25.4 / img.ppi.1 }
}

// --- PNG ---

fn lecteur_png(src: &Path) -> Result<png::Reader<BufReader<File>>, Refus> {
    let mut d = png::Decoder::new(BufReader::new(File::open(src).map_err(illisible)?));
    // Palette et gris < 8 bits dépliés ; 16 bits gardés (gros-boutiste, comme le PDF).
    d.set_transformations(png::Transformations::EXPAND);
    d.read_info().map_err(illisible)
}

fn png_vers_pdf(src: &Path, dst: &Path, plaque_alpha: Option<&'static str>) -> Result<Remarques, Refus> {
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
    let alpha_brut = (canaux > couleur).then_some(couleur);
    let mut alpha = alpha_brut;
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
    let plaque_alpha = plaque_alpha.filter(|_| alpha_brut.is_some());
    let img = Image { largeur, hauteur, ppi: ppi.unwrap_or((PPI_PAR_DEFAUT, PPI_PAR_DEFAUT)), espace, composantes: couleur as u32, bits, icc, decode: None, dct: None, alpha: alpha.is_some(), spots: vec![], plaque_alpha };
    let passe = |w: &mut dyn Write, f: Flux| -> Result<(), Refus> {
        let garder_alpha = matches!(f, Flux::Masque | Flux::Alpha);
        let mut r = lecteur_png(src)?;
        let (mut c, mut a) = (vec![], vec![]);
        while let Some(l) = r.next_row().map_err(illisible)? {
            separer(l.data(), canaux, couleur, alpha_brut, octets, &mut c, &mut a);
            w.write_all(if garder_alpha { &a } else { &c }).map_err(interne)?;
        }
        Ok(())
    };
    ecrire_pdf(dst, &img, passe)?;
    let mut r = remarques(&img, ppi.is_none(), 0);
    r.plaque_alpha = img.plaque_alpha;
    Ok(r)
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
    let img = Image { largeur, hauteur, ppi: ppi.unwrap_or((PPI_PAR_DEFAUT, PPI_PAR_DEFAUT)), espace, composantes: n, bits: 8, icc, decode, dct: Some(taille), alpha: false, spots: vec![], plaque_alpha: None };
    ecrire_pdf(dst, &img, |w, _| {
        f.rewind().map_err(illisible)?;
        std::io::copy(&mut f, w).map(|_| ()).map_err(interne)
    })?;
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
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
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
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
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
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
        assert_eq!((r.largeur_mm.round(), r.hauteur_mm.round(), r.sans_resolution), (51.0, 25.0, false), "300 ppi de Photoshop");
        let o = std::fs::read(&pdf).unwrap();
        assert!(!o.windows(6).any(|w| w == b"/SMask"), "alpha opaque : pas de masque");
        assert!(plaques(&pdf, &d, 10, 5, 5)[1] < 60, "rouge : magenta posé");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Valeur (0 = pleine encre) de la plaque `nom` au point (x, y), après `plaques`.
    fn plaque(d: &Path, nom: &str, x: u32, y: u32) -> u8 {
        image::open(d.join(format!("p({nom}).tif"))).unwrap().to_luma8().get_pixel(x, y)[0]
    }

    /// Ressource Photoshop `8BIM` n° `id` (nom vide).
    fn ressource(id: u16, donnees: &[u8]) -> Vec<u8> {
        let mut o = b"8BIM".to_vec();
        o.extend_from_slice(&id.to_be_bytes());
        o.extend_from_slice(&[0, 0]);
        o.extend_from_slice(&(donnees.len() as u32).to_be_bytes());
        o.extend_from_slice(donnees);
        if donnees.len() % 2 == 1 {
            o.push(0);
        }
        o
    }

    /// CMJN + 2 canaux en plus, 8 bits (le codeur n'a pas ce type).
    struct Cmjn2;
    impl tiff::encoder::colortype::ColorType for Cmjn2 {
        type Inner = u8;
        const TIFF_VALUE: tiff::tags::PhotometricInterpretation = tiff::tags::PhotometricInterpretation::CMYK;
        const BITS_PER_SAMPLE: &'static [u16] = &[8; 6];
        const SAMPLE_FORMAT: &'static [tiff::tags::SampleFormat] = &[tiff::tags::SampleFormat::Uint; 6];
        fn horizontal_predict(_: &[u8], _: &mut Vec<u8>) {
            unreachable!("pas de prédicteur")
        }
    }

    /// Roadmap 9.2 : TIFF CMJN + canal `Spot1` (nom Unicode de Photoshop) → plaque White,
    /// posée en surimpression : le cyan reste dessous.
    #[test]
    fn tiff_cmjn_canal_spot1_plaque_blanc() {
        let d = dossier();
        let src = d.join("s.tif");
        let mut noms = vec![];
        let nom: Vec<u16> = "Spot1".encode_utf16().chain([0]).collect();
        noms.extend_from_slice(&(nom.len() as u32).to_be_bytes());
        nom.iter().for_each(|u| noms.extend_from_slice(&u.to_be_bytes()));
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        let mut im = e.new_image::<tiff::encoder::colortype::CMYKA8>(20, 10).unwrap();
        im.resolution(tiff::tags::ResolutionUnit::Inch, tiff::encoder::Rational { n: 72, d: 1 });
        im.encoder().write_tag(tiff::tags::Tag::ExtraSamples, &[0u16][..]).unwrap();
        im.encoder().write_tag(tiff::tags::Tag::Unknown(34377), &ressource(1045, &noms)[..]).unwrap();
        // Cyan partout ; le canal en plus plein à gauche, vide à droite.
        let px: Vec<u8> = (0..20 * 10).flat_map(|i| [255, 0, 0, 0, if i % 20 < 10 { 255 } else { 0 }]).collect();
        im.write_data(&px).unwrap();
        let pdf = d.join("s.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
        assert_eq!((r.tons_directs.clone(), r.canaux_ignores), (vec!["White".to_string()], 0));
        assert_eq!(plaques(&pdf, &d, 72, 3, 5), [0, 255, 255, 255], "cyan gardé sous le blanc");
        assert_eq!((plaque(&d, "White", 3, 5), plaque(&d, "White", 16, 5)), (0, 255), "blanc plein à gauche seulement");
        assert_eq!(plaques(&pdf, &d, 72, 16, 5)[0], 0);
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Canal de sélection (nature 0 du bloc 1077) ignoré et compté ; ton direct nommé par le
    /// bloc 1006 gardé sous son nom ; deux noms de blanc reconnus.
    #[test]
    fn tiff_canaux_selection_et_ton_nomme() {
        let d = dossier();
        let src = d.join("n.tif");
        let mut noms = vec![9];
        noms.extend_from_slice(b"Selection");
        noms.push(13);
        noms.extend_from_slice(b"PANTONE 485 C");
        let mut affichage = 1u32.to_be_bytes().to_vec();
        // RVB rouge, opacité 100, nature 0 (sélection) puis 2 (ton direct).
        for nature in [0u8, 2] {
            affichage.extend_from_slice(&[0, 0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 100, nature]);
        }
        let mut res = ressource(1006, &noms);
        res.extend(ressource(1077, &affichage));
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        let mut im = e.new_image::<Cmjn2>(8, 8).unwrap();
        im.encoder().write_tag(tiff::tags::Tag::ExtraSamples, &[0u16, 0][..]).unwrap();
        im.encoder().write_tag(tiff::tags::Tag::Unknown(34377), &res[..]).unwrap();
        im.write_data(&[0u8, 0, 0, 0, 255, 255].repeat(64)).unwrap();
        let pdf = d.join("n.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
        assert_eq!((r.tons_directs.clone(), r.canaux_ignores), (vec!["PANTONE 485 C".to_string()], 1));
        let o = std::fs::read(&pdf).unwrap();
        assert!(o.windows(19).any(|w| w == b"/PANTONE#20485#20C "), "nom PDF échappé");
        plaques(&pdf, &d, 72, 2, 2);
        assert_eq!(plaque(&d, "PANTONE 485 C", 2, 2), 0);
        assert!(!d.join("p(Selection).tif").exists(), "la sélection n'est pas une plaque");
        assert_eq!(plaque_du_canal("Blanc"), "White");
        assert_eq!(plaque_du_canal("spot_1"), "White");
        assert_eq!(plaque_du_canal("Spot 2"), "Spot 2");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// RVB + canal sans nom : `Spot1` → White ; en RVB la valeur est de la lumière (0 = encre).
    #[test]
    fn tiff_rvb_canal_sans_nom() {
        let d = dossier();
        let src = d.join("r.tif");
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        let mut im = e.new_image::<tiff::encoder::colortype::RGBA8>(20, 10).unwrap();
        im.encoder().write_tag(tiff::tags::Tag::ExtraSamples, &[0u16][..]).unwrap();
        let px: Vec<u8> = (0..20 * 10).flat_map(|i| [255, 255, 255, if i % 20 < 10 { 0 } else { 255 }]).collect();
        im.write_data(&px).unwrap();
        let pdf = d.join("r.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Tiff, None).unwrap();
        assert_eq!(r.tons_directs, ["White"]);
        plaques(&pdf, &d, 72, 3, 5);
        assert_eq!((plaque(&d, "White", 3, 5), plaque(&d, "White", 16, 5)), (0, 255));
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
        let r = vers_pdf(&src, &pdf, Genre::Png, None).unwrap();
        assert_eq!((r.largeur_mm.round(), r.hauteur_mm.round()), (25.0, 13.0));
        let p = plaques(&pdf, &d, 10, 3, 2);
        assert!(p[0] < 80 && p[2] > 200, "bleu : cyan posé, pas de jaune : {p:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Roadmap 9.12 : PNG détouré (disque opaque sur fond transparent) avec la plaque alpha
    /// en blanc : la plaque White suit le masque alpha à 1 px près, la couleur reste
    /// détourée. En vernis : plaque Varnish, pas de White.
    #[test]
    fn png_detoure_plaque_blanc_et_vernis() {
        let d = dossier();
        let src = d.join("e.png");
        let (l, h) = (120u32, 80u32);
        let dedans = |x: u32, y: u32| (x as f64 - 60.0).powi(2) + (y as f64 - 40.0).powi(2) < 30.0f64.powi(2);
        let mut e = png::Encoder::new(BufWriter::new(File::create(&src).unwrap()), l, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        let px: Vec<u8> = (0..l * h).flat_map(|i| [255, 0, 0, if dedans(i % l, i / l) { 255 } else { 0 }]).collect();
        w.write_image_data(&px).unwrap();
        w.finish().unwrap();
        let pdf = d.join("e.pdf");
        let r = vers_pdf(&src, &pdf, Genre::Png, Some(crate::blanc::PLAQUE)).unwrap();
        assert_eq!(r.plaque_alpha, Some("White"));
        plaques(&pdf, &d, 72, 0, 0);
        let blanc = image::open(d.join("p(White).tif")).unwrap().to_luma8();
        let magenta = image::open(d.join("p(Magenta).tif")).unwrap().to_luma8();
        assert_eq!(blanc.dimensions(), (l, h));
        let pres = |x: u32, y: u32, v: bool| (x.saturating_sub(1)..=(x + 1).min(l - 1)).any(|a| (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|b| dedans(a, b) == v));
        let mut ecarts = 0;
        for y in 0..h {
            for x in 0..l {
                let encre = blanc.get_pixel(x, y)[0] < 128;
                if encre != dedans(x, y) {
                    ecarts += 1;
                    assert!(pres(x, y, encre), "({x}, {y}) : plaque blanc à plus de 1 px du masque");
                }
            }
        }
        assert!(ecarts < 2 * 30 * 7, "{ecarts} pixels de bord");
        assert_eq!((blanc.get_pixel(60, 40)[0], blanc.get_pixel(3, 3)[0]), (0, 255), "plein au centre, rien hors du disque");
        assert_eq!((magenta.get_pixel(60, 40)[0] < 60, magenta.get_pixel(3, 3)[0]), (true, 255), "couleur détourée");

        let v = dossier();
        let pdf = v.join("v.pdf");
        assert_eq!(vers_pdf(&src, &pdf, Genre::Png, Some(crate::blanc::VERNIS)).unwrap().plaque_alpha, Some("Varnish"));
        plaques(&pdf, &v, 72, 0, 0);
        assert_eq!((plaque(&v, "Varnish", 60, 40), plaque(&v, "Varnish", 3, 3)), (0, 255));
        assert!(!v.join("p(White).tif").exists());
        std::fs::remove_dir_all(&d).unwrap();
        std::fs::remove_dir_all(&v).unwrap();
    }

    /// TIFF RVBA partout opaque : pas de masque (rendu rapide), mais la plaque alpha demandée
    /// couvre toute l'image ; sans canal alpha (JPEG), rien.
    #[test]
    fn plaque_alpha_opaque_et_sans_alpha() {
        let d = dossier();
        let src = d.join("o.tif");
        let mut e = tiff::encoder::TiffEncoder::new(File::create(&src).unwrap()).unwrap();
        e.write_image::<tiff::encoder::colortype::RGBA8>(20, 10, &[0u8, 0, 255, 255].repeat(200)).unwrap();
        let pdf = d.join("o.pdf");
        assert_eq!(vers_pdf(&src, &pdf, Genre::Tiff, Some("White")).unwrap().plaque_alpha, Some("White"));
        assert!(!std::fs::read(&pdf).unwrap().windows(6).any(|w| w == b"/SMask"));
        plaques(&pdf, &d, 72, 0, 0);
        assert_eq!((plaque(&d, "White", 0, 0), plaque(&d, "White", 19, 9)), (0, 0));
        let jpg = d.join("j.jpg");
        image::RgbImage::from_pixel(16, 8, image::Rgb([0, 200, 0])).save(&jpg).unwrap();
        assert_eq!(vers_pdf(&jpg, &d.join("j.pdf"), Genre::Jpeg, Some("White")).unwrap().plaque_alpha, None);
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
        let r = vers_pdf(&src, &pdf, Genre::Jpeg, None).unwrap();
        assert_eq!((r.largeur_mm * 10.0).round() / 10.0, 8.1);
        assert!(std::fs::read(&pdf).unwrap().windows(10).any(|w| w == b"/DCTDecode"));
        let p = plaques(&pdf, &d, 72, 10, 5);
        assert!(p[2] < 120 && p[1] > 150, "vert : jaune posé : {p:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
