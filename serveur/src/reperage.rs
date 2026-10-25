//! Repérage pour l'impression puis la découpe sur le Mutoh Kona.
//!
//! La page est agrandie, son contenu décalé, et les repères ajoutés en noir pur (plaque
//! K seule, le capteur EPOS du Kona lit un noir franc). Cotes : manuel Kona AP-75380
//! chap. 6 et Application Guide Kona, chap. 4.4.2.
//! - `Manuel` : quatre croix P1–P4 aux coins, pointées à la main sur le Kona.
//! - `Cadre` : cadre continu de Type 1, mesuré automatiquement, sans code-barres ni
//!   CutServer.
//! - `CadreCode` (Type 2, « SF ») : cadre continu, deux codes POSTNET
//!   (bas à gauche, haut à droite) qui portent le numéro du fichier de découpe CutServer.
//! - `CadreMulti` (Type 3, « MF ») : angles de 125 mm et repères intermédiaires sur les
//!   côtés, pour une découpe par segments ; mêmes codes.
//!
//! Types 2 et 3 : cotes des tableaux du guide (chap. 4.4.2) ; POSTNET standard (62 barres,
//! chiffre de contrôle), vérifié en décodant l'exemple imprimé du guide. Lus sur les
//! schémas, sans cote : la place du code du haut, l'écart des repères du Type 3.
//! Le Kona n'a pas de lecture automatique sans code-barres (manuel § 6.2 : manuel, Single
//! Frame, Multi Frame) : le Type 1 ne se lit pas seul, essai du 24/10 → EPOS 6213.
//! Provisoire jusqu'au premier essai réel.

use lopdf::{Document, Object, Stream, dictionary};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reperage {
    #[default]
    Aucun,
    Manuel,
    Cadre,
    CadreCode,
    CadreMulti,
}

impl Reperage {
    /// Mot du nom de fichier CutServer (`Nom_SF_<numéro>.plt`) ; `None` sans code-barres.
    pub fn code_cutserver(self) -> Option<&'static str> {
        match self {
            Reperage::CadreCode => Some("SF"),
            Reperage::CadreMulti => Some("MF"),
            _ => None,
        }
    }
}

/// Fond sous les repères (roadmap 4.15), pour un support sombre, transparent ou
/// réfléchissant : une bande d'encre autour de chaque repère et code-barres, ou un aplat de
/// toute la zone des repères (l'image exceptée). Les repères noirs sont posés dessus en
/// surimpression : le fond reste aussi sous eux.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct FondReperes {
    /// Débord du fond autour de chaque repère, en mm (0,5 à 13).
    pub debord_mm: f64,
    /// Aplat de toute la zone des repères, plutôt qu'une bande par repère.
    #[serde(default)]
    pub aplat: bool,
    /// Encre CMJN (0 à 1) ; sans : le ton direct `White` de l'imprimante.
    #[serde(default)]
    pub cmjn: Option<[f64; 4]>,
}

impl FondReperes {
    pub fn verifier(&self) -> Result<(), &'static str> {
        let bon = (0.5..=13.0).contains(&self.debord_mm) && self.cmjn.is_none_or(|c| c.iter().all(|v| (0.0..=1.0).contains(v)));
        if bon { Ok(()) } else { Err("fond_reperes_invalide") }
    }
}

/// Numéro à 11 chiffres d'une page : 9 tirés du travail, 2 de la page (1 à 99).
pub fn numero(travail: uuid::Uuid, page: usize) -> u64 {
    (travail.as_u128() % 1_000_000_000) as u64 * 100 + page as u64 % 100
}

const PT_PAR_MM: f64 = 72.0 / 25.4;

/// Cotes du cadre de Type 1, en mm, choisies au-dessus des minimums du manuel.
pub mod cadre {
    /// Épaisseur : 3 mm minimum, 5 mm conseillés pour une impression jet d'encre.
    pub const EPAISSEUR: f64 = 5.0;
    /// Blanc entre l'image et le cadre : 6 mm minimum.
    pub const MARGE_IMAGE: f64 = 10.0;
    /// Cadre intérieur minimal : 250 × 250 mm.
    pub const INTERIEUR_MIN: f64 = 250.0;
    /// Blanc à gauche et à droite : 20 mm minimum.
    pub const BORD_COTE: f64 = 25.0;
    /// Blanc à l'avant (côté sortie) : 25 mm minimum.
    pub const BORD_AVANT: f64 = 30.0;
    /// Blanc à l'arrière : 75 mm minimum sur une feuille.
    pub const BORD_ARRIERE: f64 = 80.0;
}

/// Code-barres des Types 2 et 3, en mm.
pub mod code {
    /// Hauteur : 20 mm exactement.
    pub const HAUTEUR: f64 = 20.0;
    /// Largeur : 246 mm au moins.
    pub const LARGEUR: f64 = 250.0;
    /// Du côté intérieur du cadre au début du code : 40 à 100 mm.
    pub const DECALAGE: f64 = 50.0;
    /// Du bord intérieur du cadre au pied des barres : 6 à 12 mm.
    pub const AU_CADRE: f64 = 9.0;
    /// Chiffres du numéro, entre le code et le cadre, à 2–6 mm du cadre.
    pub const CHIFFRES_HAUTEUR: f64 = 3.5;
    pub const CHIFFRES_AU_CADRE: f64 = 3.0;
    /// Intérieur minimal : le code et son décalage.
    pub const INTERIEUR_MIN_L: f64 = DECALAGE + LARGEUR + 10.0;
}

