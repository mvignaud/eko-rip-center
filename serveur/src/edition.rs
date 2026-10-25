//! Édition rapide d'un PDF avant le rendu : échelle, rotation, miroir, marge blanche,
//! œillets (repères imprimés le long des bords) et contour de découpe rectangulaire ;
//! finitions (roadmap 3.D) : marques (traits de coupe, croix, contour, plis), fond perdu et
//! blanc de rogne. L'étiquette et le QR code se posent après la mosaïque (`etiquette.rs`).
//! Chaque page devient un formulaire XObject posé par une matrice sur une nouvelle page ;
//! la découpe, lue sur le PDF produit, suit la même transformation.

use crate::imbrication::{PT_PAR_MM, page_en_formulaire};
use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Oeillets {
    /// Écart maximal entre deux œillets, en mm (ils sont répartis régulièrement).
    pub espacement_mm: f64,
    /// Distance du centre de l'œillet au bord, en mm : bord de la page, ou trait de coupe
    /// (vers l'intérieur) avec `depuis_coupe`.
    pub retrait_mm: f64,
    pub diametre_mm: f64,
    /// Bords garnis : haut, bas, gauche, droite.
    pub bords: [bool; 4],
    /// Mode « quantité » : nombre d'œillets de chaque bord (haut, bas, gauche, droite), coins
    /// compris, à la place de l'espacement.
    #[serde(default)]
    pub quantites: Option<[u32; 4]>,
    #[serde(default)]
    pub forme: FormeOeillet,
    /// Le retrait se compte depuis le trait de coupe (bord de l'image), pas depuis le bord
    /// de la page.
    #[serde(default)]
    pub depuis_coupe: bool,
}

impl Default for Oeillets {
    fn default() -> Self {
        Oeillets { espacement_mm: 500.0, retrait_mm: 20.0, diametre_mm: 10.0, bords: [true; 4], quantites: None, forme: FormeOeillet::default(), depuis_coupe: false }
    }
}

/// Dessin d'un œillet ; la croix fait le diamètre.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormeOeillet {
    Cercle,
    Disque,
    Croix,
    /// Cercle et petite croix de centre pour le poinçon (comportement d'avant ce champ).
    #[default]
    CercleCroix,
}

/// Côté de la page.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cote {
    Haut,
    #[default]
    Bas,
    Gauche,
    Droite,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenreTrait {
    #[default]
    Plein,
    Pointille,
    /// Tirets de la couleur sur un trait blanc continu.
    Alterne,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncreTrait {
    #[default]
    Noir,
    /// Les quatre encres à 100 % : visible sur chaque plaque.
    Reperage,
    Blanc,
}

impl EncreTrait {
    fn cmjn(self) -> &'static str {
        match self {
            EncreTrait::Noir => "0 0 0 1",
            EncreTrait::Reperage => "1 1 1 1",
            EncreTrait::Blanc => "0 0 0 0",
        }
    }
}

/// Style commun des marques (traits de coupe, croix, contour, plis).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct StyleTrait {
    #[serde(default)]
    pub genre: GenreTrait,
    /// Tiret et vide du pointillé ou de l'alterné, en mm.
    pub trait_mm: f64,
    pub vide_mm: f64,
    pub epaisseur_mm: f64,
    #[serde(default)]
    pub couleur: EncreTrait,
}

impl Default for StyleTrait {
    fn default() -> Self {
        StyleTrait { genre: GenreTrait::Plein, trait_mm: 3.0, vide_mm: 2.0, epaisseur_mm: 0.25, couleur: EncreTrait::Noir }
    }
}

/// Repère de pli ou de couture, parallèle à un côté, à `distance_mm` de lui vers l'intérieur.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pli {
    pub cote: Cote,
    pub distance_mm: f64,
    /// Couture : trait sur toute la longueur ; pli : traits courts dans la marge, à chaque bout.
    #[serde(default)]
    pub couture: bool,
    /// Masqué : gardé dans la liste, pas imprimé.
    #[serde(default = "vrai")]
    pub visible: bool,
}

fn vrai() -> bool {
    true
}

/// Marques imprimées autour de l'image. Les traits de coupe et les croix partent à
/// `distance_mm` du bord du fond perdu (jamais dedans) et font `longueur_mm`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Marques {
    /// Traits de coupe aux angles (sans effet avec les croix de repérage).
    #[serde(default)]
    pub coupe: bool,
    /// Croix de repérage aux angles.
    #[serde(default)]
    pub reperage: bool,
    /// Contour imprimé sur le trait de coupe.
    #[serde(default)]
    pub contour: bool,
    pub distance_mm: f64,
    pub longueur_mm: f64,
    #[serde(default)]
    pub plis: Vec<Pli>,
    #[serde(default)]
    pub style: StyleTrait,
}

