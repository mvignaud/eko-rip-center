//! Imbrication : plusieurs pages (ou copies d'une page) placées côte à côte sur la laize
//! du rouleau, en un seul PDF. Chaque page source devient un formulaire XObject placé
//! par sa matrice ; le résultat suit ensuite le circuit ordinaire (rendu, repérage,
//! découpe — l'extracteur de découpe lit les formulaires).

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use serde::Deserialize;

pub const PT_PAR_MM: f64 = 72.0 / 25.4;

/// Un élément à placer : largeur et hauteur en points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Taille {
    pub largeur: f64,
    pub hauteur: f64,
}

/// Position (coin bas gauche de la case) de l'élément `indice`, en points, et son angle :
/// 0, 90, 180 ou 270° vers la gauche (à 90 et 270°, sa largeur devient sa hauteur).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub indice: usize,
    pub x: f64,
    pub y: f64,
    pub angle: u16,
}

impl Place {
    /// Quart de tour (90 ou 270°) : largeur et hauteur échangées.
    pub fn tourne(&self) -> bool {
        self.angle % 180 == 90
    }
}

/// Matrice PDF qui pose une page de taille `t` (MediaBox commençant en `ll`) tournée de
/// `angle` dans la case dont le coin bas gauche est (x, y).
pub fn matrice_pose(t: Taille, [llx, lly]: [f64; 2], angle: u16, x: f64, y: f64) -> [f64; 6] {
    let (w, h) = (t.largeur, t.hauteur);
    match angle {
        90 => [0.0, 1.0, -1.0, 0.0, x + lly + h, y - llx],
        180 => [-1.0, 0.0, 0.0, -1.0, x + llx + w, y + lly + h],
        270 => [0.0, -1.0, 1.0, 0.0, x - lly, y + llx + w],
        _ => [1.0, 0.0, 0.0, 1.0, x - llx, y - lly],
    }
}

/// Placement en rangées : éléments par hauteur décroissante, de gauche à droite, nouvelle
/// rangée quand la laize est pleine. Avec `rotation`, quatre orientations sont essayées —
/// tel quel, tout tourné, tout couché, tout debout — et la plus courte en longueur de
/// rouleau est gardée ; un élément trop large n'y passe que tourné. Rend les places et la
/// taille de la planche.
// ponytail: rangées simples ; un placement en « skyline » ou sur la forme du contour
// gagnerait du support sur des formats très mélangés.
pub fn placer(tailles: &[Taille], laize: f64, espace: f64, rotation: bool) -> Result<(Vec<Place>, Taille), String> {
    let tient = |l: f64| l <= laize + 1e-6;
    if let Some(i) = tailles.iter().position(|t| !tient(t.largeur) && !(rotation && tient(t.hauteur))) {
        let t = tailles[i];
        return Err(format!("l'élément {} ({:.0} × {:.0} mm) est plus large que la laize ({:.0} mm)", i + 1, t.largeur / PT_PAR_MM, t.hauteur / PT_PAR_MM, laize / PT_PAR_MM));
    }
    // Orientation de chaque élément selon une règle ; un élément trop large est toujours tourné.
    let selon = |regle: &dyn Fn(&Taille) -> bool| -> Vec<bool> { tailles.iter().map(|t| !tient(t.largeur) || (tient(t.hauteur) && regle(t))).collect() };
    let tel_quel = selon(&|_| false);
    let mut meilleur = rangees(tailles, &tel_quel, laize, espace, true);
    if !rotation {
        return Ok(meilleur);
    }
    // Tout tourné, côté long en travers (couché), côté long dans le sens du rouleau (debout).
    for tourne in [selon(&|_| true), selon(&|t| t.hauteur > t.largeur), selon(&|t| t.largeur > t.hauteur)] {
        let essai = rangees(tailles, &tourne, laize, espace, true);
        if essai.1.hauteur < meilleur.1.hauteur - 1e-6 {
            meilleur = essai;
        }
    }
    Ok(meilleur)
}

/// Placement « groupé dans l'ordre » (roadmap 5.1) : en rangées, dans l'ordre donné, sans
/// rotation ni réordonnancement ; refus si un élément est plus large que la laize.
pub fn placer_en_ordre(tailles: &[Taille], laize: f64, espace: f64) -> Result<(Vec<Place>, Taille), String> {
    if let Some(i) = tailles.iter().position(|t| t.largeur > laize + 1e-6) {
        return Err(format!("l'élément {} ({:.0} mm) est plus large que la laize ({:.0} mm)", i + 1, tailles[i].largeur / PT_PAR_MM, laize / PT_PAR_MM));
    }
    Ok(rangees(tailles, &vec![false; tailles.len()], laize, espace, false))
}