/// Côtés du Type 3, en mm.
pub mod multi {
    /// Angles (« start / stop ») : 125 mm exactement.
    pub const ANGLE: f64 = 125.0;
    /// Repères intermédiaires : 10 à 60 mm.
    pub const REPERE: f64 = 30.0;
    /// Écart maximal entre deux repères (non coté dans le guide).
    pub const ECART_MAX: f64 = 500.0;
    /// Intérieur minimal : les deux angles et un repère entre eux.
    pub const INTERIEUR_MIN_H: f64 = 2.0 * ANGLE + 2.0 * REPERE;
}

/// Croix du repérage manuel : bras de 10 mm, trait de 0,5 mm, à 10 mm de l'image.
const CROIX_BRAS: f64 = 10.0;
const CROIX_TRAIT: f64 = 0.5;
const CROIX_MARGE: f64 = 10.0;
const CROIX_BORD: f64 = 20.0;

/// Mise en page : taille de la nouvelle page, décalage du contenu d'origine, repères
/// en rectangles pleins. Tout en points.
#[derive(Debug)]
pub struct Mise {
    pub page: (f64, f64),
    pub decalage: (f64, f64),
    pub rectangles: Vec<[f64; 4]>,
    /// Cadre Mutoh : coins opposés (x1, y1, x2, y2) au MILIEU des traits, ce que le Kona
    /// attend dans `AL5` (mesuré sur la capture ONYX du 24/10 : écart = cadre − épaisseur).
    pub coins: Option<[f64; 4]>,
    /// Repérage réellement appliqué à la page (le cadre simple devient multi-cadre au-delà de
    /// `MULTI_DES_MM`).
    pub reperage: Reperage,
}

/// Longueur (sens d'avance = hauteur de page) à partir de laquelle le cadre simple avec code
/// devient un multi-cadre : manuel Kona AP-75380 § 6.3 — cadre simple déconseillé au-delà de
/// 2 m, multi-cadre déconseillé sous 1,5 m et conçu pour les grandes longueurs.
pub const MULTI_DES_MM: f64 = 1500.0;

/// Repérage effectif d'une page de `hauteur` pt : `CadreCode` passe en `CadreMulti` dès
/// `MULTI_DES_MM` ; les autres restent tels quels.
pub fn effectif(reperage: Reperage, hauteur: f64) -> Reperage {
    if reperage == Reperage::CadreCode && hauteur / PT_PAR_MM >= MULTI_DES_MM { Reperage::CadreMulti } else { reperage }
}

