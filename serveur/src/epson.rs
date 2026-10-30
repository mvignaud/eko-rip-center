//! Epson SC-S80600 : ce qui part vers l'imprimante, et par où.
//!
//! Deux sorties, selon la connexion de la machine :
//! - **pilote direct** (connexion réseau, port 9100 ; roadmap 2.4) : `imprimer` lit les
//!   plaques C, M, J, N du rendu, les sépare en 9 encres (`separation.rs`), trame chaque
//!   encre en 2 bits (`tramage.rs`) et émet le flux ESC/P2 (`escp2.rs`) bande par bande
//!   dans la connexion. C'est le format que l'imprimante reçoit d'un RIP (capture du 24/10) :
//!   raster tramé sur 9 canaux, pas de ton continu ;
//! - **mode « Dossier »** (connexion `Dossier`) : les plaques sont copiées telles quelles,
//!   un fichier par canal (`preparer`), pour un autre logiciel ou un contrôle à l'œil.
//!
//! Dans les deux cas, les tons de découpe ne partent jamais, et un ton direct sans encre
//! sur l'Epson fait refuser l'envoi plutôt que d'imprimer sans lui. Blanc et argent (WH,
//! MS) : canaux à confirmer par une capture (2.5) ; le pilote direct les refuse d'ici là.
//!
//! Mémoire du pilote : les plaques sont lues bande par bande (`blanc::LecteurTiff`), rien
//! de la page n'est gardé ; une bâche de plusieurs mètres passe comme un autocollant.

use crate::blanc::LecteurTiff;
use crate::decoupe;
use crate::escp2::{self, Encre, LIGNES};
use crate::separation::{self, Cmjn, Separation};
use crate::tramage::{Tramage, Trameur};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Canal d'une plaque en mode « Dossier », dans l'ordre des fichiers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
pub enum Canal {
    K,
    C,
    M,
    Y,
    /// Blanc.
    WH,
    /// Argent métallisé.
    MS,
}

/// Où l'encre d'accompagnement (WH ou MS) se pose par rapport à la couleur.
#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Couches {
    /// Sous la couleur : support transparent ou foncé vu de face, argent teinté.
    #[default]
    Dessous,
    /// Sur la couleur : vitrophanie vue à travers le support.
    Dessus,
}

/// Une couche de l'empilement (roadmap 2.10), de la première posée sur le support à la
/// dernière : blanc-couleur-blanc pour une vitrophanie vue des deux côtés, couleur-couleur
/// pour un rétroéclairé jour/nuit, etc.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Couche {
    /// Les plaques K, C, M, Y présentes.
    Couleur,
    /// La plaque WH.
    Blanc,
    /// La plaque MS.
    Argent,
}

impl Couche {
    fn prend(self, c: Canal) -> bool {
        match self {
            Couche::Couleur => matches!(c, Canal::K | Canal::C | Canal::M | Canal::Y),
            Couche::Blanc => c == Canal::WH,
            Couche::Argent => c == Canal::MS,
        }
    }
}

/// Couches au plus dans un empilement.
pub const COUCHES_MAX: usize = 5;