impl Default for Marques {
    fn default() -> Self {
        Marques { coupe: true, reperage: false, contour: false, distance_mm: 3.0, longueur_mm: 5.0, plis: vec![], style: StyleTrait::default() }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenreFond {
    /// Aplat de `couleur`.
    #[default]
    Couleur,
    /// Aplat de la couleur du premier pixel (coin haut gauche de l'image posée).
    PremierPixel,
    /// L'image retournée sur chaque bord.
    Miroir,
    /// Les pixels du bord étirés.
    Duplication,
}

/// Fond perdu autour de l'image, puis blanc de rogne (« trim box ») autour du fond perdu.
/// Côtés : haut, bas, gauche, droite (liés ou non : l'écran envoie les quatre).
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct FondPerdu {
    pub cotes_mm: [f64; 4],
    #[serde(default)]
    pub genre: GenreFond,
    /// CMJN de 0 à 1 (genre `couleur`).
    #[serde(default)]
    pub couleur: [f64; 4],
    #[serde(default)]
    pub rogne_mm: [f64; 4],
}

/// Étiquette imprimée dans la marge du haut ou du bas : nom, date, L × H, lé, copies, mode,
/// référence de commande. Copies et commande n'existent pas encore dans un travail
/// (fiche travail, roadmap 5.6) : leur champ s'imprime vide (« — »).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Etiquette {
    #[serde(default)]
    pub cote: Cote,
    /// Corps du texte, en mm.
    pub taille_mm: f64,
}

impl Default for Etiquette {
    fn default() -> Self {
        Etiquette { cote: Cote::Bas, taille_mm: 3.0 }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub enum NiveauQr {
    L,
    #[default]
    M,
    Q,
    H,
}

/// Données portées par le code, dans cet ordre, `clé=valeur` séparées par `;`.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Balises {
    #[serde(default)]
    pub nom: bool,
    #[serde(default)]
    pub page: bool,
    #[serde(default)]
    pub tuile: bool,
    #[serde(default)]
    pub date: bool,
    #[serde(default)]
    pub travail: bool,
    #[serde(default)]
    pub coupe: bool,
    #[serde(default)]
    pub machine: bool,
    #[serde(default)]
    pub commande: bool,
}

impl Default for Balises {
    fn default() -> Self {
        Balises { nom: true, page: true, tuile: true, date: false, travail: true, coupe: false, machine: false, commande: false }
    }
}

/// QR code imprimé dans la marge du haut ou du bas, à droite.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct CodeMarge {
    #[serde(default)]
    pub balises: Balises,
    /// Côté du carré, en mm.
    pub taille_mm: f64,
    /// Blanc autour du code (et place du contour de découpe), en mm.
    pub marge_mm: f64,
    #[serde(default)]
    pub niveau: NiveauQr,
    /// Contour `CutContour` autour du code, à `marge_mm` : le code se découpe en étiquette.
    #[serde(default)]
    pub contour: bool,
    #[serde(default)]
    pub cote: Cote,
}

impl Default for CodeMarge {
    fn default() -> Self {
        CodeMarge { balises: Balises::default(), taille_mm: 20.0, marge_mm: 3.0, niveau: NiveauQr::M, contour: false, cote: Cote::Bas }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Edition {
    /// Échelle en %.
    #[serde(default = "cent")]
    pub echelle: f64,
    /// Quart de tour à gauche : 0, 90, 180 ou 270.
    #[serde(default)]
    pub rotation: u16,
    /// Miroir horizontal (vitrophanie, caisson lumineux vu de l'arrière).
    #[serde(default)]
    pub miroir: bool,
    /// Blanc ajouté autour de l'image (et du fond perdu), en mm ; agrandi d'office pour
    /// loger les marques, l'étiquette et le code (`marge_effective`).
    #[serde(default)]
    pub marge_mm: f64,
    #[serde(default)]
    pub oeillets: Option<Oeillets>,
    /// Contour `CutContour` rectangulaire, décalé de son bord de référence (mm, négatif =
    /// dedans : le retrait, « choke »).
    #[serde(default)]
    pub contour_mm: Option<f64>,
    /// Bord de référence du contour (roadmap 4.10) : l'image, son fond perdu, ou le blanc de
    /// rogne autour (« trim box »).
    #[serde(default)]
    pub contour_sur: BordContour,
    /// Recadrage (x, y, l, h) en mm dans la page source, origine au coin bas gauche de sa
    /// MediaBox : la page produite est ce cadre, le contenu et la découpe y sont rognés.
    #[serde(default)]
    pub recadrage: Option<[f64; 4]>,
    /// Échelle déliée (largeur, hauteur) en % de la page source : remplace `echelle`, et
    /// la découpe se déforme pareil.
    #[serde(default)]
    pub echelle_xy: Option<[f64; 2]>,
    /// Ce que retourne `miroir`.
    #[serde(default)]
    pub miroir_portee: PorteeMiroir,
    #[serde(default)]
    pub marques: Option<Marques>,
    #[serde(default)]
    pub fond_perdu: Option<FondPerdu>,
    #[serde(default)]
    pub etiquette: Option<Etiquette>,
    #[serde(default)]
    pub code: Option<CodeMarge>,
    /// Filtre couleur (roadmap 3.E2, 3.E3) : appliqué aux plaques après le rendu, pas au PDF ;
    /// réglé dans la visionneuse avec le reste de l'édition (même historique, même envoi).
    #[serde(default)]
    pub filtre: Option<crate::filtre::Filtre>,
}

/// Bord autour duquel le contour rectangulaire est tracé.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BordContour {
    /// Bord de l'image, le trait de coupe (comportement d'avant ce champ).
    #[default]
    Image,
    FondPerdu,
    /// Fond perdu et blanc de rogne : la boîte de rognage.
    Rogne,
}

/// Portée du miroir. ⚠️ À éprouver à l'atelier : selon le support et la pose (vitrophanie
/// vue de l'extérieur, caisson retourné…), la découpe doit suivre l'image ou non ; le choix
/// reste explicite tant qu'aucun essai réel ne l'a tranché.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PorteeMiroir {
    /// Image et tracés de découpe retournés ensemble (comportement d'avant ce champ).
    #[default]
    ImageEtDecoupe,
    /// Image retournée, tracés de découpe laissés dans le sens du fichier.
    ImageSeule,
}

fn cent() -> f64 {
    100.0
}

impl Default for Edition {
    fn default() -> Self {
        Edition { echelle: 100.0, rotation: 0, miroir: false, marge_mm: 0.0, oeillets: None, contour_mm: None, contour_sur: BordContour::Image, recadrage: None, echelle_xy: None, miroir_portee: PorteeMiroir::default(), marques: None, fond_perdu: None, etiquette: None, code: None, filtre: None }
    }
}

impl Edition {
    /// Rien à faire au PDF : la portée d'un miroir absent ne compte pas, le filtre couleur
    /// non plus (il agit sur les plaques).
    pub fn neutre(&self) -> bool {
        let contour_sur = if self.contour_mm.is_some() { self.contour_sur } else { BordContour::default() };
        Edition { miroir_portee: if self.miroir { self.miroir_portee } else { PorteeMiroir::default() }, contour_sur, filtre: None, ..self.clone() } == Edition::default()
    }

    /// Échelle de chaque axe (1 = 100 %).
    pub fn echelles(&self) -> (f64, f64) {
        self.echelle_xy.map_or((self.echelle / 100.0, self.echelle / 100.0), |[x, y]| (x / 100.0, y / 100.0))
    }

    pub fn verifier(&self) -> Result<(), &'static str> {
        let o = self.oeillets.unwrap_or_default();
        let bon = (1.0..=1000.0).contains(&self.echelle)
            && [0, 90, 180, 270].contains(&self.rotation)
            && (0.0..=500.0).contains(&self.marge_mm)
            && (50.0..=5000.0).contains(&o.espacement_mm)
            && (3.0..=200.0).contains(&o.retrait_mm)
            && (2.0..=50.0).contains(&o.diametre_mm)
            && self.contour_mm.is_none_or(|c| (-50.0..=50.0).contains(&c))
            && self.echelle_xy.is_none_or(|e| e.iter().all(|v| (1.0..=1000.0).contains(v)));
        if !bon {
            return Err("edition_invalide");
        }
        // Cadre vide ou qui sort de la page par la gauche ou le bas : refusé sans lire le PDF.
        // Le bord droit et le haut se vérifient sur chaque page (`appliquer`).
        if self.recadrage.is_some_and(|[x, y, l, h]| ![x, y, l, h].iter().all(|v| v.is_finite()) || x < 0.0 || y < 0.0 || l < 1.0 || h < 1.0) {
            return Err("recadrage_invalide");
        }
        let dans = |v: f64, a: f64, b: f64| (a..=b).contains(&v);
        if self.oeillets.and_then(|o| o.quantites).is_some_and(|q| q.iter().any(|n| !(1..=200).contains(n))) {
            return Err("oeillets_invalides");
        }
        if let Some(m) = &self.marques {
            let s = m.style;
            let bon = dans(m.distance_mm, 0.0, 100.0)
                && dans(m.longueur_mm, 1.0, 100.0)
                && dans(s.epaisseur_mm, 0.05, 5.0)
                && dans(s.trait_mm, 0.1, 50.0)
                && dans(s.vide_mm, 0.1, 50.0)
                && m.plis.len() <= 20
                && m.plis.iter().all(|p| dans(p.distance_mm, 0.0, 5000.0));
            if !bon {
                return Err("marques_invalides");
            }
        }
        if let Some(f) = &self.fond_perdu
            && !(f.cotes_mm.iter().all(|v| dans(*v, 0.0, 100.0)) && f.rogne_mm.iter().all(|v| dans(*v, 0.0, 200.0)) && f.couleur.iter().all(|v| dans(*v, 0.0, 1.0)))
        {
            return Err("fond_perdu_invalide");
        }
        if self.etiquette.is_some_and(|t| !dans(t.taille_mm, 1.5, 20.0) || !matches!(t.cote, Cote::Haut | Cote::Bas)) {
            return Err("etiquette_invalide");
        }
        if self.code.is_some_and(|c| !dans(c.taille_mm, 5.0, 100.0) || !dans(c.marge_mm, 0.0, 50.0) || !matches!(c.cote, Cote::Haut | Cote::Bas)) {
            return Err("code_invalide");
        }
        self.filtre.as_ref().map_or(Ok(()), crate::filtre::Filtre::verifier)
    }