/// `numero` : celui des codes-barres (Types 2 et 3), ignoré sinon.
pub fn mise_en_page(reperage: Reperage, largeur: f64, hauteur: f64, numero: u64) -> Mise {
    let mm = |v: f64| v * PT_PAR_MM;
    match reperage {
        Reperage::Aucun => Mise { page: (largeur, hauteur), decalage: (0.0, 0.0), rectangles: vec![], coins: None, reperage },
        Reperage::Cadre | Reperage::CadreCode | Reperage::CadreMulti => {
            use cadre::*;
            let avec_code = reperage != Reperage::Cadre;
            // Bandes du haut et du bas réservées aux codes, plus la marge de l'image.
            let bande = if avec_code { mm(code::AU_CADRE + code::HAUTEUR) } else { 0.0 };
            let (min_l, min_h) = match reperage {
                Reperage::Cadre => (INTERIEUR_MIN, INTERIEUR_MIN),
                Reperage::CadreCode => (code::INTERIEUR_MIN_L, INTERIEUR_MIN),
                _ => (code::INTERIEUR_MIN_L, multi::INTERIEUR_MIN_H),
            };
            // Intérieur du cadre : l'image et ses marges, au moins les minimums, image centrée.
            let int_l = (largeur + 2.0 * mm(MARGE_IMAGE)).max(mm(min_l));
            let int_h = (hauteur + 2.0 * (mm(MARGE_IMAGE) + bande)).max(mm(min_h));
            let e = mm(EPAISSEUR);
            let (x0, y0) = (mm(BORD_COTE), mm(BORD_AVANT));
            let (ext_l, ext_h) = (int_l + 2.0 * e, int_h + 2.0 * e);
            let decalage = (x0 + e + (int_l - largeur) / 2.0, y0 + e + (int_h - hauteur) / 2.0);
            let (x1, y1) = (x0 + ext_l - e, y0 + ext_h - e);
            let mut rectangles = match reperage {
                // Cadre continu, coins pleins (Type 2 : comme le cadre d'ONYX qui passe sur notre
                // Kona ; les fenêtres du schéma EasySign ne servent pas).
                Reperage::Cadre | Reperage::CadreCode => vec![[x0, y0, ext_l, e], [x0, y1, ext_l, e], [x0, y0, e, ext_h], [x1, y0, e, ext_h]],
                _ => {
                    let mut r = vec![[x0, y0, ext_l, e], [x0, y1, ext_l, e]];
                    let angle = mm(multi::ANGLE);
                    for x in [x0, x1] {
                        r.push([x, y0, e, angle]);
                        r.push([x, y0 + ext_h - angle, e, angle]);
                        // Repères à égale distance entre les angles.
                        let libre = ext_h - 2.0 * angle;
                        let n = (libre / mm(multi::ECART_MAX)).ceil().max(1.0);
                        for k in 0..n as usize {
                            let centre = y0 + angle + libre * (k as f64 + 0.5) / n;
                            r.push([x, centre - mm(multi::REPERE) / 2.0, e, mm(multi::REPERE)]);
                        }
                    }
                    r
                }
            };
            if avec_code {
                let (ix0, iy0, ix1, iy1) = (x0 + e, y0 + e, x1, y1);
                // Bas à gauche, chiffres dessous ; haut à droite, chiffres dessus.
                let (bx, by) = (ix0 + mm(code::DECALAGE), iy0 + mm(code::AU_CADRE));
                rectangles.extend(postnet(numero, bx, by));
                rectangles.extend(chiffres(numero, bx + mm(code::LARGEUR) / 2.0, iy0 + mm(code::CHIFFRES_AU_CADRE)));
                let (hx, hy) = (ix1 - mm(code::DECALAGE) - mm(code::LARGEUR), iy1 - mm(code::AU_CADRE) - mm(code::HAUTEUR));
                rectangles.extend(postnet(numero, hx, hy));
                rectangles.extend(chiffres(numero, hx + mm(code::LARGEUR) / 2.0, iy1 - mm(code::CHIFFRES_AU_CADRE + code::CHIFFRES_HAUTEUR)));
            }
            // Sens d'insertion, comme le ▲ d'ONYX : vers le haut de la page, le bord qui sort
            // le premier de l'Epson (constaté à l'atelier le 24/10). Dans le coin intérieur
            // bas-droit, hors de l'image (marge de 10 mm).
            rectangles.extend(fleche_vers_le_haut(x1 - mm(3.0) - mm(FLECHE) / 2.0, y0 + e + mm(3.0)));
            let coins = Some([x0 + e / 2.0, y0 + e / 2.0, x0 + ext_l - e / 2.0, y0 + ext_h - e / 2.0]);
            Mise { page: (ext_l + 2.0 * mm(BORD_COTE), y0 + ext_h + mm(BORD_ARRIERE)), decalage, rectangles, coins, reperage }
        }
        Reperage::Manuel => {
            let bord = mm(CROIX_BORD + CROIX_BRAS / 2.0);
            let m = mm(CROIX_MARGE);
            let decalage = (bord + m, bord + m);
            let (bras, trait_) = (mm(CROIX_BRAS), mm(CROIX_TRAIT));
            let mut rectangles = vec![];
            for (cx, cy) in [(bord, bord), (bord + largeur + 2.0 * m, bord), (bord + largeur + 2.0 * m, bord + hauteur + 2.0 * m), (bord, bord + hauteur + 2.0 * m)] {
                rectangles.push([cx - bras / 2.0, cy - trait_ / 2.0, bras, trait_]);
                rectangles.push([cx - trait_ / 2.0, cy - bras / 2.0, trait_, bras]);
            }
            Mise { page: (largeur + 2.0 * (bord + m), hauteur + 2.0 * (bord + m)), decalage, rectangles, coins: None, reperage }
        }
    }
}

/// Consigne du support Kona (repères non lus) : 5 à 8 cm de blanc autour des repères,
/// image d'au moins 25 cm de côté. ⚠️ À confirmer sur le Kona : nos cadres laissent 10 mm
/// entre l'image et le cadre (6 mm minimum au manuel), donc le blanc est toujours signalé.
// Gardé pour l'alerte `reperes_blanc`, suspendue (voir `ecarts`).
#[allow(dead_code)]
pub const BLANC_CONSEILLE_MM: f64 = 50.0;
pub const IMAGE_MIN_MM: f64 = 250.0;

/// Écarts d'une page `largeur × hauteur` (pt) à la consigne : (code, détail).
/// `reperes_image_petite` : un côté sous 25 cm (`reperes_blanc` suspendue, voir plus bas).
pub fn ecarts(reperage: Reperage, largeur: f64, hauteur: f64) -> Vec<(&'static str, String)> {
    if reperage == Reperage::Aucun {
        return vec![];
    }
    let mm = |v: f64| v / PT_PAR_MM;
    let mut e = vec![];
    if mm(largeur.min(hauteur)) < IMAGE_MIN_MM - 1e-6 {
        e.push(("reperes_image_petite", format!("{:.0} × {:.0} mm", mm(largeur), mm(hauteur))));
    }
    // ponytail: l'alerte `reperes_blanc` (moins de BLANC_CONSEILLE_MM de blanc entre un
    // repère et l'image) est suspendue : nos cadres laissent 10 mm, elle se lèverait sur TOUT
    // travail Kona et ne serait plus lue. À rétablir quand la règle sera éprouvée sur le Kona.
    e
}