fn rangees(tailles: &[Taille], tourne: &[bool], laize: f64, espace: f64, trier: bool) -> (Vec<Place>, Taille) {
    let pose = |i: usize| if tourne[i] { Taille { largeur: tailles[i].hauteur, hauteur: tailles[i].largeur } } else { tailles[i] };
    let mut ordre: Vec<usize> = (0..tailles.len()).collect();
    if trier {
        ordre.sort_by(|&a, &b| pose(b).hauteur.total_cmp(&pose(a).hauteur));
    }
    let (mut places, mut x, mut y, mut haut_rangee, mut large_max) = (vec![], 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for i in ordre {
        let t = pose(i);
        if x > 0.0 && x + t.largeur > laize + 1e-6 {
            y += haut_rangee + espace;
            (x, haut_rangee) = (0.0, 0.0);
        }
        places.push(Place { indice: i, x, y, angle: if tourne[i] { 90 } else { 0 } });
        large_max = large_max.max(x + t.largeur);
        haut_rangee = haut_rangee.max(t.hauteur);
        x += t.largeur + espace;
    }
    (places, Taille { largeur: large_max, hauteur: y + haut_rangee })
}

/// Taille (points) de la page `page` (à partir de 1) d'un PDF, d'après sa MediaBox.
pub fn taille_page(pdf: &[u8], page: usize) -> Result<Taille, String> {
    let doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let id = *doc.get_pages().get(&(page as u32)).ok_or(format!("pas de page {page}"))?;
    let b = doc.get_dictionary(id).map_err(|e| e.to_string())?.get(b"MediaBox").ok().and_then(nombres).filter(|b| b.len() == 4).ok_or("page sans MediaBox")?;
    Ok(Taille { largeur: b[2] - b[0], hauteur: b[3] - b[1] })
}

/// Ramène chaque page à la seule géométrie que lisent l'édition, le repérage, l'imbrication
/// et la découpe : sa MediaBox, sans /Rotate. Ghostscript, lui, rogne l'impression à la
/// CropBox et tourne la page selon /Rotate : sans cela, la coupe tombait à côté de l'encre.
/// La MediaBox devient son intersection avec la CropBox ; /Rotate passe dans le contenu ;
/// ces attributs, héritables, sont écrits sur chaque page et retirés de l'arbre des pages.
/// `None` : rien à changer.
pub fn normaliser_pages(pdf: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let herite = |doc: &Document, id: ObjectId, cle: &[u8]| -> Option<Object> {
        let mut d = doc.get_dictionary(id).ok()?;
        loop {
            if let Ok(v) = d.get(cle) {
                return doc.dereference(v).ok().map(|(_, o)| o.clone());
            }
            d = doc.get_dictionary(d.get(b"Parent").and_then(Object::as_reference).ok()?).ok()?;
        }
    };
    let rectangle = |o: Option<Object>| o.as_ref().and_then(nombres).filter(|b| b.len() == 4).map(|b| [b[0].min(b[2]), b[1].min(b[3]), b[0].max(b[2]), b[1].max(b[3])]);
    let mut change = false;
    for (n, id) in doc.get_pages() {
        let media = rectangle(herite(&doc, id, b"MediaBox")).ok_or(format!("page {n} sans MediaBox"))?;
        let crop = rectangle(herite(&doc, id, b"CropBox"));
        let rotate = herite(&doc, id, b"Rotate").and_then(|o| o.as_i64().ok()).unwrap_or(0).rem_euclid(360);
        let [x0, y0, x1, y1] = crop.map_or(media, |c| [media[0].max(c[0]), media[1].max(c[1]), media[2].min(c[2]), media[3].min(c[3])]);
        if x1 <= x0 || y1 <= y0 {
            return Err(format!("page {n} : CropBox hors de la MediaBox"));
        }
        let propre = doc.get_dictionary(id).is_ok_and(|d| d.get(b"MediaBox").is_ok()) && [x0, y0, x1, y1] == media && rotate == 0;
        if propre {
            continue;
        }
        change = true;
        let (w, h) = (x1 - x0, y1 - y0);
        // Matrice qui pose la page vue (tournée de `rotate` dans le sens horaire) sur [0, l] × [0, h].
        let (matrice, boite) = match rotate {
            90 => ([0.0, -1.0, 1.0, 0.0, -y0, w + x0], [0.0, 0.0, h, w]),
            180 => ([-1.0, 0.0, 0.0, -1.0, w + x0, h + y0], [0.0, 0.0, w, h]),
            270 => ([0.0, 1.0, -1.0, 0.0, h + y0, -x0], [0.0, 0.0, h, w]),
            _ => ([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], [x0, y0, x1, y1]),
        };
        if rotate != 0 {
            let anciens: Vec<Object> = match doc.get_dictionary(id).map_err(|e| e.to_string())?.get(b"Contents") {
                Ok(Object::Array(a)) => a.clone(),
                Ok(o) => vec![o.clone()],
                Err(_) => vec![],
            };
            let [a, b, c, d, e, f] = matrice;
            let debut = doc.add_object(Stream::new(dictionary! {}, format!("q {a} {b} {c} {d} {e:.4} {f:.4} cm\n").into_bytes()));
            let fin = doc.add_object(Stream::new(dictionary! {}, b"\nQ\n".to_vec()));
            let mut contenus = vec![Object::Reference(debut)];
            contenus.extend(anciens);
            contenus.push(Object::Reference(fin));
            doc.get_dictionary_mut(id).map_err(|e| e.to_string())?.set("Contents", Object::Array(contenus));
        }
        let dict = doc.get_dictionary_mut(id).map_err(|e| e.to_string())?;
        dict.set("MediaBox", boite.iter().map(|v| Object::Real(*v as f32)).collect::<Vec<_>>());
        for cle in [b"CropBox".as_slice(), b"BleedBox", b"TrimBox", b"ArtBox", b"Rotate"] {
            dict.remove(cle);
        }
    }
    if !change {
        return Ok(None);
    }
    // Les nœuds de l'arbre ne doivent plus rien transmettre : chaque page porte sa boîte.
    for o in doc.objects.values_mut() {
        if let Object::Dictionary(d) = o
            && d.get(b"Type").and_then(Object::as_name).is_ok_and(|t| t == b"Pages")
        {
            for cle in [b"MediaBox".as_slice(), b"CropBox", b"Rotate"] {
                d.remove(cle);
            }
        }
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok(Some(sortie))
}

/// Suffixe d'une couche de découpe désactivée sur son travail, une fois sur une planche.
pub const SUFFIXE_EXCLU: &str = "_exclu";

/// Couches désactivées d'un travail posé sur une planche (roadmap 5.3) : leurs tons de
/// découpe sont renommés `<ton>_exclu` dans tout le PDF (Separation et DeviceN). Le préfixe
/// reste, donc le ton reste un ton de découpe jamais imprimé, mais il forme une couche à
/// part, que la planche garde désactivée ; l'encre n'est pas touchée.
pub fn exclure_tons(pdf: &[u8], exclus: &[String]) -> Result<Vec<u8>, String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    fn renommer(o: &mut Object, exclus: &[String]) {
        let nom = |o: &mut Object| {
            if let Object::Name(n) = o {
                let ton = String::from_utf8_lossy(n).into_owned();
                if exclus.iter().any(|e| crate::decoupe::meme_couche(e, &ton)) {
                    *n = format!("{ton}{SUFFIXE_EXCLU}").into_bytes();
                }
            }
        };
        match o {
            Object::Array(a) => {
                let famille = a.first().and_then(|x| x.as_name().ok()).map(<[u8]>::to_vec);
                match (famille.as_deref(), a.get_mut(1)) {
                    (Some(b"Separation"), Some(x)) => nom(x),
                    (Some(b"DeviceN"), Some(Object::Array(noms))) => noms.iter_mut().for_each(nom),
                    _ => {}
                }
                a.iter_mut().for_each(|x| renommer(x, exclus));
            }
            Object::Dictionary(d) => d.iter_mut().for_each(|(_, x)| renommer(x, exclus)),
            Object::Stream(s) => s.dict.iter_mut().for_each(|(_, x)| renommer(x, exclus)),
            _ => {}
        }
    }
    doc.objects.values_mut().for_each(|o| renommer(o, exclus));
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok(sortie)
}

/// Une page à imbriquer : PDF source, numéro de page (à partir de 1), nombre de copies.
pub struct Source<'a> {
    pub pdf: &'a [u8],
    pub page: usize,
    pub copies: usize,
}

