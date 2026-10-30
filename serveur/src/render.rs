//! Rendu d'un PDF par Ghostscript : pour chaque page, une plaque 8 bits par encre
//! (CMJN + tons directs).
//!
//! Convention des plaques `tiffsep` (mesurée le 23/10/2026, Ghostscript 10.08) :
//! 0 = pleine encre, 255 = pas d'encre. Un ton direct (`White`, `Silver`…) sort sur
//! sa propre plaque, nommée d'après la séparation du PDF. Une page sans ce ton n'a pas
//! sa plaque : chaque page garde sa liste.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

/// Résolution des aperçus (plaques et composite) ; la couverture d'encre y est mesurée.
pub const DPI_APERCU: u32 = 50;

pub struct Rendu {
    /// Pour chaque page (dans l'ordre), ses plaques en ordre alphabétique.
    pub pages: Vec<Vec<String>>,
    /// Tons directs de la table fondus en CMJN (roadmap 8.6), sans doublon.
    pub tons_fondus: Vec<String>,
}

/// Plaque de production (pleine définition) d'une page.
pub fn chemin_plaque(dossier: &Path, page: usize, nom: &str) -> PathBuf {
    dossier.join("prod").join(format!("{}({nom}).tif", prefixe(page)))
}

/// `plaque-001` : préfixe des fichiers d'une page (numérotée à partir de 1).
fn prefixe(page: usize) -> String {
    format!("plaque-{page:03}")
}

/// Rend `pdf` dans `dossier` : plaques pleine définition (`prod/`), plaques et
/// composites basse définition pour l'aperçu (`apercu/`). `options` : réglages de
/// couleur (profils ICC, intention), appliqués aussi aux aperçus. Un ton direct de `table`
/// est fondu dans C, M, J, N d'après sa couleur Lab, convertie avec l'intention de la table
/// (roadmap 8.7), et sa plaque disparaît.
pub async fn rendre(gs: &str, pdf: &Path, dossier: &Path, dpi: u32, options: &[String], delai: Duration, table: &crate::tons::Table) -> Result<Rendu, String> {
    let prod = dossier.join("prod");
    let apercu = dossier.join("apercu");
    for d in [&prod, &apercu] {
        tokio::fs::create_dir_all(d).await.map_err(|e| e.to_string())?;
    }

    rendre_prod(gs, pdf, &prod, dpi, options, delai, Limites::DEFAUT).await?;
    ghostscript(gs, pdf, "tiffsep", DPI_APERCU, options, &apercu.join("plaque-%03d.tif"), delai).await?;

    let mut pages = plaques_par_page(&prod)?;
    let mut recettes: BTreeMap<String, [u8; 4]> = BTreeMap::new();
    let options_tons: Vec<String> = options.iter().filter(|o| !o.starts_with("-dRenderIntent=")).cloned().chain([format!("-dRenderIntent={}", table.intention)]).collect();
    for (i, plaques) in pages.iter_mut().enumerate() {
        let n = i + 1;
        for nom in plaques.clone() {
            let Some(ton) = crate::tons::trouver(&table.tons, &nom) else { continue };
            let recette = match recettes.get(&nom) {
                Some(r) => *r,
                None => *recettes.entry(nom.clone()).or_insert(crate::tons::recette(gs, &options_tons, ton.lab, delai).await?),
            };
            for (d, r) in [(&prod, dpi), (&apercu, DPI_APERCU)] {
                let cmjn = ["Cyan", "Magenta", "Yellow", "Black"].map(|c| d.join(format!("{}({c}).tif", prefixe(n))));
                let spot = d.join(format!("{}({nom}).tif", prefixe(n)));
                let (c2, s2) = (cmjn.clone(), spot.clone());
                tokio::task::spawn_blocking(move || crate::tons::fondre(c2.each_ref().map(PathBuf::as_path), &s2, recette, r))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| format!("ton direct « {nom} », page {n} : {e}"))?;
                let _ = std::fs::remove_file(&spot);
            }
            plaques.retain(|p| *p != nom);
        }
    }
    // L'aperçu est en RVB : le profil du support (CMJN) y sert d'épreuve écran. Les tons
    // fondus y prennent leur couleur Lab plutôt que l'alternative du PDF.
    let epreuve: Vec<String> = options.iter().map(|o| o.replace("-sOutputICCProfile=", "-sProofProfile=")).collect();
    let pdf_lab = dossier.join("apercu-lab.pdf");
    let source_apercu = if recettes.is_empty() {
        pdf.to_path_buf()
    } else {
        let octets = std::fs::read(pdf).map_err(|e| e.to_string())?;
        let (lab, _) = crate::tons::en_lab(&octets, &table.tons)?;
        std::fs::write(&pdf_lab, lab).map_err(|e| e.to_string())?;
        pdf_lab.clone()
    };
    let composite = ghostscript(gs, &source_apercu, "png16m", DPI_APERCU, &epreuve, &apercu.join("composite-%03d.png"), delai).await;
    let _ = std::fs::remove_file(&pdf_lab);
    composite?;
    for (i, plaques) in pages.iter().enumerate() {
        for nom in plaques {
            let tif = apercu.join(format!("{}({nom}).tif", prefixe(i + 1)));
            image::open(&tif)
                .and_then(|img| img.save(chemin_apercu_plaque(dossier, i + 1, nom)))
                .map_err(|e| format!("aperçu de la plaque {nom}, page {} : {e}", i + 1))?;
            let _ = std::fs::remove_file(&tif);
        }
        // Le composite CMJN 32 bits (≈ 40 % de la place) ne sert ni au pilote, qui lit les
        // plaques, ni à l'aperçu, en PNG.
        for d in [&prod, &apercu] {
            let _ = std::fs::remove_file(d.join(format!("{}.tif", prefixe(i + 1))));
        }
    }
    Ok(Rendu { pages, tons_fondus: recettes.into_keys().collect() })
}

