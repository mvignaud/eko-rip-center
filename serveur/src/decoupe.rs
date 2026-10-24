//! Découpe : les tracés peints dans un ton de découpe (`CutContour`, `Thrucut`…) sont
//! extraits du PDF puis écrits en HP-GL pour le plotter.
//!
//! On lit le contenu de la page : état graphique (`q`/`Q`/`cm`), formulaires XObject,
//! chemins (`m l c v y h re`) et couleur courante. Seule une couleur en Séparation dont
//! le nom est un ton de découpe rend un chemin « à découper ».

use lopdf::{Dictionary, Document, Object, content::Content};

/// Tons de découpe reconnus, comparés sans casse, espaces, tirets ni soulignés.
const TONS_DECOUPE: &[&str] = &["cutcontour", "thrucut", "kisscut", "insidecut", "perforation", "creasing", "score", "vcut"];

/// Écart maximal entre une courbe et ses segments : 0,025 mm, en points PDF.
const TOLERANCE_PT: f64 = 0.025 * 72.0 / 25.4;

/// Unités HP-GL par point PDF : 100 par mm (pas de 0,010 mm). La découpe de contour du
/// Kona n'accepte que ce pas ; le menu « Resolution » du plotter doit être à 0,010 mm,
/// sinon tout sort 2,5 fois trop grand ou trop petit (manuel Kona, chap. 4).
const UNITES_PAR_PT: f64 = 100.0 * 25.4 / 72.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// Nom du ton tel qu'écrit dans le PDF.
    pub ton: String,
    /// Points en points PDF, origine au coin bas gauche de la page.
    pub points: Vec<(f64, f64)>,
    pub ferme: bool,
}

fn cle(nom: &str) -> String {
    nom.chars().filter(|c| !matches!(c, ' ' | '-' | '_')).flat_map(char::to_lowercase).collect()
}

pub fn est_ton_de_decoupe(nom: &str) -> bool {
    TONS_DECOUPE.contains(&cle(nom).as_str())
}