fn nombres(o: &Object) -> Option<Vec<f64>> {
    Some(o.as_array().ok()?.iter().filter_map(|v| v.as_float().ok()).map(f64::from).collect())
}

/// Copie la page `page` (à partir de 1) de `pdf` dans `cible` sous forme de formulaire
/// XObject : rend son identifiant, sa taille et le coin bas gauche de sa MediaBox.
/// Tous les objets de la source passent dans la cible (numéros décalés) ; les inutiles
/// sont retirés par `prune_objects` à la fin de la composition.
pub fn page_en_formulaire(cible: &mut Document, pdf: &[u8], page: usize) -> Result<(ObjectId, Taille, [f64; 2]), String> {
    let mut doc = Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    doc.renumber_objects_with(cible.max_id + 1);
    let id_page = *doc.get_pages().get(&(page as u32)).ok_or(format!("pas de page {page}"))?;
    let dict = doc.get_dictionary(id_page).map_err(|e| e.to_string())?;
    let boite = dict.get(b"MediaBox").ok().and_then(nombres).filter(|b| b.len() == 4).ok_or("page sans MediaBox")?;
    if dict.get(b"Rotate").and_then(Object::as_i64).is_ok_and(|r| r % 360 != 0) {
        return Err("page tournée (/Rotate) non prise en charge".into());
    }
    let ressources: Dictionary = match doc.get_page_resources(id_page).map_err(|e| e.to_string())? {
        (Some(d), _) => d.clone(),
        (None, ids) => ids.first().and_then(|id| doc.get_dictionary(*id).ok()).cloned().unwrap_or_default(),
    };
    let contenu = doc.get_page_content(id_page);
    cible.max_id = doc.max_id;
    cible.objects.extend(doc.objects);
    let form = Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => boite.iter().map(|v| Object::Real(*v as f32)).collect::<Vec<_>>(),
            "Resources" => ressources,
        },
        contenu,
    );
    let id = cible.add_object(form);
    Ok((id, Taille { largeur: boite[2] - boite[0], hauteur: boite[3] - boite[1] }, [boite[0], boite[1]]))
}