    /// Marge blanche réellement posée, en mm : celle saisie, agrandie pour loger les marques
    /// (au-delà du fond perdu et du blanc de rogne), l'étiquette et le code.
    pub fn marge_effective(&self) -> f64 {
        let rogne = self.fond_perdu.map_or([0.0; 4], |f| f.rogne_mm);
        let mut besoin = self.marge_mm;
        if let Some(m) = &self.marques
            && (m.coupe || m.reperage || m.plis.iter().any(|p| p.visible && !p.couture))
        {
            // Les croix débordent de leur centre d'une demi-longueur : même place que les traits.
            let l = m.distance_mm + m.longueur_mm + m.style.epaisseur_mm;
            besoin = besoin.max(rogne.iter().map(|r| l - r).fold(0.0, f64::max));
        }
        if let Some(t) = self.etiquette {
            besoin = besoin.max(t.taille_mm + 4.0);
        }
        if let Some(c) = self.code {
            besoin = besoin.max(c.taille_mm + 2.0 * c.marge_mm + 2.0);
        }
        besoin
    }

    /// Fond perdu de chaque côté, en mm : gauche, bas, droite, haut.
    pub fn fond_mm(&self) -> [f64; 4] {
        self.fond_perdu.map_or([0.0; 4], |f| {
            let [h, b, g, d] = f.cotes_mm;
            [g, b, d, h]
        })
    }

    /// Du bord de la page au trait de coupe (bord de l'image), en mm : gauche, bas, droite,
    /// haut. Marge, blanc de rogne et fond perdu.
    pub fn bords_mm(&self) -> [f64; 4] {
        let m = self.marge_effective();
        let [h, b, g, d] = self.fond_perdu.map_or([0.0; 4], |f| f.rogne_mm);
        let fond = self.fond_mm();
        [m + g + fond[0], m + b + fond[1], m + d + fond[2], m + h + fond[3]]
    }
}

/// Centres des œillets d'un rectangle `l × h` (pt) : sur chaque bord garni, à `retrait` du
/// bord, coins compris ; écart régulier au plus `espacement`, ou nombre fixe par bord
/// (`quantites`, un seul : au milieu).
pub fn centres_oeillets(l: f64, h: f64, o: &Oeillets) -> Vec<(f64, f64)> {
    let (r, e) = (o.retrait_mm * PT_PAR_MM, o.espacement_mm * PT_PAR_MM);
    let repartir = |debut: f64, fin: f64, quantite: Option<u32>| -> Vec<f64> {
        let longueur = fin - debut;
        if longueur <= 0.0 {
            return vec![debut];
        }
        let n = match quantite {
            Some(1) => return vec![debut + longueur / 2.0],
            Some(q) => q as usize - 1,
            None => (longueur / e).ceil().max(1.0) as usize,
        };
        (0..=n).map(|i| debut + longueur * i as f64 / n as f64).collect()
    };
    let q = |i: usize| o.quantites.map(|q| q[i]);
    let [haut, bas, gauche, droite] = o.bords;
    let mut c = vec![];
    if bas {
        c.extend(repartir(r, l - r, q(1)).into_iter().map(|x| (x, r)));
    }
    if haut {
        c.extend(repartir(r, l - r, q(0)).into_iter().map(|x| (x, h - r)));
    }
    if gauche {
        c.extend(repartir(r, h - r, q(2)).into_iter().map(|y| (r, y)));
    }
    if droite {
        c.extend(repartir(r, h - r, q(3)).into_iter().map(|y| (l - r, y)));
    }
    // Les coins sont partagés par deux bords : une seule fois.
    let mut uniques: Vec<(f64, f64)> = vec![];
    for p in c {
        if !uniques.iter().any(|q| (q.0 - p.0).abs() < 0.01 && (q.1 - p.1).abs() < 0.01) {
            uniques.push(p);
        }
    }
    uniques
}

/// Tracé des œillets : contour noir (disque : rempli), centres décalés de (`dx`, `dy`).
fn dessin_oeillets(o: &Oeillets, centres: &[(f64, f64)]) -> String {
    let r = o.diametre_mm * PT_PAR_MM / 2.0;
    let mut s = String::from("q 0 0 0 1 K 0 0 0 1 k 0.6 w ");
    for &(x, y) in centres {
        if matches!(o.forme, FormeOeillet::Cercle | FormeOeillet::Disque | FormeOeillet::CercleCroix) {
            s += &cercle(x, y, r);
        }
        // Croix : le diamètre ; cercle + croix : petite croix de centre pour le poinçon.
        let b = match o.forme {
            FormeOeillet::Croix => r,
            FormeOeillet::CercleCroix => 1.5,
            _ => 0.0,
        };
        if b > 0.0 {
            s += &format!("{:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} m {:.3} {:.3} l ", x - b, y, x + b, y, x, y - b, x, y + b);
        }
    }
    s += if o.forme == FormeOeillet::Disque { "f Q\n" } else { "S Q\n" };
    s
}

/// Opérateurs de style d'un trait (épaisseur, tirets, couleur) ; `alterne` : la passe
/// blanche continue sous les tirets.
fn style_pdf(s: &StyleTrait, dessous: bool) -> String {
    let w = s.epaisseur_mm * PT_PAR_MM;
    let (t, v) = (s.trait_mm * PT_PAR_MM, s.vide_mm * PT_PAR_MM);
    match (s.genre, dessous) {
        (GenreTrait::Alterne, true) => format!("{w:.3} w [] 0 d 0 0 0 0 K"),
        (GenreTrait::Plein, _) => format!("{w:.3} w [] 0 d {} K", s.couleur.cmjn()),
        _ => format!("{w:.3} w [{t:.3} {v:.3}] 0 d {} K", s.couleur.cmjn()),
    }
}

/// Segments (x0, y0, x1, y1) et cercles (x, y, rayon), en pt.
type Traits = (Vec<[f64; 4]>, Vec<(f64, f64, f64)>);

/// Segments (x0, y0, x1, y1, en pt) des marques autour de l'image `[ix, iy, l, h]`, fond
/// perdu `fond` (gauche, bas, droite, haut, en pt). Les croix ajoutent un cercle chacune
/// (centre, rayon).
pub fn segments_marques(m: &Marques, [ix, iy, l, h]: [f64; 4], fond: [f64; 4]) -> Traits {
    let mm = PT_PAR_MM;
    let (dist, long) = (m.distance_mm * mm, m.longueur_mm * mm);
    let [fg, fb, fd, fh] = fond;
    let (x1, y1) = (ix + l, iy + h);
    let mut s = vec![];
    let mut cercles = vec![];
    if m.reperage {
        // Croix centrées sur la diagonale de chaque angle, hors du fond perdu.
        for (cx, cy) in [(ix - fg - dist - long / 2.0, iy - fb - dist - long / 2.0), (x1 + fd + dist + long / 2.0, iy - fb - dist - long / 2.0), (ix - fg - dist - long / 2.0, y1 + fh + dist + long / 2.0), (x1 + fd + dist + long / 2.0, y1 + fh + dist + long / 2.0)] {
            s.push([cx - long / 2.0, cy, cx + long / 2.0, cy]);
            s.push([cx, cy - long / 2.0, cx, cy + long / 2.0]);
            cercles.push((cx, cy, long / 4.0));
        }
    } else if m.coupe {
        // Dans le prolongement des traits de coupe, à `dist` du bord du fond perdu.
        for y in [iy, y1] {
            s.push([ix - fg - dist - long, y, ix - fg - dist, y]);
            s.push([x1 + fd + dist, y, x1 + fd + dist + long, y]);
        }
        for x in [ix, x1] {
            s.push([x, iy - fb - dist - long, x, iy - fb - dist]);
            s.push([x, y1 + fh + dist, x, y1 + fh + dist + long]);
        }
    }
    if m.contour {
        s.extend([[ix, iy, x1, iy], [x1, iy, x1, y1], [x1, y1, ix, y1], [ix, y1, ix, iy]]);
    }
    for p in m.plis.iter().filter(|p| p.visible) {
        let d = p.distance_mm * mm;
        let horizontal = matches!(p.cote, Cote::Haut | Cote::Bas);
        let v = match p.cote {
            Cote::Haut => y1 - d,
            Cote::Bas => iy + d,
            Cote::Gauche => ix + d,
            Cote::Droite => x1 - d,
        };
        match (horizontal, p.couture) {
            (true, true) => s.push([ix - fg, v, x1 + fd, v]),
            (false, true) => s.push([v, iy - fb, v, y1 + fh]),
            (true, false) => s.extend([[ix - fg - dist - long, v, ix - fg - dist, v], [x1 + fd + dist, v, x1 + fd + dist + long, v]]),
            (false, false) => s.extend([[v, iy - fb - dist - long, v, iy - fb - dist], [v, y1 + fh + dist, v, y1 + fh + dist + long]]),
        }
    }
    (s, cercles)
}