/// Matrice PDF `[a b c d e f]`.
type Matrice = [f64; 6];
const IDENTITE: Matrice = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` puis `n` (convention PDF : nouvelle CTM = `cm` × CTM).
fn produit(m: &Matrice, n: &Matrice) -> Matrice {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn transformer(m: &Matrice, (x, y): (f64, f64)) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

#[derive(Clone)]
struct Etat {
    ctm: Matrice,
    /// Ton de découpe de l'espace de couleur courant (trait, fond), qu'il soit encré ou non.
    espace_trait: Option<String>,
    espace_fond: Option<String>,
    /// Ton de découpe effectivement encré (teinte > 0).
    trait_: Option<String>,
    fond: Option<String>,
}

struct SousChemin {
    points: Vec<(f64, f64)>,
    ferme: bool,
}

struct Lecteur<'a> {
    doc: &'a Document,
    /// Mode « tous les tracés » : tout chemin peint est découpé, quelle que soit sa couleur.
    tous: bool,
    traces: Vec<Trace>,
}

fn nombres(operandes: &[Object]) -> Vec<f64> {
    operandes.iter().filter_map(|o| o.as_float().ok()).map(f64::from).collect()
}

/// Courbe de Bézier aplatie : nombre de segments tiré de l'écart au polygone de contrôle.
fn bezier(sortie: &mut Vec<(f64, f64)>, p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) {
    let ecart = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| ((a.0 - 2.0 * b.0 + c.0).powi(2) + (a.1 - 2.0 * b.1 + c.1).powi(2)).sqrt();
    let d = ecart(p0, p1, p2).max(ecart(p1, p2, p3));
    let n = ((0.75 * d / TOLERANCE_PT).sqrt().ceil() as usize).clamp(1, 1000);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        sortie.push((a * p0.0 + b * p1.0 + c * p2.0 + e * p3.0, a * p0.1 + b * p1.1 + c * p2.1 + e * p3.1));
    }
}

fn deref<'d>(doc: &'d Document, o: &'d Object) -> &'d Object {
    doc.dereference(o).map(|(_, o)| o).unwrap_or(o)
}

fn sous_dict<'d>(doc: &'d Document, ressources: Option<&'d Dictionary>, cle: &[u8]) -> Option<&'d Dictionary> {
    deref(doc, ressources?.get(cle).ok()?).as_dict().ok()
}

/// Ton de découpe d'un espace de couleur nommé : `[/Separation /CutContour …]`.
fn ton_espace(doc: &Document, ressources: Option<&Dictionary>, nom: &[u8]) -> Option<String> {
    let espaces = sous_dict(doc, ressources, b"ColorSpace")?;
    let tableau = deref(doc, espaces.get(nom).ok()?).as_array().ok()?;
    if deref(doc, tableau.first()?).as_name().ok()? != b"Separation" {
        return None;
    }
    let ton = String::from_utf8_lossy(deref(doc, tableau.get(1)?).as_name().ok()?).into_owned();
    est_ton_de_decoupe(&ton).then_some(ton)
}

impl<'a> Lecteur<'a> {
    fn lire(&mut self, contenu: &[u8], ressources: Option<&'a Dictionary>, etat_initial: Etat, profondeur: u32) -> Result<(), String> {
        let doc = self.doc;
        if profondeur > 20 {
            return Err("formulaires imbriqués trop profondément".into());
        }
        let operations = Content::decode(contenu).map_err(|e| format!("contenu de page illisible : {e}"))?.operations;
        let mut etat = etat_initial;
        let mut pile: Vec<Etat> = vec![];
        let mut chemin: Vec<SousChemin> = vec![];
        let mut courant = (0.0, 0.0);
        let mut debut = (0.0, 0.0);

        for op in &operations {
            let n = nombres(&op.operands);
            let pt = |i: usize| transformer(&etat.ctm, (n[i], n[i + 1]));
            match op.operator.as_str() {
                "q" => pile.push(etat.clone()),
                "Q" => etat = pile.pop().unwrap_or(etat),
                "cm" if n.len() == 6 => etat.ctm = produit(&[n[0], n[1], n[2], n[3], n[4], n[5]], &etat.ctm),
                "CS" | "cs" => {
                    let ton = op.operands.first().and_then(|o| o.as_name().ok()).and_then(|nom| ton_espace(doc, ressources, nom));
                    // Une Séparation commence à la teinte 1.
                    if op.operator == "CS" {
                        (etat.espace_trait, etat.trait_) = (ton.clone(), ton);
                    } else {
                        (etat.espace_fond, etat.fond) = (ton.clone(), ton);
                    }
                }
                "SC" | "SCN" => etat.trait_ = etat.espace_trait.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "sc" | "scn" => etat.fond = etat.espace_fond.clone().filter(|_| n.last().is_some_and(|t| *t > 0.0)),
                "G" | "RG" | "K" => (etat.espace_trait, etat.trait_) = (None, None),
                "g" | "rg" | "k" => (etat.espace_fond, etat.fond) = (None, None),
                "m" if n.len() == 2 => {
                    courant = pt(0);
                    debut = courant;
                    chemin.push(SousChemin { points: vec![courant], ferme: false });
                }
                "l" if n.len() == 2 => {
                    courant = pt(0);
                    if let Some(s) = chemin.last_mut() {
                        s.points.push(courant);
                    }
                }
                "c" | "v" | "y" if n.len() >= 4 => {
                    let (p1, p2, p3) = match op.operator.as_str() {
                        "c" if n.len() == 6 => (pt(0), pt(2), pt(4)),
                        "v" => (courant, pt(0), pt(2)),
                        "y" => (pt(0), pt(2), pt(2)),
                        _ => continue,
                    };
                    if let Some(s) = chemin.last_mut() {
                        bezier(&mut s.points, courant, p1, p2, p3);
                    }
                    courant = p3;
                }
                "h" => {
                    if let Some(s) = chemin.last_mut() {
                        s.ferme = true;
                    }
                    courant = debut;
                }
                "re" if n.len() == 4 => {
                    let (x, y, w, h) = (n[0], n[1], n[2], n[3]);
                    let coins = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|p| transformer(&etat.ctm, p));
                    chemin.push(SousChemin { points: coins.to_vec(), ferme: true });
                    (courant, debut) = (coins[0], coins[0]);
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    let remplissage = matches!(op.operator.as_str(), "f" | "F" | "f*");
                    let ton = match op.operator.as_str() {
                        "n" => None,
                        _ if self.tous => Some(TOUS.to_string()),
                        "S" | "s" => etat.trait_.clone(),
                        _ if remplissage => etat.fond.clone(),
                        _ => etat.trait_.clone().or_else(|| etat.fond.clone()),
                    };
                    // Un remplissage ou une fermeture (s, b) ferme chaque sous-chemin.
                    let fermer = op.operator != "S" && op.operator != "B" && op.operator != "B*";
                    if let Some(ton) = ton {
                        for s in chemin.drain(..).filter(|s| s.points.len() >= 2) {
                            self.traces.push(Trace { ton: ton.clone(), points: s.points, ferme: s.ferme || fermer });
                        }
                    }
                    chemin.clear();
                }
                "Do" => {
                    let Some(nom) = op.operands.first().and_then(|o| o.as_name().ok()) else { continue };
                    let Some(xobjets) = sous_dict(doc, ressources, b"XObject") else { continue };
                    let Ok(flux) = xobjets.get(nom).map(|o| deref(doc, o)).and_then(Object::as_stream) else { continue };
                    if flux.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Form".as_slice()) {
                        continue;
                    }
                    let matrice = flux
                        .dict
                        .get(b"Matrix")
                        .and_then(Object::as_array)
                        .map(|a| nombres(a))
                        .ok()
                        .filter(|m| m.len() == 6)
                        .map(|m| [m[0], m[1], m[2], m[3], m[4], m[5]])
                        .unwrap_or(IDENTITE);
                    let contenu = flux.decompressed_content().unwrap_or_else(|_| flux.content.clone());
                    let ressources_form = flux.dict.get(b"Resources").ok().and_then(|o| deref(doc, o).as_dict().ok()).or(ressources);
                    let mut etat_form = etat.clone();
                    etat_form.ctm = produit(&matrice, &etat.ctm);
                    self.lire(&contenu, ressources_form, etat_form, profondeur + 1)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Ton attribué aux tracés en mode « tous les tracés » (lame).
pub const TOUS: &str = "Tous";

/// Tracés à découper et format de la page, en points PDF.
pub struct Decoupe {
    pub traces: Vec<Trace>,
    pub largeur: f64,
    pub hauteur: f64,
}

impl Decoupe {
    /// Déplace les tracés comme le contenu de la page (repérage) et prend la nouvelle page.
    pub fn decaler(&mut self, (dx, dy): (f64, f64), (largeur, hauteur): (f64, f64)) {
        for t in &mut self.traces {
            for p in &mut t.points {
                *p = (p.0 + dx, p.1 + dy);
            }
        }
        (self.largeur, self.hauteur) = (largeur, hauteur);
    }
}

/// Tracés de découpe de chaque page d'un PDF (ou d'un fichier Illustrator enregistré
/// avec « Créer un fichier compatible PDF »), dans l'ordre des pages.
pub fn extraire(pdf: &[u8], tous: bool) -> Result<Vec<Decoupe>, String> {
    let doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let pages: Vec<_> = doc.get_pages().into_values().collect();
    if pages.is_empty() {
        return Err("PDF sans page".into());
    }
    pages.into_iter().enumerate().map(|(i, page)| extraire_page(&doc, page, tous).map_err(|e| format!("page {} : {e}", i + 1))).collect()
}

fn extraire_page(doc: &Document, page: lopdf::ObjectId, tous: bool) -> Result<Decoupe, String> {
    let dict_page = doc.get_dictionary(page).map_err(|e| e.to_string())?;
    let ressources = match doc.get_page_resources(page).map_err(|e| e.to_string())? {
        (Some(d), _) => Some(d),
        (None, ids) => ids.first().and_then(|id| doc.get_dictionary(*id).ok()),
    };
    // Origine au coin bas gauche de la MediaBox.
    let boite = dict_page
        .get(b"MediaBox")
        .and_then(Object::as_array)
        .map(|a| nombres(a))
        .ok()
        .filter(|m| m.len() == 4)
        .ok_or("page sans MediaBox")?;
    let ctm = [1.0, 0.0, 0.0, 1.0, -boite[0], -boite[1]];
    let etat = Etat { ctm, espace_trait: None, espace_fond: None, trait_: None, fond: None };

    let mut lecteur = Lecteur { doc, tous, traces: vec![] };
    lecteur.lire(&doc.get_page_content(page), ressources, etat, 0)?;
    let (largeur, hauteur) = (boite[2] - boite[0], boite[3] - boite[1]);
    Ok(Decoupe { traces: rogner(lecteur.traces, largeur, hauteur), largeur, hauteur })
}

/// Tracés d'un SVG. Sans couleur en Séparation dans ce format, un tracé est « de
/// découpe » quand son `id` ou celui d'un groupe parent porte un nom de ton : c'est le
/// nom du calque ou du tracé qu'Illustrator écrit à l'export.
pub fn extraire_svg(svg: &[u8], tous: bool) -> Result<Decoupe, String> {
    let arbre = usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|e| format!("SVG illisible : {e}"))?;
    // Unités SVG (1/96 pouce, axe y vers le bas) → points PDF (axe y vers le haut).
    let k = 72.0 / 96.0;
    let hauteur = arbre.size().height() as f64 * k;
    let vers_pt = |p: usvg::tiny_skia_path::Point| (p.x as f64 * k, hauteur - p.y as f64 * k);
    let mut traces = vec![];
    parcourir_svg(arbre.root(), None, tous, &vers_pt, &mut traces);
    Ok(Decoupe { traces, largeur: arbre.size().width() as f64 * k, hauteur })
}

fn parcourir_svg(groupe: &usvg::Group, ton_parent: Option<&str>, tous: bool, vers_pt: &dyn Fn(usvg::tiny_skia_path::Point) -> (f64, f64), traces: &mut Vec<Trace>) {
    let ton_groupe = Some(groupe.id()).filter(|id| est_ton_de_decoupe(id)).or(ton_parent);
    for noeud in groupe.children() {
        match noeud {
            usvg::Node::Group(g) => parcourir_svg(g, ton_groupe, tous, vers_pt, traces),
            usvg::Node::Path(chemin) if chemin.is_visible() => {
                let ton = Some(chemin.id()).filter(|id| est_ton_de_decoupe(id)).or(ton_groupe);
                let ton = if tous { TOUS } else if let Some(ton) = ton { ton } else { continue };
                let Some(donnees) = chemin.data().clone().transform(chemin.abs_transform()) else { continue };
                let mut courant = vec![];
                let mut depart = (0.0, 0.0);
                let mut position = (0.0, 0.0);
                let mut fermer = |points: &mut Vec<(f64, f64)>, ferme: bool| {
                    if points.len() >= 2 {
                        traces.push(Trace { ton: ton.to_string(), points: std::mem::take(points), ferme });
                    }
                    points.clear();
                };
                use usvg::tiny_skia_path::PathSegment as S;
                for segment in donnees.segments() {
                    match segment {
                        S::MoveTo(p) => {
                            fermer(&mut courant, false);
                            position = vers_pt(p);
                            depart = position;
                            courant.push(position);
                        }
                        S::LineTo(p) => {
                            position = vers_pt(p);
                            courant.push(position);
                        }
                        S::QuadTo(c, p) => {
                            // Quadratique en cubique : points de contrôle aux deux tiers.
                            let (c, p) = (vers_pt(c), vers_pt(p));
                            let c1 = (position.0 + 2.0 / 3.0 * (c.0 - position.0), position.1 + 2.0 / 3.0 * (c.1 - position.1));
                            let c2 = (p.0 + 2.0 / 3.0 * (c.0 - p.0), p.1 + 2.0 / 3.0 * (c.1 - p.1));
                            bezier(&mut courant, position, c1, c2, p);
                            position = p;
                        }
                        S::CubicTo(c1, c2, p) => {
                            let p = vers_pt(p);
                            bezier(&mut courant, position, vers_pt(c1), vers_pt(c2), p);
                            position = p;
                        }
                        S::Close => {
                            fermer(&mut courant, true);
                            position = depart;
                        }
                    }
                }
                fermer(&mut courant, false);
            }
            _ => {}
        }
    }
}

/// Aperçu des tracés à découper, à la taille de la page (1 unité = 1 point).
pub fn apercu_svg(d: &Decoupe) -> String {
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {l:.1} {h:.1}"><rect width="100%" height="100%" fill="#fff"/><g fill="none" stroke="#d62020" stroke-width="{e:.2}" stroke-linejoin="round">"##,
        l = d.largeur,
        h = d.hauteur,
        e = (d.largeur.max(d.hauteur) / 400.0).max(0.5)
    );
    for t in &d.traces {
        let points: Vec<String> = t.points.iter().map(|(x, y)| format!("{x:.1},{:.1}", d.hauteur - y)).collect();
        let balise = if t.ferme { "polygon" } else { "polyline" };
        svg += &format!(r#"<{balise} points="{}"/>"#, points.join(" "));
    }
    svg + "</g></svg>"
}

/// Segment `a`–`b` rogné au rectangle [0, l] × [0, h] (Liang–Barsky) ; `None` s'il est dehors.
fn rogner_segment(a: (f64, f64), b: (f64, f64), l: f64, h: f64) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [(-dx, a.0), (dx, l - a.0), (-dy, a.1), (dy, h - a.1)] {
        if p.abs() < 1e-12 {
            if q < -1e-9 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some(((a.0 + t0 * dx, a.1 + t0 * dy), (a.0 + t1 * dx, a.1 + t1 * dy)))
}

/// Tracés rognés à la page [0, l] × [0, h] : un tracé qui déborde devient un ou plusieurs
/// tracés ouverts (la lame ne coupe pas hors du support) ; un tracé fermé coupé raccorde
/// son dernier morceau au premier.
pub fn rogner(traces: Vec<Trace>, l: f64, h: f64) -> Vec<Trace> {
    let pres = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6;
    let mut sortie = vec![];
    for t in traces {
        let mut points = t.points.clone();
        if t.ferme {
            points.push(t.points[0]);
        }
        let mut morceaux: Vec<Vec<(f64, f64)>> = vec![];
        let mut coupe = false;
        for w in points.windows(2) {
            let Some((a, b)) = rogner_segment(w[0], w[1], l, h) else {
                coupe = true;
                continue;
            };
            coupe |= !pres(a, w[0]) || !pres(b, w[1]);
            match morceaux.last_mut() {
                Some(m) if pres(*m.last().unwrap(), a) => m.push(b),
                _ => morceaux.push(vec![a, b]),
            }
        }
        if !coupe && t.ferme {
            sortie.push(t);
            continue;
        }
        // Tracé fermé coupé : le dernier morceau rejoint le premier s'ils se touchent.
        if t.ferme && morceaux.len() > 1 && pres(*morceaux.last().unwrap().last().unwrap(), morceaux[0][0]) {
            let mut fin = morceaux.pop().unwrap();
            fin.extend(morceaux.remove(0).into_iter().skip(1));
            morceaux.insert(0, fin);
        }
        sortie.extend(morceaux.into_iter().filter(|m| m.len() >= 2).map(|points| Trace { ton: t.ton.clone(), points, ferme: false }));
    }
    sortie
}

/// Outil du plotter selon le ton : 1 lame, 2 rainage, 3 perforation, 4 coupe en V.
fn outil(ton: &str) -> u8 {
    match cle(ton).as_str() {
        "creasing" | "score" => 2,
        "perforation" => 3,
        "vcut" => 4,
        _ => 1,
    }
}

/// Fichier HP-GL : un `SP` par outil, chaque tracé en `PU` puis `PD`, unités 1/100 mm.
pub fn hpgl(traces: &[Trace]) -> String {
    let u = |(x, y): (f64, f64)| ((x * UNITES_PAR_PT).round() as i64, (y * UNITES_PAR_PT).round() as i64);
    let mut sortie = String::from("IN;");
    let mut outils: Vec<u8> = traces.iter().map(|t| outil(&t.ton)).collect();
    outils.sort();
    outils.dedup();
    for o in outils {
        sortie += &format!("SP{o};");
        for t in traces.iter().filter(|t| outil(&t.ton) == o) {
            let (x0, y0) = u(t.points[0]);
            sortie += &format!("PU{x0},{y0};PD");
            let mut suite: Vec<String> = t.points[1..].iter().map(|p| u(*p)).map(|(x, y)| format!("{x},{y}")).collect();
            if t.ferme {
                suite.push(format!("{x0},{y0}"));
            }
            sortie += &suite.join(",");
            sortie += ";";
        }
    }
    sortie += "PU;SP0;";
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Stream, dictionary};

    fn separation(ton: &str) -> Object {
        Object::Array(vec![
            "Separation".into(),
            Object::Name(ton.as_bytes().to_vec()),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ])
    }

    /// Page : un formulaire tourné de 90° puis décalé, qui découpe un carré de 72 pt, un trait
    /// `Thrucut`, un fond `Silver` et un trait noir (ignorés), un cercle `cut contour`.
    fn pdf_de_test() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let espaces = dictionary! { "CS0" => separation("CutContour"), "CS1" => separation("Silver"), "CS2" => separation("Thrucut"), "CS3" => separation("cut contour") };
        let espaces_id = doc.add_object(espaces);
        let form = Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(), 0.into(), 72.into(), 72.into()], "Matrix" => vec![0.into(), 1.into(), (-1).into(), 0.into(), 100.into(), 0.into()], "Resources" => dictionary! { "ColorSpace" => espaces_id } },
            b"/CS0 CS 1 SCN 0 0 72 72 re S".to_vec(),
        );
        let form_id = doc.add_object(form);
        // Cercle de rayon 50 centré en (400, 300), en quatre arcs de Bézier.
        let k = 0.5523 * 50.0;
        let cercle = format!(
            "/CS3 CS 1 SCN 450 300 m 450 {a} {b} 350 400 350 c {c} 350 350 {a} 350 300 c 350 {d} {c} 250 400 250 c {b} 250 450 {d} 450 300 c S",
            a = 300.0 + k, b = 400.0 + k, c = 400.0 - k, d = 300.0 - k
        );
        let contenu = format!(
            "q 1 0 0 1 0 50 cm /Fm0 Do Q /CS1 cs 1 scn 0 0 10 10 re f /CS2 CS 1 SCN 200 0 m 200 72 l S /CS2 CS 0 SCN 210 0 m 210 72 l S 0 0 0 1 K 300 0 m 300 72 l S {cercle}"
        );
        let contenu_id = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "MediaBox" => vec![0.into(), 0.into(), 600.into(), 600.into()],
            "Contents" => contenu_id, "Resources" => dictionary! { "ColorSpace" => espaces_id, "XObject" => dictionary! { "Fm0" => form_id } },
        });
        doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1 }));
        let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalogue);
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        octets
    }

    #[test]
    fn noms_de_tons() {
        assert!(est_ton_de_decoupe("CutContour"));
        assert!(est_ton_de_decoupe("cut contour"));
        assert!(est_ton_de_decoupe("Kiss-Cut"));
        assert!(!est_ton_de_decoupe("White"));
        assert!(!est_ton_de_decoupe("Silver"));
    }

    #[test]
    fn seuls_les_tons_de_decoupe_encres_sont_extraits() {
        let traces = extraire(&pdf_de_test(), false).unwrap().remove(0).traces;
        let tons: Vec<&str> = traces.iter().map(|t| t.ton.as_str()).collect();
        assert_eq!(tons, ["CutContour", "Thrucut", "cut contour"], "Silver, trait noir et Thrucut à 0 % ignorés");

        // Formulaire : Matrix (rotation de 90°, puis +100 en x) PUIS cm (+50 en y) ;
        // l'ordre compte, une rotation ne commute pas avec une translation.
        let carre = &traces[0];
        assert!(carre.ferme);
        assert_eq!(carre.points, [(100.0, 50.0), (100.0, 122.0), (28.0, 122.0), (28.0, 50.0)]);

        let trait_ = &traces[1];
        assert!(!trait_.ferme);
        assert_eq!(trait_.points, [(200.0, 0.0), (200.0, 72.0)]);

        let cercle = &traces[2];
        assert!(cercle.points.len() > 20, "courbes aplaties en {} points", cercle.points.len());
        for (x, y) in &cercle.points {
            let r = ((x - 400.0).powi(2) + (y - 300.0).powi(2)).sqrt();
            assert!((r - 50.0).abs() < 0.1, "point hors du cercle : rayon {r}");
        }
    }

    #[test]
    fn mode_tous_les_traces() {
        let d = extraire(&pdf_de_test(), true).unwrap().remove(0);
        // Carré, fond Silver, deux traits Thrucut (même à 0 %), trait noir, cercle.
        assert_eq!(d.traces.len(), 6);
        assert!(d.traces.iter().all(|t| t.ton == TOUS));
        assert_eq!((d.largeur, d.hauteur), (600.0, 600.0));
    }

    /// 200 × 100 mm (viewBox en mm), un calque `CutContour` avec un rectangle déplacé et
    /// un cercle dans un autre calque.
    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200mm" height="100mm" viewBox="0 0 200 100">
        <g id="CutContour"><rect x="10" y="10" width="50" height="20" transform="translate(5 0)"/></g>
        <g id="Visuel"><circle cx="150" cy="50" r="25" fill="red"/></g>
    </svg>"#;

    #[test]
    fn svg_calque_de_decoupe() {
        let d = extraire_svg(SVG.as_bytes(), false).unwrap();
        let mm = |pt: f64| pt * 25.4 / 72.0;
        assert!((mm(d.largeur) - 200.0).abs() < 0.01 && (mm(d.hauteur) - 100.0).abs() < 0.01, "format {}×{}", mm(d.largeur), mm(d.hauteur));
        assert_eq!(d.traces.len(), 1, "seul le calque CutContour");
        let t = &d.traces[0];
        assert_eq!(t.ton, "CutContour");
        assert!(t.ferme);
        // Rectangle de 15 à 65 mm en x ; en y, 10–30 mm depuis le haut = 70–90 mm depuis le bas.
        let (xs, ys): (Vec<f64>, Vec<f64>) = t.points.iter().map(|(x, y)| (mm(*x), mm(*y))).unzip();
        let bornes = |v: &[f64]| (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max));
        let ((x0, x1), (y0, y1)) = (bornes(&xs), bornes(&ys));
        for (obtenu, attendu) in [(x0, 15.0), (x1, 65.0), (y0, 70.0), (y1, 90.0)] {
            assert!((obtenu - attendu).abs() < 0.01, "{obtenu} au lieu de {attendu}");
        }
    }

    #[test]
    fn svg_tous_les_traces() {
        let d = extraire_svg(SVG.as_bytes(), true).unwrap();
        assert_eq!(d.traces.len(), 2);
        let cercle = &d.traces[1];
        let (cx, cy, r) = (150.0 * 72.0 / 25.4, 50.0 * 72.0 / 25.4, 25.0 * 72.0 / 25.4);
        for (x, y) in &cercle.points {
            assert!((((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r).abs() < 0.1);
        }
    }

    #[test]
    fn rognage_a_la_page() {
        let carre = |x0: f64, x1: f64| Trace { ton: "CutContour".into(), points: vec![(x0, 10.0), (x1, 10.0), (x1, 90.0), (x0, 90.0)], ferme: true };
        // Dedans : inchangé, reste fermé.
        assert_eq!(rogner(vec![carre(10.0, 50.0)], 100.0, 100.0), vec![carre(10.0, 50.0)]);
        // Dehors : disparaît.
        assert!(rogner(vec![carre(150.0, 190.0)], 100.0, 100.0).is_empty());
        // À cheval sur le bord droit : un seul tracé ouvert, raccordé, de (100,90) à (100,10).
        let r = rogner(vec![carre(50.0, 150.0)], 100.0, 100.0);
        assert_eq!(r.len(), 1, "{r:?}");
        assert!(!r[0].ferme);
        assert_eq!(r[0].points, [(100.0, 90.0), (50.0, 90.0), (50.0, 10.0), (100.0, 10.0)]);
    }

    #[test]
    fn hpgl_en_centiemes_de_millimetre() {
        let traces = vec![
            Trace { ton: "CutContour".into(), points: vec![(0.0, 0.0), (72.0, 0.0), (72.0, 72.0)], ferme: true },
            Trace { ton: "Creasing".into(), points: vec![(0.0, 0.0), (0.0, 72.0)], ferme: false },
        ];
        // 72 pt = 25,4 mm = 2540 unités.
        assert_eq!(hpgl(&traces), "IN;SP1;PU0,0;PD2540,0,2540,2540,0,0;SP2;PU0,0;PD0,2540;PU;SP0;");
    }
}