/// Côté du triangle de la flèche d'insertion, en mm.
const FLECHE: f64 = 5.0;

/// Triangle plein pointant vers le haut (base en y, centrée sur cx), en marches de
/// 0,25 mm : le repérage ne dessine que des rectangles.
fn fleche_vers_le_haut(cx: f64, y: f64) -> Vec<[f64; 4]> {
    let (c, marche) = (FLECHE * PT_PAR_MM, 0.25 * PT_PAR_MM);
    let n = (c * 0.87 / marche).round() as usize;
    (0..n).map(|i| { let l = c * (n - i) as f64 / n as f64; [cx - l / 2.0, y + i as f64 * marche, l, marche] }).collect()
}

/// POSTNET du numéro (11 chiffres et contrôle), pied des barres en (x, y), en points.
/// Barres : 1 d'encadrement, 5 par chiffre (poids 7 4 2 1 0, deux hautes), 1 d'encadrement.
pub fn postnet(numero: u64, x: f64, y: f64) -> Vec<[f64; 4]> {
    const MOTIFS: [&str; 10] = ["11000", "00011", "00101", "00110", "01001", "01010", "01100", "10001", "10010", "10100"];
    let texte = format!("{numero:011}");
    let somme: u32 = texte.chars().filter_map(|c| c.to_digit(10)).sum();
    let controle = (10 - somme % 10) % 10;
    let barres: String = std::iter::once("1").chain(texte.chars().filter_map(|c| c.to_digit(10)).chain([controle]).map(|d| MOTIFS[d as usize])).chain(["1"]).collect();
    let m = PT_PAR_MM;
    // Proportions de la norme : barre = 0,44 pas, courte = 0,4 haute.
    let pas = code::LARGEUR * m / (barres.len() as f64 - 1.0 + 0.44);
    barres.chars().enumerate().map(|(i, b)| [x + i as f64 * pas, y, 0.44 * pas, code::HAUTEUR * m * if b == '1' { 1.0 } else { 0.4 }]).collect()
}

/// Chiffres du numéro en sept segments, centrés sur `cx`, pied en `y`, en points.
fn chiffres(numero: u64, cx: f64, y: f64) -> Vec<[f64; 4]> {
    // Segments a b c d e f g allumés par chiffre.
    const SEGMENTS: [&str; 10] = ["abcdef", "bc", "abdeg", "abcdg", "bcfg", "acdfg", "acdefg", "abc", "abcdefg", "abcdfg"];
    let h = code::CHIFFRES_HAUTEUR * PT_PAR_MM;
    let (l, t, pas) = (0.55 * h, 0.12 * h, 0.8 * h);
    let texte = format!("{numero:011}");
    let x0 = cx - (texte.len() as f64 * pas - (pas - l)) / 2.0;
    let mut r = vec![];
    for (i, c) in texte.chars().enumerate() {
        let x = x0 + i as f64 * pas;
        for s in SEGMENTS[c.to_digit(10).unwrap_or(0) as usize].chars() {
            r.push(match s {
                'a' => [x, y + h - t, l, t],
                'b' => [x + l - t, y + h / 2.0, t, h / 2.0],
                'c' => [x + l - t, y, t, h / 2.0],
                'd' => [x, y, l, t],
                'e' => [x, y, t, h / 2.0],
                'f' => [x, y + h / 2.0, t, h / 2.0],
                _ => [x, y + (h - t) / 2.0, l, t],
            });
        }
    }
    r
}

/// Réécrit chaque page du PDF : nouvelle MediaBox, contenu décalé, repères en noir.
/// Rend une mise en page par page, dans l'ordre. `travail` donne le numéro des codes-barres.
pub fn envelopper(pdf: &[u8], reperage: Reperage, travail: uuid::Uuid, fond: Option<FondReperes>) -> Result<(Vec<u8>, Vec<Mise>), String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    if pages.is_empty() {
        return Err("PDF sans page".into());
    }
    if reperage.code_cutserver().is_some() && pages.len() > 99 {
        return Err("plus de 99 pages : le numéro du code-barres n'en distingue que 99".into());
    }
    let mut mises = vec![];
    for (i, page) in pages.into_iter().enumerate() {
        mises.push(envelopper_page(&mut doc, page, reperage, numero(travail, i + 1), fond).map_err(|e| format!("page {} : {e}", i + 1))?);
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok((sortie, mises))
}