/// Canal d'une plaque du rendu ; `None` pour un ton sans encre sur l'Epson.
/// Noms conseillés par Epson (« Printing with WH/MS ») : `White` et `Metallic`.
pub fn canal(plaque: &str) -> Option<Canal> {
    let n = plaque.trim().to_lowercase();
    Some(match n.as_str() {
        "black" => Canal::K,
        "cyan" => Canal::C,
        "magenta" => Canal::M,
        "yellow" => Canal::Y,
        "white" | "blanc" | "wh" => Canal::WH,
        "metallic" | "silver" | "ms" | "argent" => Canal::MS,
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Envoi {
    /// Canaux présents dans l'ordre K C M Y puis WH ou MS, avec leur plaque.
    pub canaux: Vec<(Canal, String)>,
    pub couches: Couches,
    /// Passages dans l'ordre de pose, chacun avec ses canaux (roadmap 2.10). Sans
    /// empilement déclaré : la couleur, et l'encre d'accompagnement dessous ou dessus.
    pub passages: Vec<Vec<Canal>>,
}

/// Plaques d'une page → envoi. Refus : ton direct sans encre, WH et MS sur la même
/// page (un passage par encre d'accompagnement), couche de l'empilement sans sa plaque ou
/// plaque qu'aucune couche ne pose. `pile` vide : `couches` décide (dessous ou dessus).
pub fn preparer(plaques: &[String], couches: Couches, pile: &[Couche]) -> Result<Envoi, String> {
    let mut canaux = vec![];
    let mut sans_encre = vec![];
    for p in plaques.iter().filter(|p| !decoupe::est_ton_de_decoupe(p)) {
        match canal(p) {
            Some(c) if canaux.iter().any(|(x, _)| *x == c) => return Err(format!("deux plaques pour le canal {c:?} (dont « {p} »)")),
            Some(c) => canaux.push((c, p.clone())),
            None => sans_encre.push(format!("« {p} »")),
        }
    }
    if !sans_encre.is_empty() {
        return Err(format!("ton direct sans encre sur l'Epson : {} ; le convertir en CMJN, ou le nommer White ou Metallic", sans_encre.join(", ")));
    }
    canaux.sort();
    if canaux.iter().any(|(c, _)| *c == Canal::WH) && canaux.iter().any(|(c, _)| *c == Canal::MS) {
        return Err("blanc et argent sur la même page : un passage par encre".into());
    }
    if canaux.is_empty() {
        return Err("rien à imprimer : la page n'a que des tons de découpe".into());
    }
    let present = |k: Couche| canaux.iter().any(|(c, _)| k.prend(*c));
    let pile: Vec<Couche> = if pile.is_empty() {
        let accompagnement = [Couche::Blanc, Couche::Argent].into_iter().filter(|&k| present(k));
        let couleur = Some(Couche::Couleur).filter(|&k| present(k));
        match couches {
            Couches::Dessous => accompagnement.chain(couleur).collect(),
            Couches::Dessus => couleur.into_iter().chain(accompagnement).collect(),
        }
    } else {
        if let Some(k) = pile.iter().find(|&&k| !present(k)) {
            return Err(format!("couche {k:?} déclarée dans l'empilement, mais la page n'a pas sa plaque"));
        }
        if let Some((_, p)) = canaux.iter().find(|(c, _)| !pile.iter().any(|k| k.prend(*c))) {
            return Err(format!("« {p} » : aucune couche de l'empilement ne la pose"));
        }
        pile.to_vec()
    };
    let passages = pile.iter().map(|k| canaux.iter().map(|(c, _)| *c).filter(|&c| k.prend(c)).collect()).collect();
    Ok(Envoi { canaux, couches, passages })
}

/// Plaques que lit le pilote direct, dans l'ordre de `imprimer`.
pub const PLAQUES_CMJN: [&str; 4] = ["Cyan", "Magenta", "Yellow", "Black"];

/// Page admise par le pilote direct : celle du mode « Dossier », sans blanc ni argent.
pub fn verifier_pour_le_pilote(plaques: &[String], couches: Couches, pile: &[Couche]) -> Result<(), String> {
    let envoi = preparer(plaques, couches, pile)?;
    if envoi.passages.len() > 1 {
        return Err(format!("empilement de {} couches : le pilote direct n'imprime qu'un passage pour l'instant ; sortie en dossier en attendant", envoi.passages.len()));
    }
    match envoi.canaux.iter().find(|(c, _)| matches!(c, Canal::WH | Canal::MS)) {
        Some((_, p)) => Err(format!("« {p} » : le blanc et l'argent ne passent pas encore par le pilote direct (canaux à capturer) ; sortie en dossier en attendant")),
        None => Ok(()),
    }
}

/// Points posés par encre : petits, moyens, gros (roadmap 6.9, encre estimée).
pub type Points = Vec<(Encre, [u64; 3])>;

/// Pilote direct : les plaques C, M, J, N (`plaques`, TIFF 8 bits du rendu, 0 = pleine
/// encre) sont séparées, tramées et émises dans `w` bande par bande. `t` donne le nom,
/// la résolution, la position et la largeur de page ; la taille de l'image vient des
/// plaques. Rend l'écrivain une fois le flux complet écrit, et les points posés, comptés
/// au tramage (sans relire les plaques).
pub fn imprimer<W: Write>(plaques: [&Path; 4], mut t: escp2::Travail, sep: &Separation, trame: &Tramage, w: W) -> Result<(W, Points), String> {
    let mut lecteurs = plaques.map(LecteurTiff::ouvrir).into_iter().collect::<Result<Vec<_>, _>>()?;
    let (l, h) = (lecteurs[0].largeur, lecteurs[0].hauteur);
    if lecteurs.iter().any(|x| (x.largeur, x.hauteur) != (l, h)) {
        return Err("plaques CMJN de tailles différentes".into());
    }
    (t.largeur, t.hauteur, t.page_hauteur) = (l, h, t.y + h as u32);
    let mut e = escp2::Emetteur::nouveau(w, &t)?;
    let mut trameurs = separation::ENCRES.iter().map(|&encre| Trameur::nouveau(encre, l, trame)).collect::<Result<Vec<_>, _>>()?;
    let mut cmjn = vec![vec![0u8; LIGNES * l]; 4];
    let mut points = vec![vec![0u8; LIGNES * l]; separation::ENCRES.len()];
    let mut comptes = vec![[0u64; 3]; separation::ENCRES.len()];
    for y0 in (0..h).step_by(LIGNES) {
        let n = LIGNES.min(h - y0);
        for (lecteur, canal) in lecteurs.iter_mut().zip(cmjn.iter_mut()) {
            for r in 0..n {
                // Plaque : 0 = pleine encre ; séparation : 255 = pleine encre.
                for (o, v) in canal[r * l..(r + 1) * l].iter_mut().zip(lecteur.ligne()?) {
                    *o = 255 - v;
                }
            }
        }
        let z = n * l;
        let s = separation::separer(&Cmjn { c: &cmjn[0][..z], m: &cmjn[1][..z], y: &cmjn[2][..z], k: &cmjn[3][..z], or: None, rd: None }, sep)?;
        for (((trameur, (_, valeurs)), sortie), compte) in trameurs.iter_mut().zip(&s).zip(points.iter_mut()).zip(comptes.iter_mut()) {
            for r in 0..n {
                trameur.tramer_ligne(&valeurs[r * l..(r + 1) * l], &mut sortie[r * l..(r + 1) * l]);
            }
            for &p in &sortie[..z] {
                if p > 0 {
                    compte[p as usize - 1] += 1;
                }
            }
        }
        let plans: Vec<(Encre, &[u8])> = s.iter().zip(&points).map(|((encre, _), p)| (*encre, &p[..z])).collect();
        e.bande(&plans)?;
    }
    Ok((e.finir()?, separation::ENCRES.iter().copied().zip(comptes).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noms(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plaques_vers_canaux_sans_la_decoupe() {
        let e = preparer(&noms(&["Cyan", "CutContour", "Magenta", "White", "Black", "Yellow", "Kiss-Cut"]), Couches::Dessus, &[]).unwrap();
        let ordre: Vec<_> = e.canaux.iter().map(|(c, _)| *c).collect();
        assert_eq!(ordre, [Canal::K, Canal::C, Canal::M, Canal::Y, Canal::WH]);
        assert!(e.canaux.iter().all(|(_, p)| !decoupe::est_ton_de_decoupe(p)), "jamais de plaque de découpe");
        assert_eq!(e.couches, Couches::Dessus);
        assert_eq!(preparer(&noms(&["Argent", "Cyan"]), Couches::Dessous, &[]).unwrap().canaux, [(Canal::C, "Cyan".into()), (Canal::MS, "Argent".into())]);
    }

    /// Roadmap 6.9 : les points comptés au tramage sont ceux que le flux ESC/P2 transporte,
    /// taille par taille et encre par encre (relus dans le flux décodé).
    #[test]
    fn points_comptes_au_tramage_egaux_au_flux() {
        let d = std::env::temp_dir().join(format!("epson-points-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let (l, h) = (80u32, 40u32);
        // Plaques du rendu : 0 = pleine encre. Cyan en dégradé, magenta en bande, jaune vide.
        type Plaque = (&'static str, fn(u32, u32) -> u8);
        let plaques: [Plaque; 4] = [("c", |x, _| 255 - (x * 255 / 79) as u8), ("m", |_, y| if y < 20 { 128 } else { 255 }), ("y", |_, _| 255), ("k", |x, y| if x > 60 && y > 30 { 0 } else { 255 })];
        for (nom, f) in plaques {
            image::GrayImage::from_fn(l, h, |x, y| image::Luma([f(x, y)])).save(d.join(format!("{nom}.tif"))).unwrap();
        }
        let chemins = ["c", "m", "y", "k"].map(|n| d.join(format!("{n}.tif")));
        let t = escp2::Travail { nom: "points".into(), poste: "atelier".into(), utilisateur: "essai".into(), largeur: 0, hauteur: 0, dpi: 720, x: 0, y: 0, page_largeur: 200, page_hauteur: 0 };
        let (flux, points) = imprimer(chemins.each_ref().map(|p| p.as_path()), t, &Separation::default(), &Tramage::default(), Vec::new()).unwrap();
        let (_, blocs) = escp2::tests::decoder(&flux);
        let mut total = 0;
        for (encre, compte) in &points {
            let mut lu = [0u64; 3];
            for b in blocs.iter().filter(|b| b.code == encre.code()) {
                for o in &b.donnees {
                    for k in 0..4 {
                        let v = (o >> (6 - 2 * k)) & 3;
                        if v > 0 {
                            lu[v as usize - 1] += 1;
                        }
                    }
                }
            }
            assert_eq!(*compte, lu, "{encre:?}");
            total += compte.iter().sum::<u64>();
        }
        let jaune = points.iter().find(|(e, _)| *e == Encre::Y).unwrap().1;
        assert_eq!(jaune, [0; 3], "plaque jaune vide : aucun point");
        assert!(total > 1000, "{points:?}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn refus_explicites() {
        assert!(preparer(&noms(&["Cyan", "PANTONE 485 C"]), Couches::Dessous, &[]).unwrap_err().contains("PANTONE 485 C"));
        assert!(preparer(&noms(&["White", "Silver"]), Couches::Dessous, &[]).unwrap_err().contains("blanc et argent"));
        assert!(preparer(&noms(&["White", "Blanc"]), Couches::Dessous, &[]).unwrap_err().contains("WH"));
        assert!(preparer(&noms(&["CutContour"]), Couches::Dessous, &[]).is_err());
    }

    /// Roadmap 2.10 : trois couches déclarées → trois passages, dans l'ordre déclaré.
    #[test]
    fn empilement_dans_l_ordre_declare() {
        use Couche::*;
        let plaques = noms(&["Cyan", "White", "Black", "CutContour"]);
        let e = preparer(&plaques, Couches::Dessous, &[Blanc, Couleur, Blanc]).unwrap();
        assert_eq!(e.passages, [vec![Canal::WH], vec![Canal::K, Canal::C], vec![Canal::WH]]);
        assert_eq!(preparer(&plaques, Couches::Dessus, &[Couleur, Blanc, Couleur]).unwrap().passages, [vec![Canal::K, Canal::C], vec![Canal::WH], vec![Canal::K, Canal::C]]);
        // Sans empilement : l'ancien réglage dessous / dessus.
        assert_eq!(preparer(&plaques, Couches::Dessous, &[]).unwrap().passages, [vec![Canal::WH], vec![Canal::K, Canal::C]]);
        assert_eq!(preparer(&plaques, Couches::Dessus, &[]).unwrap().passages, [vec![Canal::K, Canal::C], vec![Canal::WH]]);
        assert_eq!(preparer(&noms(&["Cyan"]), Couches::Dessous, &[]).unwrap().passages, [vec![Canal::C]]);
        assert!(preparer(&noms(&["Cyan"]), Couches::Dessous, &[Blanc, Couleur]).unwrap_err().contains("Blanc"), "couche sans plaque");
        assert!(preparer(&plaques, Couches::Dessous, &[Couleur]).unwrap_err().contains("White"), "plaque sans couche");
        assert!(verifier_pour_le_pilote(&noms(&["Cyan"]), Couches::Dessous, &[Couleur, Couleur]).unwrap_err().contains("2 couches"));
        assert!(verifier_pour_le_pilote(&noms(&["Cyan"]), Couches::Dessous, &[Couleur]).is_ok());
    }
}