/// Imposition des copies déroulées : en rangées, ou sur la forme de leurs contours de
/// découpe (`forme`).
pub struct Imposition {
    pub places: Vec<Place>,
    pub planche: Taille,
    /// Pour chaque copie : sa source et sa taille.
    pub elements: Vec<usize>,
    pub tailles: Vec<Taille>,
    /// Contours fermés de chaque source (vides sans `forme`), en points de sa page.
    pub contours: Vec<Vec<Vec<(f64, f64)>>>,
}

/// Step & repeat d'une seule source : grille de `colonnes` × `rangees`, avec demi-saut
/// (une colonne sur deux descend d'une demi-hauteur, comme un papier peint).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Motif {
    pub colonnes: usize,
    pub rangees: usize,
    #[serde(default)]
    pub demi_saut: bool,
}

/// Places de la grille et taille de la planche, en points ; refus si elle dépasse la laize.
pub fn grille(t: Taille, m: Motif, laize: f64, espace: f64) -> Result<(Vec<Place>, Taille), String> {
    let (pas_x, pas_y) = (t.largeur + espace, t.hauteur + espace);
    let largeur = m.colonnes as f64 * pas_x - espace;
    if largeur > laize + 1e-6 {
        return Err(format!("{} colonnes de {:.0} mm dépassent la laize", m.colonnes, t.largeur / PT_PAR_MM));
    }
    let saut = if m.demi_saut && m.colonnes > 1 { pas_y / 2.0 } else { 0.0 };
    let places = (0..m.colonnes)
        .flat_map(|c| (0..m.rangees).map(move |r| (c, r)))
        .enumerate()
        .map(|(indice, (c, r))| Place { indice, x: c as f64 * pas_x, y: r as f64 * pas_y + if c % 2 == 1 { saut } else { 0.0 }, angle: 0 })
        .collect();
    Ok((places, Taille { largeur, hauteur: m.rangees as f64 * pas_y - espace + saut }))
}

pub fn imposer(sources: &[Source], laize_mm: f64, espace_mm: f64, rotation: bool, forme: bool, motif: Option<Motif>) -> Result<Imposition, String> {
    let tailles_sources = sources.iter().enumerate().map(|(n, s)| taille_page(s.pdf, s.page).map_err(|e| format!("source {} : {e}", n + 1))).collect::<Result<Vec<_>, _>>()?;
    let elements: Vec<usize> = match motif {
        Some(_) if sources.len() != 1 => return Err("un motif répète une seule page".into()),
        Some(m) => vec![0; m.colonnes * m.rangees],
        None => sources.iter().enumerate().flat_map(|(n, s)| std::iter::repeat_n(n, s.copies)).collect(),
    };
    if elements.is_empty() {
        return Err("rien à imbriquer".into());
    }
    let tailles: Vec<Taille> = elements.iter().map(|&n| tailles_sources[n]).collect();
    let (laize, espace) = (laize_mm * PT_PAR_MM, espace_mm * PT_PAR_MM);
    // Silhouette de chaque source : ses contours de découpe fermés (rien : la page).
    // Toujours extraits : ils servent aussi à mesurer la surface réellement utilisée.
    let contours: Vec<Vec<Vec<(f64, f64)>>> = sources
        .iter()
        .map(|s| {
            let d = crate::decoupe::extraire(s.pdf, false).ok().and_then(|pages| pages.into_iter().nth(s.page - 1));
            d.map(|d| d.traces.into_iter().filter(|t| t.ferme && t.points.len() >= 3).map(|t| t.points).collect()).unwrap_or_default()
        })
        .collect();
    let (places, planche) = if let Some(m) = motif {
        grille(tailles_sources[0], m, laize, espace)?
    } else if forme {
        let silhouettes: Vec<crate::forme::Silhouette> = elements.iter().map(|&n| crate::forme::Silhouette { taille: tailles_sources[n], polygones: contours[n].clone() }).collect();
        crate::forme::placer(&silhouettes, laize, espace, rotation)?
    } else {
        placer(&tailles, laize, espace, rotation)?
    };
    Ok(Imposition { places, planche, elements, tailles, contours })
}

