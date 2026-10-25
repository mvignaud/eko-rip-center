//! Table de tons directs (roadmap 8.6) : nom exact → couleur Lab (D50), gardée dans
//! `tons_directs.json`. Un ton du PDF trouvé ici est imprimé en CMJN d'après sa valeur Lab
//! (et non d'après l'alternative du PDF) ; un ton absent garde le comportement d'avant.
//!
//! La recherche respecte la casse **et** les espaces : « PANTONE 485 C » ne trouve pas
//! « Pantone 485 C » ni « PANTONE  485 C » (un alias les ajoute au besoin). Aucune
//! bibliothèque de couleurs n'est livrée : l'atelier importe celle de son fournisseur
//! (CSV Lab), jamais une bibliothèque sous licence qu'il n'a pas.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ton {
    pub nom: String,
    /// Autres noms exacts du même ton dans les fichiers reçus.
    #[serde(default)]
    pub alias: Vec<String>,
    /// L* (0 à 100), a*, b* (−128 à 127), illuminant D50.
    pub lab: [f64; 3],
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("tons_directs.json")
}

pub fn lire(donnees: &Path) -> Vec<Ton> {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, table: &[Ton]) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), &table, false)
}

/// Ton de la table pour ce nom de séparation (nom ou alias, à l'identique).
pub fn trouver<'a>(table: &'a [Ton], nom: &str) -> Option<&'a Ton> {
    table.iter().find(|t| t.nom == nom || t.alias.iter().any(|a| a == nom))
}

/// Refus (code, détail) : nom vide, Lab hors bornes, nom en double (alias compris), ou nom
/// qui a déjà sa propre encre ou sert à la découpe (la table ne doit pas l'avaler).
pub fn verifier(table: &[Ton]) -> Result<(), (&'static str, String)> {
    let mut vus = std::collections::HashSet::new();
    for t in table {
        let [l, a, b] = t.lab;
        if !(0.0..=100.0).contains(&l) || !(-128.0..=127.0).contains(&a) || !(-128.0..=127.0).contains(&b) {
            return Err(("lab_invalide", t.nom.clone()));
        }
        for n in std::iter::once(&t.nom).chain(&t.alias) {
            if n.trim().is_empty() {
                return Err(("nom_vide", t.nom.clone()));
            }
            if crate::epson::canal(n).is_some() || crate::decoupe::est_ton_de_decoupe(n) {
                return Err(("nom_reserve", n.clone()));
            }
            if !vus.insert(n.as_str()) {
                return Err(("nom_en_double", n.clone()));
            }
        }
    }
    Ok(())
}

/// Lit un CSV `nom;L;a;b` (séparateur `;`, tabulation ou `,` ; avec `;` ou tabulation, la
/// virgule décimale est admise). Une première ligne non numérique est un en-tête. Le nom
/// est gardé tel quel, espaces compris. Refus : ligne illisible (n° de ligne).
pub fn lire_csv(texte: &str) -> Result<Vec<Ton>, String> {
    let texte = texte.strip_prefix('\u{feff}').unwrap_or(texte);
    let mut tons = vec![];
    for (i, ligne) in texte.lines().enumerate() {
        let ligne = ligne.strip_suffix('\r').unwrap_or(ligne);
        if ligne.trim().is_empty() {
            continue;
        }
        let sep = if ligne.contains(';') { ';' } else if ligne.contains('\t') { '\t' } else { ',' };
        let champs: Vec<&str> = ligne.split(sep).collect();
        let nombre = |s: &str| if sep == ',' { s.trim().parse::<f64>().ok() } else { s.trim().replace(',', ".").parse::<f64>().ok() };
        let lab = match champs.as_slice() {
            [_, l, a, b] => [nombre(l), nombre(a), nombre(b)],
            _ => [None; 3],
        };
        match lab {
            [Some(l), Some(a), Some(b)] => tons.push(Ton { nom: champs[0].into(), alias: vec![], lab: [l, a, b] }),
            _ if i == 0 => continue,
            _ => return Err(format!("ligne {} : attendu nom;L;a;b", i + 1)),
        }
    }
    Ok(tons)
}