fn dessin_marques(m: &Marques, image: [f64; 4], fond: [f64; 4]) -> String {
    let (segments, cercles) = segments_marques(m, image, fond);
    let mut chemin = String::new();
    for [a, b, c, d] in &segments {
        chemin += &format!("{a:.4} {b:.4} m {c:.4} {d:.4} l ");
    }
    for &(x, y, r) in &cercles {
        chemin += &cercle(x, y, r);
    }
    if chemin.is_empty() {
        return String::new();
    }
    let passe = |dessous: bool| format!("q {} {chemin}S Q\n", style_pdf(&m.style, dessous));
    if m.style.genre == GenreTrait::Alterne { passe(true) + &passe(false) } else { passe(false) }
}

/// Cercle approché par quatre Béziers, centre (x, y), rayon `r`.
fn cercle(x: f64, y: f64, r: f64) -> String {
    let k = 0.552_284_75 * r;
    format!(
        "{:.3} {:.3} m {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {:.3} c h ",
        x + r,
        y,
        x + r,
        y + k,
        x + k,
        y + r,
        x,
        y + r,
        x - k,
        y + r,
        x - r,
        y + k,
        x - r,
        y,
        x - r,
        y - k,
        x - k,
        y - r,
        x,
        y - r,
        x + k,
        y - r,
        x + r,
        y - k,
        x + r,
        y
    )
}

/// Largeur de la bande de bord étirée par `duplication` et `premier_pixel`, en pt (0,1 mm).
const BORD_ETIRE: f64 = 0.1 * PT_PAR_MM;

/// Fond perdu sous l'image `[ix, iy, l, h]` posée par `m` ; `fond` : gauche, bas, droite,
/// haut, en pt. L'image vient du formulaire `/N`, aux tons de découpe neutralisés : le fond
/// perdu ne crée aucun tracé.
fn dessin_fond(f: &FondPerdu, m: &[f64; 6], [ix, iy, l, h]: [f64; 4], fond: [f64; 4]) -> String {
    let [fg, fb, fd, fh] = fond;
    let exterieur = [ix - fg, iy - fb, l + fg + fd, h + fb + fh];
    let poser = |clip: [f64; 4], t: [f64; 6]| {
        let n = produit(m, &t);
        format!("q {:.4} {:.4} {:.4} {:.4} re W n {:.6} {:.6} {:.6} {:.6} {:.4} {:.4} cm /N Do Q\n", clip[0], clip[1], clip[2], clip[3], n[0], n[1], n[2], n[3], n[4], n[5])
    };
    match f.genre {
        GenreFond::Couleur => {
            let [c, mg, j, k] = f.couleur;
            format!("q {c:.4} {mg:.4} {j:.4} {k:.4} k {:.4} {:.4} {:.4} {:.4} re f Q\n", exterieur[0], exterieur[1], exterieur[2], exterieur[3])
        }
        GenreFond::PremierPixel => {
            // Le coin haut gauche (BORD_ETIRE de côté) étiré sur tout le fond perdu.
            let (kx, ky) = (exterieur[2] / BORD_ETIRE, exterieur[3] / BORD_ETIRE);
            poser(exterieur, [kx, 0.0, 0.0, ky, exterieur[0] - kx * ix, exterieur[1] - ky * (iy + h - BORD_ETIRE)])
        }
        GenreFond::Miroir | GenreFond::Duplication => {
            // Huit zones autour de l'image : chaque axe retourné (miroir) ou étiré depuis la
            // bande du bord (duplication) du côté de la zone, inchangé au milieu.
            let axe = |sens: i8, debut: f64, long: f64, avant: f64, apres: f64| -> ([f64; 2], f64, f64) {
                match (sens, f.genre) {
                    (-1, GenreFond::Miroir) => ([debut - avant, avant], -1.0, 2.0 * debut),
                    (1, GenreFond::Miroir) => ([debut + long, apres], -1.0, 2.0 * (debut + long)),
                    (-1, _) => ([debut - avant, avant], avant / BORD_ETIRE, debut - avant - avant / BORD_ETIRE * debut),
                    (1, _) => ([debut + long, apres], apres / BORD_ETIRE, debut + long - apres / BORD_ETIRE * (debut + long - BORD_ETIRE)),
                    _ => ([debut, long], 1.0, 0.0),
                }
            };
            let mut s = String::new();
            for sx in [-1, 0, 1] {
                for sy in [-1, 0, 1] {
                    let ([x, w], ax, ex) = axe(sx, ix, l, fg, fd);
                    let ([y, hh], ay, ey) = axe(sy, iy, h, fb, fh);
                    if (sx, sy) != (0, 0) && w > 0.0 && hh > 0.0 {
                        s += &poser([x, y, w, hh], [ax, 0.0, 0.0, ay, ex, ey]);
                    }
                }
            }
            s
        }
    }
}

/// Produit de matrices PDF : d'abord `m`, puis `n`.
fn produit(m: &[f64; 6], n: &[f64; 6]) -> [f64; 6] {
    [m[0] * n[0] + m[1] * n[2], m[0] * n[1] + m[1] * n[3], m[2] * n[0] + m[3] * n[2], m[2] * n[1] + m[3] * n[3], m[4] * n[0] + m[5] * n[2] + n[4], m[4] * n[1] + m[5] * n[3] + n[5]]
}