/// Habillage de la planche (roadmap 5.5) : blanc sous et au-dessus des éléments pour la
/// table de découpe (points), et contour de toute la planche dans le ton `NestTrimBox`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Finition {
    pub marge_bas: f64,
    pub marge_haut: f64,
    pub contour: bool,
}

/// Compose la planche : rend le PDF et les places (dans l'ordre des copies déroulées).
pub fn composer(sources: &[Source], laize_mm: f64, espace_mm: f64, rotation: bool, forme: bool, motif: Option<Motif>, finition: Finition) -> Result<(Vec<u8>, Vec<Place>), String> {
    let Imposition { places, planche, elements, .. } = imposer(sources, laize_mm, espace_mm, rotation, forme, motif)?;
    let pdf = composer_places(sources, &places, &elements, planche, finition)?;
    Ok((pdf, places))
}

/// PDF d'une planche déjà placée : `places` (indice → copie), `elements` (copie → source),
/// `planche` la boîte des éléments ; l'habillage s'ajoute autour.
pub fn composer_places(sources: &[Source], places: &[Place], elements: &[usize], planche: Taille, finition: Finition) -> Result<Vec<u8>, String> {
    let mut cible = Document::with_version("1.6");
    // Formulaires créés, un par source, et leur taille.
    let mut formulaires: Vec<(ObjectId, Taille, [f64; 2])> = vec![];
    for (n, s) in sources.iter().enumerate() {
        formulaires.push(page_en_formulaire(&mut cible, s.pdf, s.page).map_err(|e| format!("source {} : {e}", n + 1))?);
    }

    let mut xobjets = Dictionary::new();
    let mut contenu = String::new();
    for (n, (id, _, _)) in formulaires.iter().enumerate() {
        xobjets.set(format!("F{n}"), *id);
    }
    let Finition { marge_bas, marge_haut, contour } = finition;
    for p in places {
        let n = elements[p.indice];
        let m = matrice_pose(formulaires[n].1, formulaires[n].2, p.angle, p.x, p.y + marge_bas);
        contenu += &format!("q {} {} {} {} {:.4} {:.4} cm /F{n} Do Q\n", m[0], m[1], m[2], m[3], m[4], m[5]);
    }
    let mut ressources = dictionary! { "XObject" => xobjets };
    if contour {
        // Ton de découpe à teinte magenta, comme `CutContour` : jamais imprimé.
        let ton = Object::Array(vec![
            "Separation".into(),
            crate::decoupe::NEST_TRIM_BOX.into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        ressources.set("ColorSpace", dictionary! { "NT" => ton });
        contenu += &format!("q /NT CS 1 SCN 0.5 w 0 {marge_bas:.4} {:.4} {:.4} re S Q\n", planche.largeur, planche.hauteur);
    }
    let flux = cible.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
    let pages = cible.new_object_id();
    let page = cible.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), Object::Real(planche.largeur as f32), Object::Real((planche.hauteur + marge_bas + marge_haut) as f32)],
        "Contents" => flux, "Resources" => ressources,
    });
    cible.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
    let catalogue = cible.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    cible.trailer.set("Root", catalogue);
    cible.prune_objects();
    let mut sortie = vec![];
    cible.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok(sortie)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const MM: f64 = PT_PAR_MM;

    fn t(l: f64, h: f64) -> Taille {
        Taille { largeur: l * MM, hauteur: h * MM }
    }

    #[test]
    fn motif_en_grille_et_demi_saut() {
        let m = Motif { colonnes: 3, rangees: 2, demi_saut: true };
        let (places, planche) = grille(t(100.0, 50.0), m, 400.0 * MM, 10.0 * MM).unwrap();
        assert_eq!(places.len(), 6);
        let mm = |v: f64| (v / MM * 1e6).round() / 1e6;
        // Colonne du milieu descendue d'une demi-hauteur de pas (60 / 2 = 30 mm).
        let colonne = |c: usize| places.iter().filter(|p| mm(p.x) == c as f64 * 110.0).map(|p| mm(p.y)).collect::<Vec<_>>();
        assert_eq!((colonne(0), colonne(1), colonne(2)), (vec![0.0, 60.0], vec![30.0, 90.0], vec![0.0, 60.0]));
        assert_eq!((mm(planche.largeur), mm(planche.hauteur)), (320.0, 140.0));
        assert!(grille(t(100.0, 50.0), Motif { colonnes: 4, ..m }, 400.0 * MM, 10.0 * MM).is_err(), "4 × 110 − 10 = 430 mm > laize");
    }

    #[test]
    fn placement_dans_la_laize_sans_chevauchement() {
        let tailles = [t(300.0, 200.0), t(500.0, 100.0), t(400.0, 300.0), t(300.0, 200.0), t(600.0, 150.0)];
        let (places, planche) = placer(&tailles, 1000.0 * MM, 5.0 * MM, false).unwrap();
        assert_eq!(places.len(), tailles.len());
        assert!(planche.largeur <= 1000.0 * MM + 1e-6);
        for (i, a) in places.iter().enumerate() {
            let ta = tailles[a.indice];
            assert!(a.x >= -1e-9 && a.x + ta.largeur <= 1000.0 * MM + 1e-6, "dans la laize");
            for b in &places[i + 1..] {
                let tb = tailles[b.indice];
                let separes = a.x + ta.largeur <= b.x + 1e-6 || b.x + tb.largeur <= a.x + 1e-6 || a.y + ta.hauteur <= b.y + 1e-6 || b.y + tb.hauteur <= a.y + 1e-6;
                assert!(separes, "chevauchement entre {} et {}", a.indice, b.indice);
            }
        }
        // Les deux plus hauts (300 et 200 mm) partagent la première rangée avec un 300 mm de large.
        assert_eq!(places[0].indice, 2);
        assert!(planche.hauteur < (300.0 + 200.0 + 200.0 + 150.0 + 100.0) * MM, "rangées partagées : {} mm", planche.hauteur / MM);
    }

    #[test]
    fn rotation_quand_elle_raccourcit_la_planche() {
        // 3 copies de 200 × 100 mm sur 320 mm : tel quel une par rangée (310 mm de long),
        // couchées (100 × 200) trois côte à côte (200 mm).
        let tailles = [t(200.0, 100.0); 3];
        let (_, sans) = placer(&tailles, 320.0 * MM, 5.0 * MM, false).unwrap();
        let (places, avec) = placer(&tailles, 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!((sans.hauteur / MM - 310.0).abs() < 1e-6 && (avec.hauteur / MM - 200.0).abs() < 1e-6);
        assert!(places.iter().all(|p| p.tourne()));
        // Un format plus large que la laize n'y passe que tourné ; sans rotation, refus.
        let (places, _) = placer(&[t(400.0, 100.0)], 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!(places[0].tourne());
        assert!(placer(&[t(400.0, 100.0)], 320.0 * MM, 5.0 * MM, false).is_err());
        // Rien à gagner : pas de rotation.
        let (places, _) = placer(&[t(100.0, 50.0); 2], 320.0 * MM, 5.0 * MM, true).unwrap();
        assert!(places.iter().all(|p| !p.tourne()));
    }

    /// Un élément tourné est rendu et découpé dans sa case : une page magenta de 60 × 40 pt
    /// (MediaBox décalée) couchée en 40 × 60 sur une laize de 50 pt.
    #[test]
    fn element_tourne_rendu_et_decoupe_en_place() {
        let magenta = pdf(60.0, 40.0, "0 1 0 0", (50.0, 30.0));
        let sources = [Source { pdf: &magenta, page: 1, copies: 1 }];
        let (planche, places) = composer(&sources, 50.0 / PT_PAR_MM, 0.0, true, false, None, Finition::default()).unwrap();
        assert!(places[0].tourne());
        let d = crate::decoupe::extraire(&planche, false).unwrap();
        assert!((d[0].largeur - 40.0).abs() < 1e-3 && (d[0].hauteur - 60.0).abs() < 1e-3, "{} × {}", d[0].largeur, d[0].hauteur);
        let (xs, ys): (Vec<f64>, Vec<f64>) = d[0].traces[0].points.iter().copied().unzip();
        let borne = |v: &[f64]| (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
        // Contour à 5 pt du bord de la page source : 5..35 en x, 5..55 en y une fois couché.
        let ((x0, x1), (y0, y1)) = (borne(&xs), borne(&ys));
        assert!((x0 - 5.0).abs() < 1e-3 && (x1 - 35.0).abs() < 1e-3 && (y0 - 5.0).abs() < 1e-3 && (y1 - 55.0).abs() < 1e-3, "{x0} {x1} {y0} {y1}");
        let dossier = std::env::temp_dir().join(format!("eko-rip-rotation-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("p.pdf"), &planche).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r72"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(dossier.join("p.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let m = image::open(dossier.join("r(Magenta).tif")).unwrap().to_luma8();
        assert_eq!(m.dimensions(), (40, 60));
        assert_eq!((m.get_pixel(20, 30)[0], m.get_pixel(2, 2)[0], m.get_pixel(37, 57)[0]), (0, 0, 0), "toute la case est magenta");
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Page carrée de `c` pt : triangle rectangle cyan bordé d'un contour `CutContour`.
    fn pdf_triangle(c: f64) -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec![
            "Separation".into(),
            "CutContour".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        let contenu = format!("1 0 0 0 k 0 0 m {c} 0 l 0 {c} l h f /CS0 CS 1 SCN 0 0 m {c} 0 l 0 {c} l h S");
        let cs = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), Object::Real(c as f32), Object::Real(c as f32)], "Contents" => cs, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        o
    }

    /// Deux triangles de 100 mm sur une laize de 105 mm, sur la forme : la planche les pose
    /// tête-bêche (≈ 100 mm de long au lieu de 200), et leurs contours découpés ne se
    /// chevauchent pas.
    #[test]
    fn planche_sur_la_forme_tete_beche() {
        let tri = pdf_triangle(100.0 * MM);
        let sources = [Source { pdf: &tri, page: 1, copies: 2 }];
        let (planche, places) = composer(&sources, 105.0, 0.0, true, true, None, Finition::default()).unwrap();
        assert!(places.iter().any(|p| p.angle == 180), "{places:?}");
        let d = crate::decoupe::extraire(&planche, false).unwrap();
        assert!(d[0].hauteur / MM < 120.0, "longueur {} mm", d[0].hauteur / MM);
        assert_eq!(d[0].traces.len(), 2);
        // Le centre de gravité de chaque triangle n'est pas dans l'autre.
        let centre = |t: &crate::decoupe::Trace| { let n = t.points.len() as f64; t.points.iter().fold((0.0, 0.0), |(a, b), p| (a + p.0 / n, b + p.1 / n)) };
        let dans = |p: (f64, f64), poly: &[(f64, f64)]| {
            let mut v = false;
            let mut j = poly.len() - 1;
            for i in 0..poly.len() {
                let ((xi, yi), (xj, yj)) = (poly[i], poly[j]);
                if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi { v = !v; }
                j = i;
            }
            v
        };
        let (a, b) = (&d[0].traces[0], &d[0].traces[1]);
        assert!(!dans(centre(a), &b.points) && !dans(centre(b), &a.points), "contours imbriqués l'un dans l'autre");
        // En rangées (sans la forme), deux carrés l'un au-dessus de l'autre.
        let (rangs, _) = composer(&sources, 105.0, 0.0, true, false, None, Finition::default()).unwrap();
        assert!(crate::decoupe::extraire(&rangs, false).unwrap()[0].hauteur / MM > 190.0);
    }

    #[test]
    fn trop_large_refuse() {
        let e = placer(&[t(1200.0, 100.0)], 1000.0 * MM, 5.0 * MM, false).unwrap_err();
        assert!(e.contains("plus large que la laize"), "{e}");
    }

    /// PDF d'une page `l × h` pt dont la MediaBox commence en (`ox`, `oy`), remplie d'une
    /// couleur CMJN, avec un contour `CutContour` à 5 pt du bord.
    pub(crate) fn pdf(l: f64, h: f64, cmjn: &str, (ox, oy): (f64, f64)) -> Vec<u8> {
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec![
            "Separation".into(),
            "CutContour".into(),
            "DeviceCMYK".into(),
            Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 }),
        ]);
        let contenu = format!("{cmjn} k {ox} {oy} {l} {h} re f /CS0 CS 1 SCN {} {} {} {} re S", ox + 5.0, oy + 5.0, l - 10.0, h - 10.0);
        let c = doc.add_object(Stream::new(dictionary! {}, contenu.into_bytes()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "MediaBox" => vec![Object::Real(ox as f32), Object::Real(oy as f32), Object::Real((ox + l) as f32), Object::Real((oy + h) as f32)], "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut o = vec![];
        doc.save_to(&mut o).unwrap();
        o
    }

    /// MediaBox, CropBox et /Rotate 270 hérités du nœud `Pages` : la page devient
    /// 40 × 80 pt (la CropBox, couchée), sans /Rotate, et la découpe suit.
    #[test]
    fn boites_et_rotation_heritees_normalisees() {
        let mut doc = Document::with_version("1.4");
        let sep = Object::Array(vec!["Separation".into(), "CutContour".into(), "DeviceCMYK".into(), Object::Dictionary(dictionary! { "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "C0" => vec![0.into(); 4], "C1" => vec![0.into(), 1.into(), 0.into(), 0.into()], "N" => 1 })]);
        let c = doc.add_object(Stream::new(dictionary! {}, b"/CS0 CS 1 SCN 20 20 10 10 re S".to_vec()));
        let pages = doc.new_object_id();
        let page = doc.add_object(dictionary! { "Type" => "Page", "Parent" => pages, "Contents" => c, "Resources" => dictionary! { "ColorSpace" => dictionary! { "CS0" => sep } } });
        doc.objects.insert(pages, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1, "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()], "CropBox" => vec![10.into(), 10.into(), 90.into(), 50.into()], "Rotate" => 270 }));
        let cat = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", cat);
        let mut pdf = vec![];
        doc.save_to(&mut pdf).unwrap();

        let n = normaliser_pages(&pdf).unwrap().expect("à normaliser");
        assert_eq!(taille_page(&n, 1).unwrap(), Taille { largeur: 40.0, hauteur: 80.0 });
        assert!(normaliser_pages(&n).unwrap().is_none(), "déjà propre");
        // Carré (20, 20)–(30, 30) : à 10 pt du bas gauche de la CropBox ; 270° horaire :
        // x' = 40 − (y − 10), y' = x − 10 → (20, 10)–(30, 20).
        let d = &crate::decoupe::extraire(&n, false).unwrap()[0];
        let xs: Vec<f64> = d.traces[0].points.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = d.traces[0].points.iter().map(|p| p.1).collect();
        let borne = |v: &[f64]| (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max));
        let ((x0, x1), (y0, y1)) = (borne(&xs), borne(&ys));
        assert!([x0 - 20.0, x1 - 30.0, y0 - 10.0, y1 - 20.0].iter().all(|e| e.abs() < 1e-3), "{x0} {x1} {y0} {y1}");
    }

    #[test]
    fn planche_rendue_et_decoupee_aux_bonnes_places() {
        let cyan = pdf(100.0, 100.0, "1 0 0 0", (0.0, 0.0));
        // MediaBox décalée, comme souvent à l'export d'Illustrator.
        let magenta = pdf(60.0, 40.0, "0 1 0 0", (50.0, 30.0));
        let sources = [Source { pdf: &cyan, page: 1, copies: 1 }, Source { pdf: &magenta, page: 1, copies: 2 }];
        // Laize 80 mm ≈ 226,8 pt : cyan (100) + magenta (60) + espace tiennent en rangée 1 ; le 2e magenta passe en rangée 2.
        let (planche, places) = composer(&sources, 80.0, 5.0, false, false, None, Finition::default()).unwrap();
        assert_eq!(places.len(), 3);

        // Découpe : trois contours, chacun dans son élément placé.
        let d = crate::decoupe::extraire(&planche, false).unwrap().remove(0);
        assert_eq!(d.traces.len(), 3);
        let elements = [0usize, 1, 1];
        for p in &places {
            let (l, h) = if elements[p.indice] == 0 { (100.0, 100.0) } else { (60.0, 40.0) };
            let attendu = (p.x + 5.0, p.y + 5.0);
            let trouve = d.traces.iter().any(|t| (t.points[0].0 - attendu.0).abs() < 1e-3 && (t.points[0].1 - attendu.1).abs() < 1e-3 && (t.points[2].0 - (p.x + l - 5.0)).abs() < 1e-3 && (t.points[2].1 - (p.y + h - 5.0)).abs() < 1e-3);
            assert!(trouve, "contour de l'élément {} absent ou mal placé", p.indice);
        }

        // Rendu : la couleur de chaque élément est à sa place.
        let dossier = std::env::temp_dir().join(format!("eko-rip-planche-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("p.pdf"), &planche).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=tiffsep", "-r36"])
            .arg(format!("-sOutputFile={}", dossier.join("r.tif").display()))
            .arg(dossier.join("p.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let lire = |plaque: &str, x: f64, y: f64| {
            let img = image::open(dossier.join(format!("r({plaque}).tif"))).unwrap().to_luma8();
            img.get_pixel((x / 2.0) as u32, img.height() - 1 - (y / 2.0) as u32)[0]
        };
        for p in &places {
            let (cx, cy, plaque) = if elements[p.indice] == 0 { (p.x + 50.0, p.y + 50.0, "Cyan") } else { (p.x + 30.0, p.y + 20.0, "Magenta") };
            assert_eq!(lire(plaque, cx, cy), 0, "{plaque} absent au centre de l'élément {}", p.indice);
        }
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