/// Encrage C, M, J, N (0 à 255) d'une couleur Lab pleine, calculé par Ghostscript avec les
/// options couleur du travail (profil de sortie, intention) : le même moteur que le rendu.
pub async fn recette(gs: &str, options: &[String], lab: [f64; 3], delai: std::time::Duration) -> Result<[u8; 4], String> {
    let [l, a, b] = lab;
    let contenu = format!("/L cs {l} {a} {b} sc 0 0 10 10 re f\n");
    let objets = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R /Resources << /ColorSpace << /L [/Lab << /WhitePoint [0.9642 1 0.8249] /Range [-128 127 -128 127] >>] >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{contenu}endstream", contenu.len()),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut positions = vec![];
    for (i, o) in objets.iter().enumerate() {
        positions.push(pdf.len());
        pdf += &format!("{} 0 obj\n{o}\nendobj\n", i + 1);
    }
    let xref = pdf.len();
    pdf += &format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1);
    for p in positions {
        pdf += &format!("{p:010} 00000 n \n");
    }
    pdf += &format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1);
    let dossier = std::env::temp_dir().join(format!("eko-rip-recette-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
    let resultat = async {
        let source = dossier.join("ton.pdf");
        std::fs::write(&source, pdf).map_err(|e| e.to_string())?;
        crate::render::ghostscript(gs, &source, "tiffsep", 72, options, &dossier.join("p.tif"), delai).await?;
        let mut encres = [0u8; 4];
        for (e, nom) in encres.iter_mut().zip(["Cyan", "Magenta", "Yellow", "Black"]) {
            let img = image::open(dossier.join(format!("p({nom}).tif"))).map_err(|x| format!("recette {nom} : {x}"))?.to_luma8();
            *e = 255 - img.get_pixel(5, 5)[0];
        }
        Ok(encres)
    }
    .await;
    let _ = std::fs::remove_dir_all(&dossier);
    resultat
}

/// Fond la plaque d'un ton direct (`ton`) dans les plaques `cmjn` (C, M, J, N ; TIFF 8 bits,
/// 0 = pleine encre), en flux, selon `recette` : chaque point du ton à la teinte t ajoute
/// t × recette, en surimpression de ce qui est déjà là (1 − (1 − c)(1 − t·r)). Les plaques
/// sont réécrites en place ; la plaque du ton n'est pas touchée.
// ponytail: teinte proportionnelle à la recette pleine (pas d'interpolation Lab des tramés) ;
// une recette par palier de teinte si un tramé de ton direct dérive à la mesure.
pub fn fondre(cmjn: [&Path; 4], ton: &Path, recette: [u8; 4], dpi: u32) -> Result<(), String> {
    use crate::blanc::{EcrivainTiff, LecteurTiff, RANGS_PAR_BANDE};
    let mut lecteurs = cmjn.iter().map(|p| LecteurTiff::ouvrir(p)).collect::<Result<Vec<_>, _>>()?;
    let mut spot = LecteurTiff::ouvrir(ton)?;
    let (l, h) = (spot.largeur, spot.hauteur);
    if lecteurs.iter().any(|x| (x.largeur, x.hauteur) != (l, h)) {
        return Err("plaques de tailles différentes".into());
    }
    let tmp: Vec<PathBuf> = cmjn.iter().map(|p| p.with_extension("tif.tmp")).collect();
    let resultat = (|| -> Result<(), String> {
        let mut sorties = tmp.iter().map(|p| EcrivainTiff::creer(p, l as u32, h as u32, dpi).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let mut bandes: Vec<Vec<u8>> = (0..4).map(|_| Vec::with_capacity(RANGS_PAR_BANDE * l)).collect();
        for y in 0..h {
            let s = spot.ligne()?;
            for ((lecteur, bande), &r) in lecteurs.iter_mut().zip(bandes.iter_mut()).zip(&recette) {
                let v = lecteur.ligne()?;
                bande.extend(v.iter().zip(&s).map(|(&v, &s)| ((v as u32 * (65025 - (255 - s as u32) * r as u32) + 32512) / 65025) as u8));
            }
            if bandes[0].len() == RANGS_PAR_BANDE * l || y + 1 == h {
                for (sortie, bande) in sorties.iter_mut().zip(bandes.iter_mut()) {
                    sortie.bande(bande).map_err(|e| e.to_string())?;
                    bande.clear();
                }
            }
        }
        sorties.into_iter().try_for_each(|s| s.finir().map_err(|e| e.to_string()))
    })();
    if let Err(e) = resultat {
        tmp.iter().for_each(|p| drop(std::fs::remove_file(p)));
        return Err(e);
    }
    tmp.iter().zip(cmjn).try_for_each(|(t, p)| std::fs::rename(t, p).map_err(|e| e.to_string()))
}

/// PDF pour le composite d'aperçu : l'alternative de chaque ton de la table devient sa
/// couleur Lab (teinte 0 = blanc du papier), dans les espaces `Separation`. Rend le PDF et
/// le nombre d'espaces réécrits (0 : le PDF est rendu tel quel).
pub fn en_lab(pdf: &[u8], table: &[Ton]) -> Result<(Vec<u8>, usize), String> {
    use lopdf::{Dictionary, Object};
    fn reel(v: f64) -> Object {
        Object::Real(v as f32)
    }
    fn visiter(o: &mut Object, table: &[Ton], n: &mut usize) {
        if let Object::Array(a) = o {
            let separation = a.first().and_then(|g| g.as_name().ok()) == Some(b"Separation".as_slice());
            let ton = a.get(1).and_then(|x| x.as_name().ok()).and_then(|x| trouver(table, &String::from_utf8_lossy(x)));
            if let (true, Some(t), 4) = (separation, ton, a.len()) {
                let mut blanc = Dictionary::new();
                blanc.set("WhitePoint", vec![reel(0.9642), reel(1.0), reel(0.8249)]);
                blanc.set("Range", vec![reel(-128.0), reel(127.0), reel(-128.0), reel(127.0)]);
                let mut f = Dictionary::new();
                f.set("FunctionType", 2);
                f.set("Domain", vec![reel(0.0), reel(1.0)]);
                f.set("C0", vec![reel(100.0), reel(0.0), reel(0.0)]);
                f.set("C1", t.lab.iter().map(|&v| reel(v)).collect::<Vec<_>>());
                f.set("N", 1);
                a[2] = Object::Array(vec![Object::Name(b"Lab".to_vec()), Object::Dictionary(blanc)]);
                a[3] = Object::Dictionary(f);
                *n += 1;
                return;
            }
        }
        match o {
            Object::Array(a) => a.iter_mut().for_each(|x| visiter(x, table, n)),
            Object::Dictionary(d) => d.iter_mut().for_each(|(_, v)| visiter(v, table, n)),
            Object::Stream(s) => s.dict.iter_mut().for_each(|(_, v)| visiter(v, table, n)),
            _ => {}
        }
    }
    let mut doc = lopdf::Document::load_mem(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    let mut n = 0;
    doc.objects.values_mut().for_each(|o| visiter(o, table, &mut n));
    if n == 0 {
        return Ok((pdf.to_vec(), 0));
    }
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok((sortie, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ton(nom: &str, lab: [f64; 3]) -> Ton {
        Ton { nom: nom.into(), alias: vec![], lab }
    }

    #[test]
    fn csv_cinquante_teintes_et_formats() {
        let mut csv = String::from("\u{feff}Nom;L*;a*;b*\r\n");
        for i in 0..50 {
            csv.push_str(&format!("HX {i:02} Rouge;{},5;-{i},25;12\r\n", 40 + i % 10));
        }
        let t = lire_csv(&csv).unwrap();
        assert_eq!(t.len(), 50);
        assert_eq!(t[3], ton("HX 03 Rouge", [43.5, -3.25, 12.0]));
        assert_eq!(lire_csv("Bleu fonce,30.1,5,-40").unwrap(), [ton("Bleu fonce", [30.1, 5.0, -40.0])]);
        assert_eq!(lire_csv(" Or\t70\t5\t60").unwrap()[0].nom, " Or", "nom gardé tel quel");
        assert_eq!(lire_csv("a;1;2;3\nb;x;2;3").unwrap_err(), "ligne 2 : attendu nom;L;a;b");
        assert!(verifier(&t).is_ok());
    }

    /// Surimpression : v' = v × (1 − t·r) en valeurs « 255 = pas d'encre ».
    #[test]
    fn fondre_en_flux() {
        let d = std::env::temp_dir().join(format!("tons-fondre-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let ecrire = |nom: &str, v: [u8; 3]| {
            let p = d.join(nom);
            let mut e = crate::blanc::EcrivainTiff::creer(&p, 3, 1, 72).unwrap();
            e.bande(&v).unwrap();
            e.finir().unwrap();
            p
        };
        let cmjn = [ecrire("c.tif", [255; 3]), ecrire("m.tif", [255, 255, 128]), ecrire("y.tif", [255; 3]), ecrire("k.tif", [255; 3])];
        // Ton : plein, 50 %, plein (sur un magenta déjà à 50 %).
        let ton = ecrire("t.tif", [0, 128, 0]);
        fondre(cmjn.each_ref().map(PathBuf::as_path), &ton, [0, 255, 204, 0], 72).unwrap();
        let lire = |p: &Path| image::open(p).unwrap().to_luma8().into_raw();
        assert_eq!(lire(&cmjn[0]), [255, 255, 255]);
        assert_eq!(lire(&cmjn[1]), [0, 128, 0]);
        assert_eq!(lire(&cmjn[2]), [51, 153, 51]);
        assert!(!d.join("m.tif.tmp").exists());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn en_lab_reecrit_seulement_les_tons_de_la_table() {
        let pdf = lopdf::Document::with_version("1.4");
        let mut doc = pdf;
        let sep = |nom: &[u8]| lopdf::Object::Array(vec![lopdf::Object::Name(b"Separation".to_vec()), lopdf::Object::Name(nom.to_vec()), lopdf::Object::Name(b"DeviceCMYK".to_vec()), lopdf::Object::Null]);
        doc.add_object(sep(b"HX Rouge"));
        doc.add_object(sep(b"White"));
        let mut octets = vec![];
        doc.save_to(&mut octets).unwrap();
        let (sortie, n) = en_lab(&octets, &[ton("HX Rouge", [48.0, 68.0, 48.0])]).unwrap();
        assert_eq!(n, 1);
        let doc = lopdf::Document::load_mem(&sortie).unwrap();
        let alternatives: Vec<Vec<u8>> = doc.objects.values().filter_map(|o| o.as_array().ok()).map(|a| a[2].as_array().map_or_else(|_| a[2].as_name().unwrap().to_vec(), |x| x[0].as_name().unwrap().to_vec())).collect();
        assert!(alternatives.contains(&b"Lab".to_vec()) && alternatives.contains(&b"DeviceCMYK".to_vec()));
        assert_eq!(en_lab(&octets, &[]).unwrap().1, 0);
    }

    #[test]
    fn recherche_exacte_et_refus() {
        let mut t = ton("PANTONE 485 C", [50.0, 70.0, 55.0]);
        t.alias = vec!["Pantone 485 C".into()];
        let table = [t];
        assert!(trouver(&table, "PANTONE 485 C").is_some());
        assert!(trouver(&table, "Pantone 485 C").is_some(), "alias");
        assert!(trouver(&table, "pantone 485 c").is_none(), "casse exacte");
        assert!(trouver(&table, "PANTONE  485 C").is_none(), "espaces exacts");
        let double = [ton("A", [50.0, 0.0, 0.0]), Ton { nom: "B".into(), alias: vec!["A".into()], lab: [50.0, 0.0, 0.0] }];
        assert_eq!(verifier(&double), Err(("nom_en_double", "A".into())));
        assert_eq!(verifier(&[ton("White", [95.0, 0.0, 0.0])]), Err(("nom_reserve", "White".into())));
        assert_eq!(verifier(&[ton("CutContour", [50.0, 0.0, 0.0])]), Err(("nom_reserve", "CutContour".into())));
        assert_eq!(verifier(&[ton("X", [101.0, 0.0, 0.0])]), Err(("lab_invalide", "X".into())));
        assert_eq!(verifier(&[ton(" ", [50.0, 0.0, 0.0])]), Err(("nom_vide", " ".into())));
    }
}