fn envelopper_page(doc: &mut Document, page: lopdf::ObjectId, reperage: Reperage, numero: u64, fond: Option<FondReperes>) -> Result<Mise, String> {
    let dict = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let boite: Vec<f64> = dict
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map(|a| a.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect())
        .map_err(|_| "page sans MediaBox")?;
    if boite.len() != 4 {
        return Err("MediaBox invalide".into());
    }
    // ponytail: page tournée (/Rotate) refusée plutôt que mal placée ; à gérer si besoin.
    if dict.get(b"Rotate").and_then(Object::as_i64).is_ok_and(|r| r % 360 != 0) {
        return Err("page tournée (/Rotate) : repérage non pris en charge".into());
    }
    let reperage = effectif(reperage, boite[3] - boite[1]);
    let mise = mise_en_page(reperage, boite[2] - boite[0], boite[3] - boite[1], numero);
    let (dx, dy) = (mise.decalage.0 - boite[0], mise.decalage.1 - boite[1]);

    let anciens: Vec<Object> = match dict.get(b"Contents") {
        Ok(Object::Array(a)) => a.clone(),
        Ok(o) => vec![o.clone()],
        Err(_) => vec![],
    };
    let debut = doc.add_object(Stream::new(dictionary! {}, format!("q 1 0 0 1 {dx:.4} {dy:.4} cm\n").into_bytes()));
    let mut fin = String::from("\nQ ");
    if let Some(f) = fond.filter(|_| !mise.rectangles.is_empty()) {
        fin += &dessin_fond(&f, &mise, (boite[2] - boite[0], boite[3] - boite[1]));
        ajouter_ressources(doc, page)?;
        // Repères en surimpression (composantes nulles comprises) : le fond reste dessous.
        fin += "q /GSRep gs 0 0 0 1 k\n";
    } else {
        fin += "q 0 0 0 1 k\n";
    }
    for [x, y, l, h] in &mise.rectangles {
        fin += &format!("{x:.4} {y:.4} {l:.4} {h:.4} re f\n");
    }
    fin += "Q\n";
    let fin = doc.add_object(Stream::new(dictionary! {}, fin.into_bytes()));

    let dict = doc.get_dictionary_mut(page).map_err(|e| e.to_string())?;
    let mut contenus = vec![Object::Reference(debut)];
    contenus.extend(anciens);
    contenus.push(Object::Reference(fin));
    dict.set("Contents", Object::Array(contenus));
    dict.set("MediaBox", vec![0.into(), 0.into(), Object::Real(mise.page.0 as f32), Object::Real(mise.page.1 as f32)]);
    for boite in [b"CropBox".as_slice(), b"BleedBox", b"TrimBox", b"ArtBox"] {
        dict.remove(boite);
    }
    Ok(mise)
}

/// Fond sous les repères : une bande par repère (rectangle grossi du débord), ou un aplat de
/// leur zone en pair-impair, l'image (et le débord autour) laissée vierge.
fn dessin_fond(f: &FondReperes, mise: &Mise, (l, h): (f64, f64)) -> String {
    let d = f.debord_mm * PT_PAR_MM;
    let couleur = match f.cmjn {
        Some([c, m, j, k]) => format!("{c:.4} {m:.4} {j:.4} {k:.4} k"),
        None => "/FondRep cs 1 scn".into(),
    };
    let mut s = format!("q {couleur}\n");
    if f.aplat {
        let b = mise.rectangles.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, r| [b[0].min(r[0]), b[1].min(r[1]), b[2].max(r[0] + r[2]), b[3].max(r[1] + r[3])]);
        let (x0, y0) = (b[0] - d, b[1] - d);
        s += &format!("{x0:.4} {y0:.4} {:.4} {:.4} re ", b[2] - b[0] + 2.0 * d, b[3] - b[1] + 2.0 * d);
        s += &format!("{:.4} {:.4} {:.4} {:.4} re f*\n", mise.decalage.0 - d, mise.decalage.1 - d, l + 2.0 * d, h + 2.0 * d);
    } else {
        for [x, y, rl, rh] in &mise.rectangles {
            s += &format!("{:.4} {:.4} {:.4} {:.4} re f\n", x - d, y - d, rl + 2.0 * d, rh + 2.0 * d);
        }
    }
    s + "Q\n"
}

