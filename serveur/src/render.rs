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
/// couleur (profils ICC, intention), appliqués aussi aux aperçus.
pub async fn rendre(gs: &str, pdf: &Path, dossier: &Path, dpi: u32, options: &[String], delai: Duration) -> Result<Rendu, String> {
    let prod = dossier.join("prod");
    let apercu = dossier.join("apercu");
    for d in [&prod, &apercu] {
        tokio::fs::create_dir_all(d).await.map_err(|e| e.to_string())?;
    }

    ghostscript(gs, pdf, "tiffsep", dpi, options, &prod.join("plaque-%03d.tif"), delai).await?;
    ghostscript(gs, pdf, "tiffsep", DPI_APERCU, options, &apercu.join("plaque-%03d.tif"), delai).await?;
    // L'aperçu est en RVB : le profil du support (CMJN) y sert d'épreuve écran.
    let epreuve: Vec<String> = options.iter().map(|o| o.replace("-sOutputICCProfile=", "-sProofProfile=")).collect();
    ghostscript(gs, pdf, "png16m", DPI_APERCU, &epreuve, &apercu.join("composite-%03d.png"), delai).await?;

    let pages = plaques_par_page(&prod)?;
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
    Ok(Rendu { pages })
}

async fn ghostscript(gs: &str, pdf: &Path, device: &str, dpi: u32, options: &[String], sortie: &Path, delai: Duration) -> Result<(), String> {
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
        let r = rendre("gs", &pdf, &dossier, dpi, &[], Duration::from_secs(60)).await.unwrap();

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

    /// Un profil de sortie convertit les couleurs : le cyan à 20 % ne reste pas tel quel.
    #[tokio::test]
    async fn le_profil_de_sortie_convertit() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-icc-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let pdf = dossier.join("source.pdf");
        std::fs::write(&pdf, pdf_deux_pages()).unwrap();
        let profil = crate::couleur::tests::profils_ghostscript().join("ps_cmyk.icc");
        let options = [format!("-sOutputICCProfile={}", profil.display())];
        rendre("gs", &pdf, &dossier, 72, &options, Duration::from_secs(60)).await.unwrap();
        let (c, m) = (pixel(&dossier, 1, "Cyan", 50.0, 72), pixel(&dossier, 1, "Magenta", 50.0, 72));
        assert_ne!((c, m), (204, 102), "valeurs inchangées : le profil n'a pas été appliqué");
        assert_eq!(pixel(&dossier, 1, "White", 140.0, 72), 0, "un ton direct n'est pas converti");
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