/// Applique l'édition à chaque page du PDF. Erreur : (code, détail).
pub fn appliquer(pdf: &[u8], e: &Edition) -> Result<Vec<u8>, (&'static str, String)> {
    let invalide = |err: String| ("edition_invalide", err);
    let nb_pages = Document::load_mem(pdf).map_err(|err| invalide(format!("PDF illisible : {err}")))?.get_pages().len();
    // Miroir de l'image seule : l'image vient d'une copie aux tons de découpe neutralisés
    // (ils ne laissent aucun tracé), et les tracés, lus sur l'original, sont redessinés
    // sans miroir par-dessus.
    let image_seule = e.miroir && e.miroir_portee == PorteeMiroir::ImageSeule;
    let fond = e.fond_mm().map(|v| v * PT_PAR_MM);
    // Fond perdu tiré de l'image : depuis la copie neutralisée, comme le miroir image seule.
    let fond_image = e.fond_perdu.is_some_and(|f| f.genre != GenreFond::Couleur) && fond.iter().any(|v| *v > 0.0);
    let neutre = if image_seule || fond_image { Some(crate::decoupe::neutraliser(pdf).map_err(invalide)?.0) } else { None };
    let decoupes = if image_seule { crate::decoupe::extraire(pdf, false).map_err(invalide)? } else { vec![] };
    let mut cible = Document::with_version("1.6");
    let pages_id = cible.new_object_id();
    let coupe = cible.add_object(separation("CutContour"));
    let (kx, ky) = e.echelles();
    let [bg, bb, bd, bh] = e.bords_mm().map(|v| v * PT_PAR_MM);
    let mut enfants = vec![];
    let mut tons: Vec<String> = vec![];
    for n in 1..=nb_pages {
        let (form, t, [llx, lly]) = page_en_formulaire(&mut cible, if image_seule { neutre.as_deref().unwrap_or(pdf) } else { pdf }, n).map_err(|err| invalide(format!("page {n} : {err}")))?;
        let form_fond = match (&neutre, fond_image) {
            (Some(neutre), true) if !image_seule => Some(page_en_formulaire(&mut cible, neutre, n).map_err(|err| invalide(format!("page {n} : {err}")))?.0),
            (_, true) => Some(form),
            _ => None,
        };
        // Cadre gardé, en points de la page source (origine au coin de sa MediaBox).
        let [cx, cy, cl, ch] = match e.recadrage {
            None => [0.0, 0.0, t.largeur, t.hauteur],
            Some(c) => {
                let [cx, cy, cl, ch] = c.map(|v| v * PT_PAR_MM);
                // 0,01 mm de jeu : un cadre saisi au bord exact de la page reste dedans.
                let jeu = 0.01 * PT_PAR_MM;
                if cx + cl > t.largeur + jeu || cy + ch > t.hauteur + jeu {
                    let mm = |v: f64| v / PT_PAR_MM;
                    return Err(("recadrage_invalide", format!("page {n} : cadre {:.1} × {:.1} mm en ({:.1}, {:.1}) hors de la page {:.1} × {:.1} mm", c[2], c[3], c[0], c[1], mm(t.largeur), mm(t.hauteur))));
                }
                // Le formulaire ne montre que le cadre : c'est le rognage du contenu, que
                // Ghostscript applique et que la lecture de la découpe suit.
                for f in [Some(form), form_fond].into_iter().flatten() {
                    if let Ok(Object::Stream(f)) = cible.get_object_mut(f) {
                        f.dict.set("BBox", [llx + cx, lly + cy, llx + cx + cl, lly + cy + ch].iter().map(|v| Object::Real(*v as f32)).collect::<Vec<_>>());
                    }
                }
                [cx, cy, cl, ch]
            }
        };
        // Image transformée : taille après échelle, puis après rotation.
        let (w, h) = (cl * kx, ch * ky);
        let (lw, lh) = if e.rotation % 180 == 90 { (h, w) } else { (w, h) };
        // (x, y) → origine du cadre à 0, échelle et miroir dans [0, w] × [0, h].
        let sx = if e.miroir { -kx } else { kx };
        let base = [sx, 0.0, 0.0, ky, -sx * (llx + cx) + if e.miroir { w } else { 0.0 }, -ky * (lly + cy)];
        // Quart de tour à gauche ramené dans le premier quadrant.
        let rot: [f64; 6] = match e.rotation {
            90 => [0.0, 1.0, -1.0, 0.0, h, 0.0],
            180 => [-1.0, 0.0, 0.0, -1.0, w, h],
            270 => [0.0, -1.0, 1.0, 0.0, 0.0, w],
            _ => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        };
        let m = produit(&produit(&base, &rot), &[1.0, 0.0, 0.0, 1.0, bg, bb]);
        let (pl, ph) = (lw + bg + bd, lh + bb + bh);
        let image = [bg, bb, lw, lh];
        let mut contenu = match (e.fond_perdu, form_fond.is_some() || fond.iter().any(|v| *v > 0.0)) {
            (Some(f), true) => dessin_fond(&f, &m, image, fond),
            _ => String::new(),
        };
        contenu += &format!("q {:.6} {:.6} {:.6} {:.6} {:.4} {:.4} cm /F Do Q\n", m[0], m[1], m[2], m[3], m[4], m[5]);
        if let Some(d) = decoupes.get(n - 1) {
            // Tracés de l'original, rognés au cadre, posés comme l'image mais sans miroir.
            let sans_miroir = produit(&produit(&[kx, 0.0, 0.0, ky, 0.0, 0.0], &rot), &[1.0, 0.0, 0.0, 1.0, bg, bb]);
            let a_poser: Vec<crate::decoupe::Trace> = d.traces.iter().map(|t| crate::decoupe::Trace { points: t.points.iter().map(|p| (p.0 - cx, p.1 - cy)).collect(), ..t.clone() }).collect();
            for t in crate::decoupe::rogner(a_poser, cl, ch) {
                let i = tons.iter().position(|x| *x == t.ton).unwrap_or_else(|| {
                    tons.push(t.ton.clone());
                    tons.len() - 1
                });
                contenu += &format!("q {:.6} {:.6} {:.6} {:.6} {:.4} {:.4} cm /T{i} CS 1 SCN 0.5 w", sans_miroir[0], sans_miroir[1], sans_miroir[2], sans_miroir[3], sans_miroir[4], sans_miroir[5]);
                for (j, p) in t.points.iter().enumerate() {
                    contenu += &format!(" {:.4} {:.4} {}", p.0, p.1, if j == 0 { "m" } else { "l" });
                }
                contenu += if t.ferme { " h S Q\n" } else { " S Q\n" };
            }
        }
        if let Some(o) = e.oeillets {
            let centres = if o.depuis_coupe { centres_oeillets(lw, lh, &o).into_iter().map(|(x, y)| (x + bg, y + bb)).collect() } else { centres_oeillets(pl, ph, &o) };
            contenu += &dessin_oeillets(&o, &centres);
        }
        if let Some(mq) = &e.marques {
            contenu += &dessin_marques(mq, image, fond);
        }
        if let Some(c) = e.contour_mm {
            // Autour de l'image, de son fond perdu, ou du blanc de rogne : gauche, bas, droite, haut.
            let rogne = e.fond_perdu.map_or([0.0; 4], |f| [f.rogne_mm[2], f.rogne_mm[1], f.rogne_mm[3], f.rogne_mm[0]].map(|v| v * PT_PAR_MM));
            let autour: [f64; 4] = match e.contour_sur {
                BordContour::Image => [0.0; 4],
                BordContour::FondPerdu => fond,
                BordContour::Rogne => std::array::from_fn(|i| fond[i] + rogne[i]),
            };
            let d = c * PT_PAR_MM;
            let (x, y) = (bg - autour[0] - d, bb - autour[1] - d);
            let (l, h) = (lw + autour[0] + autour[2] + 2.0 * d, lh + autour[1] + autour[3] + 2.0 * d);
            if l > 0.0 && h > 0.0 {
                contenu += &format!("q /CC CS 1 SCN 0.5 w {x:.4} {y:.4} {l:.4} {h:.4} re S Q\n");
            }
        }
        let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let mut xobjets = Dictionary::new();
        xobjets.set("F", form);
        if let Some(f) = form_fond {
            xobjets.set("N", f);
        }
        let mut espaces = dictionary! { "CC" => coupe };
        for (i, ton) in tons.iter().enumerate() {
            espaces.set(format!("T{i}"), separation(ton));
        }
        enfants.push(Object::Reference(cible.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(pl as f32), Object::Real(ph as f32)],
            "Contents" => flux,
            "Resources" => dictionary! { "XObject" => xobjets, "ColorSpace" => espaces },
        })));
    }
    let compte = enfants.len() as i64;
    cible.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => compte }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|err| invalide(err.to_string()))?;
    Ok(sortie)
}