/// Ressources de la page (les siennes, sinon celles héritées) recopiées sur elle, avec le ton
/// `White` du fond (`/FondRep`) et l'état graphique de surimpression des repères (`/GSRep`).
fn ajouter_ressources(doc: &mut Document, page: lopdf::ObjectId) -> Result<(), String> {
    let (direct, ids) = doc.get_page_resources(page).map_err(|e| e.to_string())?;
    let mut res = match (direct, ids.first()) {
        (Some(d), _) => d.clone(),
        (None, Some(id)) => doc.get_dictionary(*id).map_err(|e| e.to_string())?.clone(),
        (None, None) => lopdf::Dictionary::new(),
    };
    let sous = |doc: &Document, res: &lopdf::Dictionary, cle: &[u8]| -> lopdf::Dictionary {
        match res.get(cle) {
            Ok(Object::Reference(id)) => doc.get_dictionary(*id).cloned().unwrap_or_default(),
            Ok(Object::Dictionary(d)) => d.clone(),
            _ => lopdf::Dictionary::new(),
        }
    };
    let mut espaces = sous(doc, &res, b"ColorSpace");
    espaces.set(
        "FondRep",
        Object::Array(vec![
            "Separation".into(),
            "White".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![Object::Real(0.2), 0.into(), 0.into(), 0.into()], "N" => 1 }),
        ]),
    );
    let mut etats = sous(doc, &res, b"ExtGState");
    etats.set("GSRep", dictionary! { "Type" => "ExtGState", "OP" => true, "op" => true, "OPM" => 1 });
    res.set("ColorSpace", espaces);
    res.set("ExtGState", etats);
    doc.get_dictionary_mut(page).map_err(|e| e.to_string())?.set("Resources", res);
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn cadre_simple_long_devient_multi_cadre() {
        // Manuel Kona § 6.3 : multi-cadre dès 1,5 m (sens d'avance = hauteur de page).
        assert_eq!(effectif(Reperage::CadreCode, 1499.0 * MM), Reperage::CadreCode);
        assert_eq!(effectif(Reperage::CadreCode, 1534.0 * MM), Reperage::CadreMulti);
        assert_eq!(effectif(Reperage::Cadre, 3000.0 * MM), Reperage::Cadre, "Type 1 : pas de code, pas de multi");
        let simple = mise_en_page(Reperage::CadreCode, 370.0 * MM, 1534.0 * MM, 1);
        let multi = mise_en_page(effectif(Reperage::CadreCode, 1534.0 * MM), 370.0 * MM, 1534.0 * MM, 1);
        assert_eq!(multi.reperage, Reperage::CadreMulti);
        assert!(multi.rectangles.len() > simple.rectangles.len(), "repères intermédiaires ajoutés : {} > {}", multi.rectangles.len(), simple.rectangles.len());
        assert_eq!(Reperage::CadreMulti.code_cutserver(), Some("MF"));
    }
    use super::*;

    const MM: f64 = PT_PAR_MM;

    #[test]
    fn cadre_aux_cotes_du_manuel() {
        // Image de 100 × 60 mm : plus petite que l'intérieur minimal de 250 × 250 mm.
        let m = mise_en_page(Reperage::Cadre, 100.0 * MM, 60.0 * MM, 0);
        let [bas, haut, gauche, droite] = [m.rectangles[0], m.rectangles[1], m.rectangles[2], m.rectangles[3]];
        let mm = |v: f64| v / MM;
        assert!((mm(bas[3]) - cadre::EPAISSEUR).abs() < 1e-9 && mm(bas[3]) >= 3.0);
        // Intérieur ≥ 250 mm dans les deux sens.
        let int_l = mm(droite[0] - (gauche[0] + gauche[2]));
        let int_h = mm(haut[1] - (bas[1] + bas[3]));
        assert!(int_l >= 250.0 - 1e-9 && int_h >= 250.0 - 1e-9, "{int_l} × {int_h}");
        // Image centrée, à au moins 6 mm du cadre.
        let (ix, iy) = (mm(m.decalage.0), mm(m.decalage.1));
        assert!(ix - mm(gauche[0] + gauche[2]) >= 6.0 && iy - mm(bas[1] + bas[3]) >= 6.0);
        assert!((ix - mm(gauche[0] + gauche[2]) - (mm(droite[0]) - (ix + 100.0))).abs() < 1e-6, "centrée en largeur");
        // Blancs autour du cadre : 20 mm sur les côtés, 25 à l'avant, 75 à l'arrière.
        assert!(mm(gauche[0]) >= 20.0 && mm(m.page.0 - (droite[0] + droite[2])) >= 20.0);
        assert!(mm(bas[1]) >= 25.0 && mm(m.page.1 - (haut[1] + haut[3])) >= 75.0);
    }

    /// Le code de l'exemple du guide (4608925061 0), décodé à l'œil : 62 barres, contrôle 9.
    #[test]
    fn postnet_de_l_exemple_du_guide() {
        let barres = postnet(46089250610, 0.0, 0.0);
        let motif: String = barres.iter().map(|b| if b[3] > code::HAUTEUR * MM * 0.5 { '1' } else { '0' }).collect();
        assert_eq!(motif, "10100101100110001001010100001010101011000011000001111000101001");
        let largeur = (barres[61][0] + barres[61][2]) / MM;
        assert!((largeur - code::LARGEUR).abs() < 1e-9 && largeur >= 246.0);
        assert!(barres.iter().all(|b| b[3] <= 20.0 * MM + 1e-9));
    }

    #[test]
    fn cadres_types_2_et_3_aux_cotes_du_guide() {
        let n = numero(uuid::Uuid::from_u128(123_456_789_012), 7);
        assert_eq!(format!("{n:011}"), "45678901207");
        for r in [Reperage::CadreCode, Reperage::CadreMulti] {
            let m = mise_en_page(r, 100.0 * MM, 60.0 * MM, n);
            let e = cadre::EPAISSEUR * MM;
            let (x0, y0) = (cadre::BORD_COTE * MM, cadre::BORD_AVANT * MM);
            // Codes : barres de 20 mm au plus, pied à 6–12 mm du cadre, à 40–100 mm du côté.
            let hautes: Vec<_> = m.rectangles.iter().filter(|b| (b[3] / MM - 20.0).abs() < 1e-9 && b[2] < 3.0 * MM).collect();
            // Deux codes : 2 barres d'encadrement et 2 hautes par chiffre (11 et le contrôle).
            assert_eq!(hautes.len(), 2 * (2 + 2 * 12), "{r:?}");
            let bas = hautes.iter().map(|b| b[1]).fold(f64::MAX, f64::min);
            assert!(((bas - (y0 + e)) / MM - code::AU_CADRE).abs() < 1e-9 && (6.0..=12.0).contains(&code::AU_CADRE));
            // Bas : depuis le côté gauche ; haut : jusqu'au côté droit (mêmes marges, en miroir).
            let (dessous, dessus): (Vec<&&[f64; 4]>, Vec<&&[f64; 4]>) = hautes.iter().partition(|b| b[1] < m.decalage.1);
            let debut = dessous.iter().map(|b| b[0]).fold(f64::MAX, f64::min);
            let fin = dessus.iter().map(|b| b[0] + b[2]).fold(f64::MIN, f64::max);
            let droite_int = m.page.0 - x0 - e;
            assert!((40.0..=100.0).contains(&((debut - (x0 + e)) / MM)) && (40.0..=100.0).contains(&((droite_int - fin) / MM)));
            let haut_int = m.page.1 - cadre::BORD_ARRIERE * MM - e;
            let sommet = dessus.iter().map(|b| b[1] + b[3]).fold(f64::MIN, f64::max);
            assert!(((haut_int - sommet) / MM - code::AU_CADRE).abs() < 1e-9);
            // L'image ne touche pas les codes : elle commence au-dessus de la bande du bas.
            assert!(m.decalage.1 >= bas + 20.0 * MM + cadre::MARGE_IMAGE * MM - 1e-9);
            // Intérieur : 250 mm au moins, et la place du code.
            let int_l = m.page.0 - 2.0 * (x0 + e);
            assert!(int_l / MM >= code::DECALAGE + code::LARGEUR);
        }
        // Type 3 : angles de 125 mm, au moins un repère de 10–60 mm par côté.
        let m = mise_en_page(Reperage::CadreMulti, 100.0 * MM, 60.0 * MM, n);
        let verticaux: Vec<f64> = m.rectangles.iter().filter(|b| (b[2] / MM - cadre::EPAISSEUR).abs() < 1e-9 && b[3] > b[2]).map(|b| b[3] / MM).collect();
        assert_eq!(verticaux.iter().filter(|&&h| (h - 125.0).abs() < 1e-9).count(), 4);
        assert!(verticaux.iter().filter(|&&h| (10.0..=60.0).contains(&h)).count() >= 2);
    }

    /// La flèche d'insertion est dans le cadre, sous l'image, pointe vers le haut.
    #[test]
    fn fleche_d_insertion_dans_le_coin_bas_droit() {
        let m = mise_en_page(Reperage::Cadre, 100.0 * MM, 60.0 * MM, 0);
        let f: Vec<_> = m.rectangles[4..].to_vec();
        assert!(f.len() > 10 && f.windows(2).all(|w| w[1][2] < w[0][2] && w[1][1] > w[0][1]), "s'affine en montant : pointe en haut");
        let bas_interieur = (cadre::BORD_AVANT + cadre::EPAISSEUR) * MM;
        assert!((f[0][1] - bas_interieur - 3.0 * MM).abs() < 1e-9 && f.last().unwrap()[1] < m.decalage.1, "sous l'image, dans la marge");
    }

    #[test]
    fn grande_image_garde_sa_marge() {
        let m = mise_en_page(Reperage::Cadre, 1000.0 * MM, 600.0 * MM, 0);
        let gauche = m.rectangles[2];
        assert!(((m.decalage.0 - (gauche[0] + gauche[2])) / MM - cadre::MARGE_IMAGE).abs() < 1e-9);
    }

    #[test]
    fn croix_aux_quatre_coins() {
        let m = mise_en_page(Reperage::Manuel, 100.0 * MM, 50.0 * MM, 0);
        assert_eq!(m.rectangles.len(), 8, "deux barres par croix");
        let centres: Vec<(f64, f64)> = m.rectangles.chunks(2).map(|c| (c[0][0] + c[0][2] / 2.0, c[0][1] + c[0][3] / 2.0)).collect();
        // Rectangle des croix = image + 10 mm de chaque côté.
        let l = (centres[1].0 - centres[0].0) / MM;
        let h = (centres[2].1 - centres[1].1) / MM;
        assert!((l - 120.0).abs() < 1e-9 && (h - 70.0).abs() < 1e-9, "{l} × {h}");
    }

    /// 4.15 : fond blanc sous les repères, débord de 3 mm : sous le cadre (le noir reste
    /// plein, posé en surimpression) et jusqu'à 3 mm autour, rien au-delà ni dans l'image ;
    /// en aplat, toute la zone des repères hors de l'image (et de 3 mm autour) ; en CMJN, sur
    /// la plaque Yellow.
    #[test]
    fn fond_sous_les_reperes() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-fond-reperes-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let mut doc = Document::with_version("1.4");
        let contenu = doc.add_object(Stream::new(dictionary! {}, b"1 0 0 0 k 0 0 100 100 re f".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()], "Contents" => contenu });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        let dpi = 72.0;
        let rendre = |fond: FondReperes, nom: &str| {
            let (pdf, mut mises) = envelopper(&octets, Reperage::Cadre, uuid::Uuid::nil(), Some(fond)).unwrap();
            std::fs::write(dossier.join(format!("{nom}.pdf")), &pdf).unwrap();
            let st = std::process::Command::new("gs")
                .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r72"])
                .arg(format!("-sOutputFile={}", dossier.join(format!("{nom}.tif")).display()))
                .arg(dossier.join(format!("{nom}.pdf")))
                .status()
                .unwrap();
            assert!(st.success());
            mises.remove(0)
        };
        let lire = |nom: &str, plaque: &str, x: f64, y: f64| {
            let img = image::open(dossier.join(format!("{nom}({plaque}).tif"))).unwrap().to_luma8();
            img.get_pixel((x * dpi / 72.0) as u32, img.height() - 1 - (y * dpi / 72.0) as u32)[0]
        };
        let mm = PT_PAR_MM;
        let bande = FondReperes { debord_mm: 3.0, aplat: false, cmjn: None };
        let mise = rendre(bande, "bande");
        // Trait bas du cadre : x0, y0, l, e.
        let [x0, y0, l, e] = mise.rectangles[0];
        let cx = x0 + l / 2.0;
        assert_eq!(lire("bande", "White", cx, y0 + e / 2.0), 0, "blanc sous le cadre");
        assert_eq!(lire("bande", "Black", cx, y0 + e / 2.0), 0, "cadre toujours en noir plein");
        assert_eq!(lire("bande", "White", cx, y0 - 1.5 * mm), 0, "blanc à 1,5 mm du cadre");
        assert_eq!(lire("bande", "White", cx, y0 - 4.5 * mm), 255, "rien au-delà du débord");
        let (ix, iy) = (mise.decalage.0 + 50.0, mise.decalage.1 + 50.0);
        assert_eq!(lire("bande", "White", ix, iy), 255, "rien dans l'image");
        assert_eq!(lire("bande", "Cyan", ix, iy), 0);
        // Aplat : entre l'image et le cadre (marge de 10 mm), à 6 mm de l'image.
        rendre(FondReperes { aplat: true, ..bande }, "aplat");
        assert_eq!(lire("aplat", "White", mise.decalage.0 - 6.0 * mm, iy), 0, "aplat entre l'image et le cadre");
        assert_eq!(lire("aplat", "White", mise.decalage.0 - 1.5 * mm, iy), 255, "l'image et 3 mm autour restent vierges");
        assert_eq!(lire("aplat", "White", ix, iy), 255);
        // CMJN : jaune, aucune plaque White.
        rendre(FondReperes { cmjn: Some([0.0, 0.0, 1.0, 0.0]), ..bande }, "jaune");
        assert_eq!(lire("jaune", "Yellow", cx, y0 + e / 2.0), 0, "jaune sous le cadre (surimpression)");
        assert!(!dossier.join("jaune(White).tif").exists());
        assert_eq!(FondReperes { debord_mm: 20.0, ..bande }.verifier(), Err("fond_reperes_invalide"));
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Le cadre sort sur la seule plaque noire, et l'image d'origine est décalée d'autant.
    #[test]
    fn rendu_du_cadre_par_ghostscript() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-reperage-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        // PDF de 100 × 100 pt avec un carré cyan plein.
        let mut doc = Document::with_version("1.4");
        let contenu = doc.add_object(Stream::new(dictionary! {}, b"1 0 0 0 k 0 0 100 100 re f".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 100.into(), 100.into()], "Contents" => contenu });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();

        let (pdf, mut mises) = envelopper(&octets, Reperage::Cadre, uuid::Uuid::nil(), None).unwrap();
        let mise = mises.remove(0);
        let chemin = dossier.join("p.pdf");
        std::fs::write(&chemin, &pdf).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r18"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(&chemin)
            .status()
            .unwrap();
        assert!(st.success());
        let lire = |plaque: &str, x: f64, y: f64| {
            let img = image::open(dossier.join(format!("r({plaque}).tif"))).unwrap().to_luma8();
            let k = 18.0 / 72.0;
            img.get_pixel((x * k) as u32, img.height() - 1 - (y * k) as u32)[0]
        };
        let bas = mise.rectangles[0];
        let (cx, cy) = (bas[0] + bas[2] / 2.0, bas[1] + bas[3] / 2.0);
        assert_eq!(lire("Black", cx, cy), 0, "cadre en noir plein");
        assert_eq!(lire("Cyan", cx, cy), 255, "cadre sans cyan");
        let (ix, iy) = (mise.decalage.0 + 50.0, mise.decalage.1 + 50.0);
        assert_eq!(lire("Cyan", ix, iy), 0, "carré cyan déplacé avec la page");
        assert_eq!(lire("Black", ix, iy), 255);
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