/// Seuils du rendu par bandes (roadmap 9.16), en octets non compressés.
#[derive(Clone, Copy)]
struct Limites {
    /// Composite CMJN de `tiffsep` (4 octets par point) au-delà duquel une page est rendue
    /// par bandes : Ghostscript l'écrit en TIFF classique, qui ne dépasse pas 4 Go
    /// (`UseBigTIFF` ignoré par ce périphérique, mesuré le 25/10 sur gs 10.08).
    composite: u64,
    /// Composite d'une bande.
    bande: u64,
    /// Plaque recollée au-delà de laquelle elle est écrite en BigTIFF.
    tiff_classique: u64,
}

impl Limites {
    const DEFAUT: Limites = Limites { composite: 3 << 30, bande: 1 << 30, tiff_classique: 3_584 << 20 };
}

/// Plaques de production (`prod/plaque-NNN(encre).tif`). Une page dont le composite
/// dépasserait la limite est rendue par bandes horizontales (une MediaBox par bande, sur la
/// grille de points de la page entière), puis ses plaques sont recollées bande par bande ;
/// les autres pages se rendent d'un trait, une à une.
async fn rendre_prod(gs: &str, pdf: &Path, prod: &Path, dpi: u32, options: &[String], delai: Duration, l: Limites) -> Result<(), String> {
    let source = pdf.to_path_buf();
    let boites = tokio::task::spawn_blocking(move || boites_pages(&source)).await.map_err(|e| e.to_string())??;
    let points = |pt: f64| (pt * dpi as f64 / 72.0).round().max(1.0) as u64;
    let grande = |b: &[f64; 4]| points(b[2] - b[0]) * points(b[3] - b[1]) * 4 > l.composite;
    if !boites.iter().any(grande) {
        return ghostscript(gs, pdf, "tiffsep", dpi, options, &prod.join("plaque-%03d.tif"), delai).await;
    }
    for (i, b) in boites.iter().enumerate() {
        let n = i + 1;
        let tmp = prod.join(format!("bandes-{n:03}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let resultat = if grande(b) {
            rendre_par_bandes(gs, pdf, n, *b, prod, &tmp, dpi, options, delai, l).await
        } else {
            let page = [format!("-dFirstPage={n}"), format!("-dLastPage={n}")];
            let opts: Vec<String> = options.iter().cloned().chain(page).collect();
            match ghostscript(gs, pdf, "tiffsep", dpi, &opts, &tmp.join("plaque-%03d.tif"), delai).await {
                Ok(()) => renommer_page(&tmp, prod, n),
                Err(e) => Err(e),
            }
        };
        let _ = std::fs::remove_dir_all(&tmp);
        resultat?;
    }
    Ok(())
}

/// MediaBox de chaque page (déjà normalisée : ni CropBox ni /Rotate, voir
/// `imbrication::normaliser_pages`).
fn boites_pages(pdf: &Path) -> Result<Vec<[f64; 4]>, String> {
    let doc = lopdf::Document::load(pdf).map_err(|e| format!("PDF illisible : {e}"))?;
    doc.get_pages()
        .into_iter()
        .map(|(n, id)| {
            let mut d = doc.get_dictionary(id).map_err(|e| e.to_string())?;
            let boite = loop {
                if let Ok(b) = d.get(b"MediaBox") {
                    break doc.dereference(b).map_err(|e| e.to_string())?.1.as_array().map_err(|e| e.to_string())?.clone();
                }
                let parent = d.get(b"Parent").and_then(lopdf::Object::as_reference).map_err(|_| format!("page {n} sans MediaBox"))?;
                d = doc.get_dictionary(parent).map_err(|e| e.to_string())?;
            };
            let v: Vec<f64> = boite.iter().filter_map(|o| o.as_float().map(f64::from).ok().or(o.as_i64().ok().map(|i| i as f64))).collect();
            match v[..] {
                [a, b, c, d] => Ok([a.min(c), b.min(d), a.max(c), b.max(d)]),
                _ => Err(format!("page {n} : MediaBox illisible")),
            }
        })
        .collect()
}

/// `tmp/plaque-001(…).tif` → `prod/plaque-NNN(…).tif`.
fn renommer_page(tmp: &Path, prod: &Path, n: usize) -> Result<(), String> {
    for e in std::fs::read_dir(tmp).map_err(|e| e.to_string())?.flatten() {
        let Ok(f) = e.file_name().into_string() else { continue };
        if let Some(reste) = f.strip_prefix("plaque-001") {
            std::fs::rename(e.path(), prod.join(format!("{}{reste}", prefixe(n)))).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Page `n` rendue par bandes de `l.bande` au plus, comptées depuis le bas sur la grille de
/// la page entière (la dernière, en haut, prend le reste) : chaque bande tombe sur des
/// lignes entières de points et le recollage est exact.
#[allow(clippy::too_many_arguments)]
async fn rendre_par_bandes(gs: &str, pdf: &Path, n: usize, b: [f64; 4], prod: &Path, tmp: &Path, dpi: u32, options: &[String], delai: Duration, l: Limites) -> Result<(), String> {
    let pt = 72.0 / dpi as f64;
    let largeur = (((b[2] - b[0]) / pt).round() as u64).max(1);
    let hauteur = (((b[3] - b[1]) / pt).round() as u64).max(1);
    let lignes = (l.bande / (largeur * 4)).max(1);
    let source = pdf.to_path_buf();
    let page = tokio::task::spawn_blocking(move || std::fs::read(&source).map_err(|e| e.to_string()).and_then(|o| crate::imbrication::pages_separees(&o)))
        .await
        .map_err(|e| e.to_string())??
        .into_iter()
        .nth(n - 1)
        .ok_or(format!("pas de page {n}"))?;
    // (dossier, lignes) de chaque bande, du haut vers le bas de l'image.
    let mut bandes = vec![];
    let mut bas = 0;
    while bas < hauteur {
        let haut = (bas + lignes).min(hauteur);
        let y_haut = if haut == hauteur { b[3] } else { b[1] + haut as f64 * pt };
        let boite = [b[0], b[1] + bas as f64 * pt, b[2], y_haut];
        let dossier = tmp.join(format!("{bas:09}"));
        std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
        let chemin = dossier.join("bande.pdf");
        let p = page.clone();
        let c = chemin.clone();
        tokio::task::spawn_blocking(move || pdf_bande(&p, boite, &c)).await.map_err(|e| e.to_string())??;
        ghostscript(gs, &chemin, "tiffsep", dpi, options, &dossier.join("plaque-%03d.tif"), delai).await?;
        let _ = std::fs::remove_file(&chemin);
        // Composite de la bande : inutile au recollage.
        let _ = std::fs::remove_file(dossier.join("plaque-001.tif"));
        bandes.push((dossier, (haut - bas) as u32));
        bas = haut;
    }
    bandes.reverse();
    let mut noms: Vec<String> = vec![];
    for (d, _) in &bandes {
        noms.extend(plaques_par_page(d).unwrap_or_default().into_iter().flatten());
    }
    noms.sort();
    noms.dedup();
    if noms.is_empty() {
        return Err("Ghostscript n'a produit aucune plaque".into());
    }
    let (prod, bandes) = (prod.to_path_buf(), std::sync::Arc::new(bandes));
    for nom in noms {
        let (sortie, bandes) = (prod.join(format!("{}({nom}).tif", prefixe(n))), bandes.clone());
        let grosse = largeur * hauteur > l.tiff_classique;
        tokio::task::spawn_blocking(move || recoller(&bandes, &nom, largeur as u32, hauteur as u32, dpi, grosse, &sortie)).await.map_err(|e| e.to_string())??;
    }
    Ok(())
}

/// PDF d'une page dont la MediaBox est réduite à `boite`.
fn pdf_bande(page: &[u8], boite: [f64; 4], sortie: &Path) -> Result<(), String> {
    let mut doc = lopdf::Document::load_mem(page).map_err(|e| e.to_string())?;
    let id = *doc.get_pages().get(&1).ok_or("page absente")?;
    let d = doc.get_dictionary_mut(id).map_err(|e| e.to_string())?;
    d.set("MediaBox", boite.iter().map(|&v| lopdf::Object::Real(v as f32)).collect::<Vec<_>>());
    for cle in [&b"CropBox"[..], b"BleedBox", b"TrimBox", b"ArtBox"] {
        d.remove(cle);
    }
    doc.save(sortie).map(|_| ()).map_err(|e| e.to_string())
}

/// Plaque `nom` recollée depuis les bandes (du haut vers le bas), écrite par morceaux
/// compressés en LZW, comme celles de Ghostscript : la page entière n'est jamais en
/// mémoire. Une bande sans cette plaque n'a pas d'encre (255).
fn recoller(bandes: &[(PathBuf, u32)], nom: &str, largeur: u32, hauteur: u32, dpi: u32, grosse: bool, sortie: &Path) -> Result<(), String> {
    let f = std::io::BufWriter::new(std::fs::File::create(sortie).map_err(|e| e.to_string())?);
    let r = if grosse {
        ecrire_plaque(tiff::encoder::TiffEncoder::new_big(f).map_err(|e| e.to_string())?, bandes, nom, largeur, hauteur, dpi, true)
    } else {
        ecrire_plaque(tiff::encoder::TiffEncoder::new(f).map_err(|e| e.to_string())?, bandes, nom, largeur, hauteur, dpi, false)
    };
    r.map_err(|e| format!("plaque {nom} : {e}"))
}

/// Écrit les morceaux puis l'en-tête de l'image à la main : `ImageEncoder::write_strip` de
/// `tiff` 0.11 annonce la compression sans l'appliquer.
fn ecrire_plaque<K: tiff::encoder::TiffKind>(mut enc: tiff::encoder::TiffEncoder<std::io::BufWriter<std::fs::File>, K>, bandes: &[(PathBuf, u32)], nom: &str, largeur: u32, hauteur: u32, dpi: u32, grosse: bool) -> Result<(), String> {
    use tiff::tags::Tag;
    let err = |e: tiff::TiffError| e.to_string();
    let mut dir = enc.image_directory().map_err(err)?;
    // Morceaux d'environ 1 Mo.
    let par_morceau = ((1 << 20) / largeur.max(1)).clamp(1, hauteur);
    let morceau = par_morceau as usize * largeur as usize;
    let (mut positions, mut longueurs): (Vec<u64>, Vec<u64>) = (vec![], vec![]);
    let mut ecrire = |dir: &mut tiff::encoder::DirectoryEncoder<'_, _, K>, brut: &[u8]| -> Result<(), String> {
        let lzw = weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8).encode(brut).map_err(|e| e.to_string())?;
        positions.push(dir.write_data(&lzw[..]).map_err(err)?);
        longueurs.push(lzw.len() as u64);
        Ok(())
    };
    let mut tampon: Vec<u8> = Vec::with_capacity(morceau);
    for (dossier, lignes) in bandes {
        let attendu = *lignes as usize * largeur as usize;
        let chemin = dossier.join(format!("{}({nom}).tif", prefixe(1)));
        let octets = if chemin.exists() {
            let f = std::io::BufReader::new(std::fs::File::open(&chemin).map_err(|e| e.to_string())?);
            let mut d = tiff::decoder::Decoder::new(f).map_err(err)?.with_limits(tiff::decoder::Limits::unlimited());
            match d.read_image().map_err(err)? {
                tiff::decoder::DecodingResult::U8(v) if v.len() == attendu => v,
                _ => return Err(format!("bande {} de taille inattendue", chemin.display())),
            }
        } else {
            vec![255; attendu]
        };
        let mut reste = &octets[..];
        while !reste.is_empty() {
            let pris = (morceau - tampon.len()).min(reste.len());
            tampon.extend_from_slice(&reste[..pris]);
            reste = &reste[pris..];
            if tampon.len() == morceau {
                ecrire(&mut dir, &tampon)?;
                tampon.clear();
            }
        }
    }
    if !tampon.is_empty() {
        ecrire(&mut dir, &tampon)?;
    }
    dir.write_tag(Tag::ImageWidth, largeur).map_err(err)?;
    dir.write_tag(Tag::ImageLength, hauteur).map_err(err)?;
    dir.write_tag(Tag::BitsPerSample, 8u16).map_err(err)?;
    dir.write_tag(Tag::Compression, 5u16).map_err(err)?;
    dir.write_tag(Tag::PhotometricInterpretation, 1u16).map_err(err)?;
    dir.write_tag(Tag::SamplesPerPixel, 1u16).map_err(err)?;
    dir.write_tag(Tag::RowsPerStrip, par_morceau).map_err(err)?;
    if grosse {
        dir.write_tag(Tag::StripOffsets, &positions[..]).map_err(err)?;
        dir.write_tag(Tag::StripByteCounts, &longueurs[..]).map_err(err)?;
    } else {
        let court = |v: &[u64]| v.iter().map(|&x| u32::try_from(x).map_err(|_| "TIFF classique dépassé".to_string())).collect::<Result<Vec<u32>, _>>();
        dir.write_tag(Tag::StripOffsets, &court(&positions)?[..]).map_err(err)?;
        dir.write_tag(Tag::StripByteCounts, &court(&longueurs)?[..]).map_err(err)?;
    }
    let r = tiff::encoder::Rational { n: dpi, d: 1 };
    dir.write_tag(Tag::XResolution, r.clone()).map_err(err)?;
    dir.write_tag(Tag::YResolution, r).map_err(err)?;
    dir.write_tag(Tag::ResolutionUnit, 2u16).map_err(err)?;
    dir.finish().map_err(err)
}

pub(crate) async fn ghostscript(gs: &str, pdf: &Path, device: &str, dpi: u32, options: &[String], sortie: &Path, delai: Duration) -> Result<(), String> {
    let mut cmd = Command::new(gs);
    cmd.args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE"])
        .arg(format!("-sDEVICE={device}"))
        .arg(format!("-r{dpi}"))
        .args(options)
        .arg(format!("-sOutputFile={}", sortie.display()))
        .arg(pdf)
        .kill_on_drop(true);
    let sortie = tokio::time::timeout(delai, cmd.output())
        .await
        .map_err(|_| format!("Ghostscript a dépassé {} s", delai.as_secs()))?
        .map_err(|e| format!("Ghostscript introuvable ({gs}) : {e}"))?;
    if !sortie.status.success() {
        let err = String::from_utf8_lossy(&sortie.stderr);
        let out = String::from_utf8_lossy(&sortie.stdout);
        return Err(format!("Ghostscript ({device}) a échoué : {}{}", err.trim(), out.trim()));
    }
    Ok(())
}

/// `plaque-002(Cyan).tif` → page 2, plaque `Cyan`. Le composite `plaque-002.tif` n'est
/// pas une plaque, mais il marque l'existence de la page.
fn plaques_par_page(dossier: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut pages: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for entree in std::fs::read_dir(dossier).map_err(|e| e.to_string())?.flatten() {
        let Ok(f) = entree.file_name().into_string() else { continue };
        let Some(reste) = f.strip_prefix("plaque-").and_then(|r| r.strip_suffix(".tif")) else { continue };
        let (numero, plaque) = match reste.split_once('(') {
            Some((n, p)) => (n, p.strip_suffix(')')),
            None => (reste, None),
        };
        let Ok(numero) = numero.parse::<usize>() else { continue };
        let liste = pages.entry(numero).or_default();
        if let Some(p) = plaque {
            liste.push(p.to_string());
        }
    }
    let pages: Vec<Vec<String>> = pages
        .into_values()
        .map(|mut p| {
            p.sort();
            p
        })
        .collect();
    if pages.is_empty() || pages.iter().any(Vec::is_empty) {
        return Err("Ghostscript n'a produit aucune plaque".into());
    }
    Ok(pages)
}

pub fn chemin_apercu_plaque(dossier: &Path, page: usize, nom: &str) -> PathBuf {
    dossier.join("apercu").join(format!("{}({nom}).png", prefixe(page)))
}

pub fn chemin_composite(dossier: &Path, page: usize) -> PathBuf {
    dossier.join("apercu").join(format!("composite-{page:03}.png"))
}

/// Composite d'avant le filtre couleur (vue scindée avant/après), s'il y a un filtre.
pub fn chemin_composite_avant(dossier: &Path, page: usize) -> PathBuf {
    dossier.join("apercu").join(format!("composite-{page:03}-avant.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PDF de deux pages : page 1 = carré CMJN, carré `White` plein, carré `Silver` à
    /// 50 % ; page 2 = magenta seul, sans ton direct.
    fn pdf_deux_pages() -> Vec<u8> {
        let contenu1 = "0.2 0.6 0 0 k 10 10 80 80 re f\n/CS1 cs 1 scn 100 10 80 80 re f\n/CS2 cs 0.5 scn 190 10 80 80 re f\n";
        let contenu2 = "0 1 0 0 k 0 0 50 50 re f\n";
        let sep = |nom: &str, k: f32| {
            format!("[/Separation /{nom} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 {k}] /N 1 >>]")
        };
        let objets = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R 7 0 R] /Count 2 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 280 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R /CS2 6 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{contenu1}endstream", contenu1.len()),
            sep("White", 0.1),
            sep("Silver", 0.4),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 8 0 R >>".to_string(),
            format!("<< /Length {} >>\nstream\n{contenu2}endstream", contenu2.len()),
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
        pdf.into_bytes()
    }

    /// Valeur du pixel au centre vertical, à l'abscisse `x` (en points PDF).
    fn pixel(dossier: &Path, page: usize, plaque: &str, x_pt: f32, dpi: u32) -> u8 {
        let img = image::open(dossier.join(format!("prod/{}({plaque}).tif", prefixe(page)))).unwrap().to_luma8();
        let x = (x_pt * dpi as f32 / 72.0) as u32;
        img.get_pixel(x, img.height() / 2)[0]
    }

    #[tokio::test]
    async fn chaque_page_a_ses_plaques_et_les_tons_directs_les_leurs() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-test-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let pdf = dossier.join("source.pdf");
        std::fs::write(&pdf, pdf_deux_pages()).unwrap();

        let dpi = 72;
        let r = rendre("gs", &pdf, &dossier, dpi, &[], Duration::from_secs(60), &Default::default()).await.unwrap();

        assert_eq!(r.pages.len(), 2);
        assert_eq!(r.pages[0], ["Black", "Cyan", "Magenta", "Silver", "White", "Yellow"]);
        assert_eq!(r.pages[1], ["Black", "Cyan", "Magenta", "Yellow"], "pas de ton direct en page 2");
        // 0 = pleine encre, 255 = pas d'encre.
        assert_eq!(pixel(&dossier, 1, "White", 140.0, dpi), 0, "blanc plein sous le carré blanc");
        assert_eq!(pixel(&dossier, 1, "White", 50.0, dpi), 255, "pas de blanc ailleurs");
        let argent = pixel(&dossier, 1, "Silver", 230.0, dpi);
        assert!((126..=129).contains(&argent), "argent à 50 % : {argent}");
        assert_eq!(pixel(&dossier, 1, "Cyan", 50.0, dpi), 204, "cyan à 20 %");
        assert_eq!(pixel(&dossier, 1, "Black", 140.0, dpi), 255, "le blanc ne doit pas fuir dans le noir");
        assert_eq!(pixel(&dossier, 2, "Magenta", 25.0, dpi), 0, "page 2 : magenta plein");
        assert!(chemin_apercu_plaque(&dossier, 1, "White").exists());
        assert!(chemin_composite(&dossier, 2).exists());
        assert!(!dossier.join("prod/plaque-001.tif").exists(), "composite TIFF effacé");
        assert!(!dossier.join("apercu/plaque-001(Cyan).tif").exists(), "TIFF d'aperçu effacé après conversion");

        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Roadmap 8.6 : un ton de la table (alternative du PDF volontairement fausse : cyan pur)
    /// est imprimé d'après sa couleur Lab (rouge) et sa plaque disparaît ; sans table, il
    /// garde sa plaque. Le ton hors table (`White`) n'est pas touché.
    #[tokio::test]
    async fn ton_de_la_table_fondu_en_cmjn() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-tons-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let pdf = dossier.join("source.pdf");
        let contenu = "/R cs 1 scn 0 0 50 100 re f\n/R cs 0.5 scn 50 0 50 100 re f\n/W cs 1 scn 100 0 50 100 re f\n";
        let objets = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 150 100] /Contents 4 0 R /Resources << /ColorSpace << /R [/Separation /HX#20Rouge /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [1 0 0 0] /N 1 >>] /W [/Separation /White /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 0.1] /N 1 >>] >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{contenu}endstream", contenu.len()),
        ];
        let mut texte = String::from("%PDF-1.4\n");
        let mut positions = vec![];
        for (i, o) in objets.iter().enumerate() {
            positions.push(texte.len());
            texte += &format!("{} 0 obj\n{o}\nendobj\n", i + 1);
        }
        let xref = texte.len();
        texte += &format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1);
        for p in positions {
            texte += &format!("{p:010} 00000 n \n");
        }
        texte += &format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1);
        std::fs::write(&pdf, texte).unwrap();

        let r = rendre("gs", &pdf, &dossier, 72, &[], Duration::from_secs(60), &Default::default()).await.unwrap();
        assert_eq!(r.pages[0], ["Black", "Cyan", "HX Rouge", "Magenta", "White", "Yellow"], "sans table : plaque gardée");
        assert!(r.tons_fondus.is_empty());

        let table = crate::tons::Table { tons: vec![crate::tons::Ton { nom: "HX Rouge".into(), alias: vec![], lab: [48.0, 68.0, 48.0] }], ..Default::default() };
        let r = rendre("gs", &pdf, &dossier, 72, &[], Duration::from_secs(60), &table).await.unwrap();
        assert_eq!(r.pages[0], ["Black", "Cyan", "Magenta", "White", "Yellow"], "plaque fondue");
        assert_eq!(r.tons_fondus, ["HX Rouge"]);
        assert!(!dossier.join("prod/plaque-001(HX Rouge).tif").exists());
        assert!(!dossier.join("apercu/plaque-001(HX Rouge).tif").exists());
        let (c, m, y) = (pixel(&dossier, 1, "Cyan", 25.0, 72), pixel(&dossier, 1, "Magenta", 25.0, 72), pixel(&dossier, 1, "Yellow", 25.0, 72));
        assert!(m < 60 && y < 110 && c > 200, "rouge Lab, pas le cyan de l'alternative : C {c} M {m} J {y}");
        let m50 = pixel(&dossier, 1, "Magenta", 75.0, 72);
        assert!(m50 > m + 60 && m50 < 230, "teinte 50 % : moitié de la recette ({m} → {m50})");
        assert_eq!(pixel(&dossier, 1, "White", 125.0, 72), 0, "ton hors table intact");
        assert_eq!(pixel(&dossier, 1, "Magenta", 125.0, 72), 255);
        // Composite d'aperçu : rouge (alternative Lab), pas cyan.
        let px = image::open(chemin_composite(&dossier, 1)).unwrap().to_rgb8().get_pixel(10, 30).0;
        assert!(px[0] > 150 && px[2] < 120, "composite rouge : {px:?}");
        assert!(!dossier.join("apercu-lab.pdf").exists());
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// PDF de pages (MediaBox, contenu), avec l'espace `/W` = ton direct `White`.
    fn pdf_pages(pages: &[([f64; 4], &str)]) -> Vec<u8> {
        let mut objets = vec!["<< /Type /Catalog /Pages 2 0 R >>".to_string()];
        let enfants: Vec<String> = (0..pages.len()).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
        objets.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", enfants.join(" "), pages.len()));
        objets.push("[/Separation /White /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 0.1] /N 1 >>]".to_string());
        for (i, (b, contenu)) in pages.iter().enumerate() {
            objets.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [{} {} {} {}] /Contents {} 0 R /Resources << /ColorSpace << /W 3 0 R >> >> >>", b[0], b[1], b[2], b[3], 5 + 2 * i));
            objets.push(format!("<< /Length {} >>\nstream\n{contenu}endstream", contenu.len()));
        }
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
        pdf.into_bytes()
    }

    /// Plaques d'une page (nom → pixels), et les dimensions.
    fn plaques(prod: &Path, page: usize) -> BTreeMap<String, (u32, u32, Vec<u8>)> {
        plaques_par_page(prod).unwrap()[page - 1]
            .iter()
            .map(|n| {
                let img = image::open(chemin_plaque(prod.parent().unwrap(), page, n)).unwrap().to_luma8();
                (n.clone(), (img.width(), img.height(), img.into_raw()))
            })
            .collect()
    }

    /// Roadmap 9.16 : une page rendue par bandes (petites ici, seuils abaissés) donne les
    /// mêmes plaques, point pour point, que d'un trait ; les pages d'à côté gardent leur
    /// numéro ; un ton direct présent dans une seule bande a sa plaque entière.
    #[tokio::test]
    async fn rendu_par_bandes_identique() {
        let racine = std::env::temp_dir().join(format!("eko-rip-bandes-{}", std::process::id()));
        // Hauteur non entière en points, diagonales et cercle à cheval sur les bandes, blanc en haut seulement.
        let grande = "0.3 0.7 0 0.1 k 0 0 m 300 500.6 l 300 0 l f\n0 0 1 0 k 150 250 m 250 250 250 350 150 350 c 50 350 50 250 150 250 c f\n/W cs 1 scn 20 440 260 50 re f\n0 0 0 1 k 0 0 3 500.6 re f\n";
        let petite = "1 0 0 0 k 10 10 50 50 re f\n";
        let pdf = racine.join("source.pdf");
        std::fs::create_dir_all(&racine).unwrap();
        std::fs::write(&pdf, pdf_pages(&[([0.0, 0.0, 100.0, 100.0], petite), ([0.0, 0.0, 300.0, 500.6], grande), ([0.0, 0.0, 80.0, 60.0], petite)])).unwrap();
        let rendre_dans = |nom: &str, l: Limites| {
            let prod = racine.join(nom).join("prod");
            let pdf = pdf.clone();
            async move {
                std::fs::create_dir_all(&prod).unwrap();
                rendre_prod("gs", &pdf, &prod, 72, &[], Duration::from_secs(60), l).await.unwrap_or_else(|e| panic!("{prod:?} : {e}"));
                prod
            }
        };
        let entier = rendre_dans("entier", Limites::DEFAUT).await;
        // Seuil entre les pages 1 et 2 ; bandes de 37 lignes (300 points de large).
        let limites = Limites { composite: 100 * 100 * 4, bande: 300 * 4 * 37, tiff_classique: u64::MAX };
        let bandes = rendre_dans("bandes", limites).await;
        for page in 1..=3 {
            let (a, b) = (plaques(&entier, page), plaques(&bandes, page));
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "page {page}");
            for (nom, (l, h, px)) in &a {
                let (l2, h2, px2) = &b[nom];
                assert_eq!((l, h), (l2, h2), "page {page} {nom}");
                let ecarts = px.iter().zip(px2).filter(|(x, y)| x != y).count();
                assert_eq!(ecarts, 0, "page {page} {nom} : {ecarts} points différents");
            }
        }
        assert_eq!(plaques(&bandes, 2)["White"].2.iter().filter(|&&v| v == 0).count(), 260 * 50, "blanc entier");
        assert!(std::fs::read_dir(&bandes).unwrap().flatten().all(|e| e.file_type().unwrap().is_file()), "dossiers de bandes effacés");
        // Au-delà du TIFF classique : BigTIFF, lisible pareil.
        let gros = rendre_dans("bigtiff", Limites { tiff_classique: 0, ..limites }).await;
        let entete = std::fs::read(chemin_plaque(gros.parent().unwrap(), 2, "Cyan")).unwrap();
        assert_eq!(&entete[..4], b"II\x2b\x00", "BigTIFF");
        assert_eq!(plaques(&gros, 2)["Cyan"], plaques(&entier, 2)["Cyan"]);
        std::fs::remove_dir_all(&racine).unwrap();
    }

    /// Roadmap 9.16, à la taille réelle : bâche 1,6 × 3 m à 720 dpi (composite ≈ 15 Go).
    /// Long (plusieurs minutes) et gourmand en disque : `cargo test -- --ignored bache`.
    #[tokio::test]
    #[ignore]
    async fn bache_1600_x_3000_a_720_dpi() {
        let racine = std::env::temp_dir().join(format!("eko-rip-bache-{}", std::process::id()));
        let prod = racine.join("prod");
        std::fs::create_dir_all(&prod).unwrap();
        let (l, h) = (1600.0 * 72.0 / 25.4, 3000.0 * 72.0 / 25.4);
        let contenu = format!("0.6 0.2 0 0 k 0 0 {l} {h} re f\n0 1 0 0 k 0 0 m {l} {h} l {l} 0 l f\n/W cs 1 scn 100 {} 500 500 re f\n", h - 600.0);
        let pdf = racine.join("source.pdf");
        std::fs::write(&pdf, pdf_pages(&[([0.0, 0.0, l, h], &contenu)])).unwrap();
        rendre_prod("gs", &pdf, &prod, 720, &[], Duration::from_secs(3600), Limites::DEFAUT).await.unwrap();
        let (lp, hp) = ((1600.0 / 25.4 * 720.0f64).round() as u32, (3000.0 / 25.4 * 720.0f64).round() as u32);
        for nom in ["Black", "Cyan", "Magenta", "White", "Yellow"] {
            let chemin = chemin_plaque(&racine, 1, nom);
            let taille = std::fs::metadata(&chemin).unwrap().len();
            let entete = std::fs::read(&chemin).map(|o| o[..4].to_vec()).unwrap();
            assert!(taille < 4 << 30 || entete == b"II\x2b\x00", "{nom} : {taille} octets en TIFF classique");
            let d = tiff::decoder::Decoder::new(std::fs::File::open(&chemin).unwrap()).unwrap().dimensions().unwrap();
            assert_eq!(d, (lp, hp), "{nom}");
        }
        assert!(!prod.join("plaque-001.tif").exists(), "pas de composite");
        std::fs::remove_dir_all(&racine).unwrap();
    }

    /// Un profil de sortie convertit les couleurs : le cyan à 20 % ne reste pas tel quel.
    #[tokio::test]
    async fn le_profil_de_sortie_convertit() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-icc-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let pdf = dossier.join("source.pdf");
        std::fs::write(&pdf, pdf_deux_pages()).unwrap();
        let profil = crate::couleur::tests::profils_ghostscript().join("ps_cmyk.icc");
        let options = [format!("-sOutputICCProfile={}", profil.display())];
        rendre("gs", &pdf, &dossier, 72, &options, Duration::from_secs(60), &Default::default()).await.unwrap();
        let (c, m) = (pixel(&dossier, 1, "Cyan", 50.0, 72), pixel(&dossier, 1, "Magenta", 50.0, 72));
        assert_ne!((c, m), (204, 102), "valeurs inchangées : le profil n'a pas été appliqué");
        assert_eq!(pixel(&dossier, 1, "White", 140.0, 72), 0, "un ton direct n'est pas converti");
        std::fs::remove_dir_all(&dossier).unwrap();
    }

    /// Roadmap 9.15 : mire 64 × 64 px en noir à 100 ppi, rendue à 400 dpi (×4). Le
    /// rééchantillonnage lissé suit bien mieux la mire continue que le plus proche voisin.
    #[tokio::test]
    async fn reechantillonnage_lisse_plus_fidele_a_l_agrandissement() {
        let mire = |u: f64, v: f64| 127.5 + 127.5 * (u * 0.35).sin() * (v * 0.27).cos();
        let cote = 64usize;
        let pt = cote as f64 * 72.0 / 100.0;
        let mut px = Vec::with_capacity(cote * cote * 4);
        for y in 0..cote {
            for x in 0..cote {
                px.extend([0, 0, 0, 255 - mire(x as f64 + 0.5, y as f64 + 0.5).round() as u8]);
            }
        }
        let contenu = format!("q {pt} 0 0 {pt} 0 0 cm /I Do Q");
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut positions = vec![];
        let objets: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {pt} {pt}] /Contents 4 0 R /Resources << /XObject << /I 5 0 R >> >> >>").into_bytes(),
            format!("<< /Length {} >>\nstream\n{contenu}\nendstream", contenu.len()).into_bytes(),
            [format!("<< /Type /XObject /Subtype /Image /Width {cote} /Height {cote} /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length {} >>\nstream\n", px.len()).into_bytes(), px, b"\nendstream".to_vec()].concat(),
        ];
        for (i, o) in objets.iter().enumerate() {
            positions.push(pdf.len());
            pdf.extend(format!("{} 0 obj\n", i + 1).bytes());
            pdf.extend(o);
            pdf.extend(b"\nendobj\n");
        }
        let xref = pdf.len();
        pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objets.len() + 1).bytes());
        for p in positions {
            pdf.extend(format!("{p:010} 00000 n \n").bytes());
        }
        pdf.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objets.len() + 1).bytes());

        let pdf = &pdf;
        let psnr = |reechantillonnage| async move {
            let dossier = std::env::temp_dir().join(format!("eko-rip-reech-{reechantillonnage:?}-{}", std::process::id()));
            std::fs::create_dir_all(&dossier).unwrap();
            let source = dossier.join("source.pdf");
            std::fs::write(&source, pdf).unwrap();
            let options = crate::reglages::Demande { reechantillonnage, ..Default::default() }.options_ghostscript(Path::new("/profils"));
            rendre("gs", &source, &dossier, 400, &options, Duration::from_secs(60), &Default::default()).await.unwrap();
            let img = image::open(dossier.join(format!("prod/{}(Black).tif", prefixe(1)))).unwrap().to_luma8();
            assert_eq!(img.dimensions(), (256, 256));
            // Bords exclus : chaque filtre y traite la sortie de l'image à sa façon.
            let (mut somme, mut n) = (0.0, 0.0);
            for y in 8..248 {
                for x in 8..248 {
                    let e = img.get_pixel(x, y)[0] as f64 - mire((x as f64 + 0.5) / 4.0, (y as f64 + 0.5) / 4.0);
                    somme += e * e;
                    n += 1.0;
                }
            }
            std::fs::remove_dir_all(&dossier).unwrap();
            10.0 * (255.0f64 * 255.0 / (somme / n)).log10()
        };
        let proche = psnr(crate::reglages::Reechantillonnage::ProcheVoisin).await;
        let lisse = psnr(crate::reglages::Reechantillonnage::Lisse).await;
        assert!(lisse > proche + 10.0, "PSNR lissé {lisse:.1} dB, plus proche voisin {proche:.1} dB");
    }
}