/// Espace `Separation` d'un ton de découpe, magenta à l'écran.
fn separation(ton: &str) -> Object {
    Object::Array(vec![
        "Separation".into(),
        Object::Name(ton.as_bytes().to_vec()),
        "DeviceCMYK".into(),
        Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    const MM: f64 = PT_PAR_MM;

    #[test]
    fn oeillets_repartis_coins_compris() {
        // 1000 × 500 mm, retrait 20, écart max 500 : bas et haut 960 mm → 2 intervalles
        // (3 œillets chacun) ; côtés 460 mm → 1 intervalle, coins déjà comptés : 6.
        let o = Oeillets { espacement_mm: 500.0, retrait_mm: 20.0, diametre_mm: 10.0, bords: [true; 4], ..Default::default() };
        let c = centres_oeillets(1000.0 * MM, 500.0 * MM, &o);
        assert_eq!(c.len(), 6, "{c:?}");
        assert!(c.iter().any(|p| (p.0 - 500.0 * MM).abs() < 1e-6 && (p.1 - 20.0 * MM).abs() < 1e-6), "milieu du bas");
        assert_eq!(centres_oeillets(1000.0 * MM, 500.0 * MM, &Oeillets { bords: [true, false, false, false], ..o }).len(), 3, "haut seul");
        assert!(Edition { echelle: 0.0, ..Default::default() }.verifier().is_err());
        assert!(Edition { rotation: 45, ..Default::default() }.verifier().is_err());
        assert!(Edition::default().neutre() && Edition::default().verifier().is_ok());
    }

    fn boite(p: &crate::decoupe::Decoupe, i: usize) -> (f64, f64, f64, f64) {
        p.traces[i].points.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(a, b, c, d), &(x, y)| (a.min(x), b.min(y), c.max(x), d.max(y)))
    }

    #[test]
    fn echelle_rotation_miroir_et_marge_suivis_par_la_decoupe() {
        // Page cyan 100 × 50 pt (MediaBox décalée), contour à 5 pt du bord.
        let source = crate::imbrication::tests::pdf(100.0, 50.0, "1 0 0 0", (30.0, 40.0));
        // 200 %, quart de tour, marge de 10 mm : page (100 + 2m) × (200 + 2m) pt.
        let m = 10.0 * MM;
        let e = Edition { echelle: 200.0, rotation: 90, marge_mm: 10.0, ..Default::default() };
        let p = &crate::decoupe::extraire(&appliquer(&source, &e).unwrap(), false).unwrap()[0];
        assert!((p.largeur - (100.0 + 2.0 * m)).abs() < 1e-3 && (p.hauteur - (200.0 + 2.0 * m)).abs() < 1e-3, "{} × {}", p.largeur, p.hauteur);
        // Contour d'origine à 5 pt du bord : 10 pt une fois doublé.
        let (x0, y0, x1, y1) = boite(p, 0);
        assert!((x0 - (m + 10.0)).abs() < 1e-3 && (x1 - (m + 90.0)).abs() < 1e-3 && (y0 - (m + 10.0)).abs() < 1e-3 && (y1 - (m + 190.0)).abs() < 1e-3, "{x0} {y0} {x1} {y1}");

        // Miroir, marge 5 mm, contour rectangulaire à 3 mm autour de l'image.
        let e = Edition { miroir: true, marge_mm: 5.0, contour_mm: Some(3.0), ..Default::default() };
        let p = &crate::decoupe::extraire(&appliquer(&source, &e).unwrap(), false).unwrap()[0];
        assert_eq!(p.traces.len(), 2);
        let (x0, y0, x1, y1) = boite(p, 1);
        let (m5, d3) = (5.0 * MM, 3.0 * MM);
        assert!((x0 - (m5 - d3)).abs() < 1e-3 && (y0 - (m5 - d3)).abs() < 1e-3 && (x1 - (m5 + 100.0 + d3)).abs() < 1e-3 && (y1 - (m5 + 50.0 + d3)).abs() < 1e-3);
        // Miroir : le contour d'origine reste symétrique (5 pt de chaque bord).
        let (x0, _, x1, _) = boite(p, 0);
        assert!((x0 - (m5 + 5.0)).abs() < 1e-3 && (x1 - (m5 + 95.0)).abs() < 1e-3);
    }

    /// Contour automatique (roadmap 4.10) : retrait de 1 mm sur le bord de l'image → réduit
    /// de 1 mm ± 0,05 de chaque côté ; sur le fond perdu (3 mm) puis la boîte de rognage
    /// (fond perdu + rogne de 2 mm), décalé d'autant.
    #[test]
    fn contour_auto_retrait_et_bords_de_reference() {
        let source = crate::imbrication::tests::pdf(100.0 * MM, 50.0 * MM, "1 0 0 0", (0.0, 0.0));
        let fond = FondPerdu { cotes_mm: [3.0; 4], rogne_mm: [2.0; 4], ..Default::default() };
        let contour = |sur: BordContour, mm: f64| {
            let e = Edition { contour_mm: Some(mm), contour_sur: sur, fond_perdu: Some(fond), ..Default::default() };
            let p = &crate::decoupe::extraire(&appliquer(&source, &e).unwrap(), false).unwrap()[0];
            // Le contour d'origine (5 pt dans l'image) puis le contour ajouté.
            assert_eq!(p.traces.len(), 2);
            let (x0, y0, x1, y1) = boite(p, 1);
            // Image posée à 5 mm (fond perdu 3 + rogne 2) du bord de la page.
            [x0 / MM - 5.0, y0 / MM - 5.0, x1 / MM - 5.0, y1 / MM - 5.0]
        };
        let proche = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 0.05);
        let c = contour(BordContour::Image, -1.0);
        assert!(proche(c, [1.0, 1.0, 99.0, 49.0]), "retrait de 1 mm : {c:?}");
        let c = contour(BordContour::FondPerdu, 0.0);
        assert!(proche(c, [-3.0, -3.0, 103.0, 53.0]), "fond perdu : {c:?}");
        let c = contour(BordContour::Rogne, -1.0);
        assert!(proche(c, [-4.0, -4.0, 104.0, 54.0]), "boîte de rognage moins 1 mm : {c:?}");
        // Ancienne édition (sans le champ) : sur l'image.
        let ancienne: Edition = serde_json::from_str(r#"{"contour_mm": 2.0}"#).unwrap();
        assert_eq!(ancienne.contour_sur, BordContour::Image);
        assert!(Edition { contour_sur: BordContour::Rogne, ..Default::default() }.neutre(), "sans contour, le bord ne compte pas");
    }

    /// Rendu réel : page cyan à gauche, magenta à droite ; miroir → magenta à gauche.
    /// Marge blanche, et anneau d'œillet noir dans le coin.
    #[test]
    fn oeillets_et_miroir_rendus() {
        let mut doc = Document::with_version("1.4");
        let (l, h) = (600.0 * MM, 300.0 * MM);
        let c = doc.add_object(Stream::new(dictionary! {}, format!("1 0 0 0 k 0 0 {} {h} re f 0 1 0 0 k {} 0 {} {h} re f", l / 2.0, l / 2.0, l / 2.0).into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)], "Contents" => c });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut pdf = vec![];
        doc.save_to(&mut pdf).unwrap();

        let o = Oeillets { espacement_mm: 300.0, retrait_mm: 15.0, diametre_mm: 10.0, bords: [true; 4], ..Default::default() };
        let e = Edition { miroir: true, marge_mm: 30.0, oeillets: Some(o), ..Default::default() };
        let d = std::env::temp_dir().join(format!("eko-rip-edition-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("e.pdf"), appliquer(&pdf, &e).unwrap()).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r20"])
            .arg(format!("-sOutputFile={}", d.join("r.tif").display()))
            .arg(d.join("e.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let px = |mm: f64| (mm / 25.4 * 20.0).round() as u32;
        let plaque = |n: &str| image::open(d.join(format!("r({n}).tif"))).unwrap().to_luma8();
        let (noir, cyan, magenta) = (plaque("Black"), plaque("Cyan"), plaque("Magenta"));
        assert_eq!(noir.dimensions(), (px(660.0), px(360.0)));
        let bas = |mm: f64| noir.height() - 1 - px(mm);
        assert_eq!(cyan.get_pixel(px(10.0), bas(180.0))[0], 255, "marge blanche");
        assert_eq!(magenta.get_pixel(px(100.0), bas(180.0))[0], 0, "miroir : magenta à gauche");
        assert_eq!(cyan.get_pixel(px(560.0), bas(180.0))[0], 0, "miroir : cyan à droite");
        // Anneau de 5 mm de rayon autour de (15, 15) mm : encre sur le cercle, pas au milieu
        // entre le cercle et la croix.
        assert!((px(18.0)..=px(22.0)).any(|x| noir.get_pixel(x, bas(15.0))[0] < 128), "anneau d'œillet");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// Segments (x0, y0, x1, y1) tracés hors de toute matrice sur la page 1, et les
    /// opérateurs du flux (pour le style).
    fn segments(pdf: &[u8]) -> (Vec<[f64; 4]>, Vec<lopdf::content::Operation>) {
        let doc = Document::load_mem(pdf).unwrap();
        let page = *doc.get_pages().get(&1).unwrap();
        let ops = lopdf::content::Content::decode(&doc.get_page_content(page)).unwrap().operations;
        let f = |o: &Object| o.as_float().map(f64::from).unwrap();
        let (mut courant, mut s) = ((0.0, 0.0), vec![]);
        for o in &ops {
            match o.operator.as_str() {
                "m" => courant = (f(&o.operands[0]), f(&o.operands[1])),
                "l" => {
                    let p = (f(&o.operands[0]), f(&o.operands[1]));
                    s.push([courant.0, courant.1, p.0, p.1]);
                    courant = p;
                }
                _ => {}
            }
        }
        (s, ops)
    }

    /// Un segment à ces cotes (mm), à 0,1 mm près, dans un sens ou l'autre.
    fn trouve(s: &[[f64; 4]], attendu: [f64; 4]) -> bool {
        let a = attendu.map(|v| v * MM);
        let pres = |x: [f64; 4]| x.iter().zip(a).all(|(p, q)| (p - q).abs() <= 0.1 * MM);
        s.iter().any(|x| pres(*x) || pres([x[2], x[3], x[0], x[1]]))
    }

    /// 3.D1 — page 100 × 50 mm, fond perdu 5 mm (couleur), traits de coupe à 3 mm du fond
    /// perdu, 5 mm de long : marge portée à 8 mm, image en (8 + 5, 8 + 5). Chaque trait, le
    /// contour et les plis à leur cote ± 0,1 mm sur le PDF produit.
    #[test]
    fn marques_a_leur_cote() {
        let source = crate::imbrication::tests::pdf(100.0 * MM, 50.0 * MM, "1 0 0 0", (0.0, 0.0));
        let plis = vec![Pli { cote: Cote::Haut, distance_mm: 10.0, couture: false, visible: true }, Pli { cote: Cote::Gauche, distance_mm: 20.0, couture: true, visible: true }, Pli { cote: Cote::Bas, distance_mm: 4.0, couture: false, visible: false }];
        let marques = Marques { coupe: true, contour: true, plis, ..Default::default() };
        let fond = FondPerdu { cotes_mm: [5.0; 4], genre: GenreFond::Couleur, couleur: [0.0, 0.0, 1.0, 0.0], rogne_mm: [0.0; 4] };
        let e = Edition { marques: Some(marques.clone()), fond_perdu: Some(fond), ..Default::default() };
        assert!(e.verifier().is_ok());
        assert!((e.marge_effective() - 8.25).abs() < 1e-9, "distance + longueur + épaisseur");
        let pdf = appliquer(&source, &e).unwrap();
        let t = crate::imbrication::taille_page(&pdf, 1).unwrap();
        let (m, f) = (8.25, 5.0);
        assert!((t.largeur / MM - (100.0 + 2.0 * (m + f))).abs() < 1e-3);
        let (ix, iy, x1, y1) = (m + f, m + f, m + f + 100.0, m + f + 50.0);
        let (s, ops) = segments(&pdf);
        // Traits de coupe : dans le prolongement des bords, de 3 à 8 mm au-delà du fond perdu.
        for y in [iy, y1] {
            assert!(trouve(&s, [ix - f - 8.0, y, ix - f - 3.0, y]), "coupe gauche en y = {y}");
            assert!(trouve(&s, [x1 + f + 3.0, y, x1 + f + 8.0, y]), "coupe droite en y = {y}");
        }
        for x in [ix, x1] {
            assert!(trouve(&s, [x, iy - f - 8.0, x, iy - f - 3.0]) && trouve(&s, [x, y1 + f + 3.0, x, y1 + f + 8.0]), "coupe en x = {x}");
        }
        assert!(trouve(&s, [ix, iy, x1, iy]) && trouve(&s, [x1, y1, ix, y1]), "contour sur le trait de coupe");
        assert!(trouve(&s, [ix - f - 8.0, y1 - 10.0, ix - f - 3.0, y1 - 10.0]) && trouve(&s, [x1 + f + 3.0, y1 - 10.0, x1 + f + 8.0, y1 - 10.0]), "pli à 10 mm du haut");
        assert!(trouve(&s, [ix + 20.0, iy - f, ix + 20.0, y1 + f]), "couture à 20 mm de la gauche, sur toute la hauteur");
        assert!(!trouve(&s, [ix - f - 8.0, iy + 4.0, ix - f - 3.0, iy + 4.0]), "pli masqué non imprimé");
        // Aucun trait dans le fond perdu ni dans l'image, hors contour, pli et couture.
        let dedans = |x: f64, y: f64| x > (ix - f) * MM + 0.01 && x < (x1 + f) * MM - 0.01 && y > (iy - f) * MM + 0.01 && y < (y1 + f) * MM - 0.01;
        let traits_coupe = s.iter().filter(|x| (x[0] - x[2]).abs() < 1e-6 || (x[1] - x[3]).abs() < 1e-6).filter(|x| (x[2] - x[0]).abs().max((x[3] - x[1]).abs()) < 5.1 * MM);
        assert!(traits_coupe.clone().count() == 8 + 2 && traits_coupe.clone().all(|x| !dedans(x[0], x[1]) && !dedans(x[2], x[3])), "repères hors du fond perdu");
        assert!(ops.iter().any(|o| o.operator == "d" && o.operands[0].as_array().is_ok_and(|a| a.is_empty())), "plein");

        // Croix de repérage : les traits de coupe disparaissent ; pointillé 2/1 mm, 0,5 mm,
        // couleur de repérage.
        let style = StyleTrait { genre: GenreTrait::Pointille, trait_mm: 2.0, vide_mm: 1.0, epaisseur_mm: 0.5, couleur: EncreTrait::Reperage };
        let e = Edition { marques: Some(Marques { reperage: true, style, ..Default::default() }), ..Default::default() };
        let pdf = appliquer(&source, &e).unwrap();
        let (s, ops) = segments(&pdf);
        let m = e.marge_effective();
        assert_eq!(s.len(), 8, "deux bras par croix, aucun trait de coupe");
        let c = m - 3.0 - 2.5;
        assert!(trouve(&s, [c - 2.5, c, c + 2.5, c]) && trouve(&s, [c, c - 2.5, c, c + 2.5]), "croix du coin bas gauche");
        let tirets = ops.iter().find(|o| o.operator == "d").unwrap().operands[0].as_array().unwrap().iter().map(|v| v.as_float().unwrap() as f64 / MM).collect::<Vec<_>>();
        assert!((tirets[0] - 2.0).abs() < 1e-3 && (tirets[1] - 1.0).abs() < 1e-3, "{tirets:?}");
        assert!(ops.iter().any(|o| o.operator == "K" && o.operands.iter().all(|v| v.as_float().unwrap() == 1.0)), "repérage : CMJN 100 %");
        assert!(ops.iter().any(|o| o.operator == "w" && (o.operands[0].as_float().unwrap() as f64 / MM - 0.5).abs() < 1e-3));

        // Alterné : une passe blanche continue, puis les tirets.
        let e = Edition { marques: Some(Marques { style: StyleTrait { genre: GenreTrait::Alterne, ..Default::default() }, ..Default::default() }), ..Default::default() };
        let (s, _) = segments(&appliquer(&source, &e).unwrap());
        assert_eq!(s.len(), 16, "huit traits, deux passes");
        assert!(Edition { marques: Some(Marques { longueur_mm: 0.0, ..Default::default() }), ..Default::default() }.verifier() == Err("marques_invalides"));
    }

    /// PDF `l × h` mm : magenta, bandes cyan de 3 mm à gauche et en haut, contour de coupe.
    fn source_bords(l: f64, h: f64) -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec![
            "Separation".into(),
            "CutContour".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        let (l, h, b) = (l * MM, h * MM, 3.0 * MM);
        let contenu = format!("0 1 0 0 k 0 0 {l} {h} re f 1 0 0 0 k 0 0 {b} {h} re f 0 {} {l} {b} re f /CS0 CS 1 SCN 20 20 {} {} re S", h - b, l - 40.0, h - 40.0);
        let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        o
    }

    /// Rendu Ghostscript en plaques à `dpi` ; lecteur (plaque, x mm, y mm depuis le bas).
    fn rendu(pdf: &[u8], dpi: u32, nom: &str) -> impl Fn(&str, f64, f64) -> u8 + use<> {
        let d = std::env::temp_dir().join(format!("eko-rip-edition-{nom}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("e.pdf"), pdf).unwrap();
        let st = std::process::Command::new("gs").args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", &format!("-r{dpi}")]).arg(format!("-sOutputFile={}", d.join("r.tif").display())).arg(d.join("e.pdf")).status().unwrap();
        assert!(st.success());
        let plaques: std::collections::HashMap<String, image::GrayImage> = ["Cyan", "Magenta", "Yellow", "Black"].iter().map(|n| (n.to_string(), image::open(d.join(format!("r({n}).tif"))).unwrap().to_luma8())).collect();
        std::fs::remove_dir_all(&d).unwrap();
        move |p: &str, x: f64, y: f64| {
            let img = &plaques[p];
            let px = |mm: f64| (mm / 25.4 * dpi as f64) as u32;
            img.get_pixel(px(x), img.height() - 1 - px(y))[0]
        }
    }

    /// 3.D3 — fond perdu de 5 mm : miroir (les 3 mm cyan du bord gauche se retrouvent contre
    /// le bord, le magenta au-delà), duplication (tout le bord cyan), premier pixel (cyan, coin
    /// haut gauche), couleur ; rogne blanche autour ; la découpe n'est pas recopiée.
    #[test]
    fn fond_perdu_miroir_duplication_et_couleur() {
        let source = source_bords(100.0, 60.0);
        let avec = |genre: GenreFond| Edition { fond_perdu: Some(FondPerdu { cotes_mm: [5.0; 4], genre, couleur: [0.0, 0.0, 1.0, 0.0], rogne_mm: [2.0, 2.0, 4.0, 4.0] }), ..Default::default() };
        let bords = avec(GenreFond::Miroir).bords_mm();
        assert_eq!(bords, [9.0, 7.0, 9.0, 7.0], "rogne puis fond perdu (gauche, bas, droite, haut)");
        let (ix, iy) = (9.0, 7.0);
        for genre in [GenreFond::Miroir, GenreFond::Duplication, GenreFond::PremierPixel, GenreFond::Couleur] {
            let pdf = appliquer(&source, &avec(genre)).unwrap();
            assert_eq!(crate::decoupe::extraire(&pdf, false).unwrap()[0].traces.len(), 1, "{genre:?} : un seul contour de coupe");
            let lire = rendu(&pdf, 200, &format!("{genre:?}"));
            let y = iy + 30.0;
            // Au bord gauche : 1,5 mm et 4 mm dans le fond perdu ; rogne blanche à 2 mm du bord.
            let (pres, loin) = ((lire("Cyan", ix - 1.5, y), lire("Magenta", ix - 1.5, y)), (lire("Cyan", ix - 4.0, y), lire("Magenta", ix - 4.0, y)));
            match genre {
                GenreFond::Miroir => assert!(pres == (0, 255) && loin == (255, 0), "miroir : cyan contre le bord, magenta au-delà ({pres:?} {loin:?})"),
                GenreFond::Duplication | GenreFond::PremierPixel => assert!(pres == (0, 255) && loin == (0, 255), "{genre:?} : cyan ({pres:?} {loin:?})"),
                GenreFond::Couleur => assert!(lire("Yellow", ix - 4.0, y) == 0 && loin == (255, 255), "aplat jaune"),
            }
            // Bord droit (magenta) : miroir et duplication magenta, premier pixel cyan.
            let droite = (lire("Cyan", ix + 102.0, y), lire("Magenta", ix + 102.0, y));
            match genre {
                GenreFond::Miroir | GenreFond::Duplication => assert_eq!(droite, (255, 0), "{genre:?} à droite"),
                GenreFond::PremierPixel => assert_eq!(droite, (0, 255)),
                GenreFond::Couleur => assert_eq!(lire("Yellow", ix + 102.0, y), 0),
            }
            assert_eq!((lire("Cyan", 2.0, y), lire("Magenta", 2.0, y), lire("Yellow", 2.0, y)), (255, 255, 255), "{genre:?} : rogne blanche");
        }
        assert_eq!(Edition { fond_perdu: Some(FondPerdu { cotes_mm: [101.0, 0.0, 0.0, 0.0], ..Default::default() }), ..Default::default() }.verifier(), Err("fond_perdu_invalide"));
    }

    /// 3.D4 — quantité par bord (coins compris, un seul : au milieu), depuis le trait de
    /// coupe, et les quatre formes.
    #[test]
    fn oeillets_par_quantite_et_formes() {
        let o = Oeillets { quantites: Some([4, 4, 1, 1]), retrait_mm: 10.0, ..Default::default() };
        let c = centres_oeillets(310.0 * MM, 100.0 * MM, &o);
        // Haut et bas : 4 chacun (x = 10, 106,7, 203,3, 300) ; gauche et droite : 1 au milieu.
        assert_eq!(c.len(), 10, "{c:?}");
        assert!(c.iter().any(|p| (p.0 / MM - (10.0 + 290.0 / 3.0)).abs() < 1e-6 && (p.1 / MM - 10.0).abs() < 1e-6));
        assert!(c.iter().any(|p| (p.0 / MM - 10.0).abs() < 1e-6 && (p.1 / MM - 50.0).abs() < 1e-6), "milieu de la gauche");
        // Mode espacement inchangé.
        assert_eq!(centres_oeillets(310.0 * MM, 100.0 * MM, &Oeillets { retrait_mm: 10.0, espacement_mm: 100.0, ..Default::default() }).len(), 2 * 4, "côtés de 80 mm : leurs coins seulement");
        assert_eq!(Edition { oeillets: Some(Oeillets { quantites: Some([0, 2, 2, 2]), ..Default::default() }), ..Default::default() }.verifier(), Err("oeillets_invalides"));

        // Depuis le trait de coupe : marge 30 mm, retrait 15 mm → centres à 45 mm du bord de page.
        let source = crate::imbrication::tests::pdf(200.0 * MM, 100.0 * MM, "1 0 0 0", (0.0, 0.0));
        let e = |forme, depuis_coupe| Edition { marge_mm: 30.0, oeillets: Some(Oeillets { quantites: Some([2, 2, 2, 2]), retrait_mm: 15.0, forme, depuis_coupe, ..Default::default() }), ..Default::default() };
        let lire = rendu(&appliquer(&source, &e(FormeOeillet::Disque, true)).unwrap(), 100, "oeillets");
        assert_eq!(lire("Black", 45.0, 45.0), 0, "disque plein au centre");
        assert_eq!(lire("Black", 15.0, 15.0), 255, "rien au bord de la page");
        let lire = rendu(&appliquer(&source, &e(FormeOeillet::Cercle, false)).unwrap(), 100, "cercle");
        assert!(lire("Black", 15.0, 15.0) == 255 && [18.0, 19.0, 20.0, 21.0, 22.0].iter().any(|x| lire("Black", *x, 15.0) < 128), "cercle creux au bord de page");
        let lire = rendu(&appliquer(&source, &e(FormeOeillet::Croix, false)).unwrap(), 100, "croix");
        assert!(lire("Black", 15.0, 15.0) < 128 && lire("Black", 18.0, 18.0) == 255, "croix : centre encré, diagonale vide");
    }
}
