//! Rendu d'un PDF par Ghostscript : une plaque 8 bits par encre (CMJN + tons directs).
//!
//! Convention des plaques `tiffsep` (mesurée le 23/10/2026, Ghostscript 10.08) :
//! 0 = pleine encre, 255 = pas d'encre. Un ton direct (`White`, `Silver`…) sort sur
//! sa propre plaque, nommée d'après la séparation du PDF.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

pub struct Rendu {
    /// Noms des plaques dans l'ordre alphabétique des fichiers produits.
    pub plaques: Vec<String>,
}

/// Rend `pdf` dans `dossier` : plaques pleine définition (`prod/`), plaques et
/// composite basse définition pour l'aperçu (`apercu/`).
pub async fn rendre(gs: &str, pdf: &Path, dossier: &Path, dpi: u32, delai: Duration) -> Result<Rendu, String> {
    let prod = dossier.join("prod");
    let apercu = dossier.join("apercu");
    for d in [&prod, &apercu] {
        tokio::fs::create_dir_all(d).await.map_err(|e| e.to_string())?;
    }

    ghostscript(gs, pdf, "tiffsep", dpi, &prod.join("plaque.tif"), delai).await?;
    ghostscript(gs, pdf, "tiffsep", 50, &apercu.join("plaque.tif"), delai).await?;
    ghostscript(gs, pdf, "png16m", 50, &apercu.join("composite.png"), delai).await?;

    let plaques = noms_des_plaques(&prod)?;
    for nom in &plaques {
        let tif = apercu.join(format!("plaque({nom}).tif"));
        let png = apercu.join(format!("plaque({nom}).png"));
        image::open(&tif)
            .and_then(|img| img.save(&png))
            .map_err(|e| format!("aperçu de la plaque {nom} : {e}"))?;
    }
    Ok(Rendu { plaques })
}

async fn ghostscript(gs: &str, pdf: &Path, device: &str, dpi: u32, sortie: &Path, delai: Duration) -> Result<(), String> {
    let mut cmd = Command::new(gs);
    cmd.args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE"])
        .arg(format!("-sDEVICE={device}"))
        .arg(format!("-r{dpi}"))
        // ponytail: une seule page en P0 ; le multipage viendra avec la file d'impression.
        .args(["-dFirstPage=1", "-dLastPage=1"])
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

/// `plaque(Cyan).tif` → `Cyan`. Le composite `plaque.tif` n'est pas une plaque.
fn noms_des_plaques(dossier: &Path) -> Result<Vec<String>, String> {
    let mut noms: Vec<String> = std::fs::read_dir(dossier)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let f = e.file_name().into_string().ok()?;
            Some(f.strip_prefix("plaque(")?.strip_suffix(").tif")?.to_string())
        })
        .collect();
    noms.sort();
    if noms.is_empty() {
        return Err("Ghostscript n'a produit aucune plaque".into());
    }
    Ok(noms)
}

pub fn chemin_apercu_plaque(dossier: &Path, nom: &str) -> PathBuf {
    dossier.join("apercu").join(format!("plaque({nom}).png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PDF d'une page : un carré CMJN, un carré `White` plein, un carré `Silver` à 50 %.
    fn pdf_tons_directs() -> Vec<u8> {
        let contenu = "0.2 0.6 0 0 k 10 10 80 80 re f\n/CS1 cs 1 scn 100 10 80 80 re f\n/CS2 cs 0.5 scn 190 10 80 80 re f\n";
        let sep = |nom: &str, k: f32| {
            format!("[/Separation /{nom} /DeviceCMYK << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 0 0 {k}] /N 1 >>]")
        };
        let objets = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 280 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R /CS2 6 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{contenu}endstream", contenu.len()),
            sep("White", 0.1),
            sep("Silver", 0.4),
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
    fn pixel(dossier: &Path, plaque: &str, x_pt: f32, dpi: u32) -> u8 {
        let img = image::open(dossier.join(format!("prod/plaque({plaque}).tif"))).unwrap().to_luma8();
        let x = (x_pt * dpi as f32 / 72.0) as u32;
        img.get_pixel(x, img.height() / 2)[0]
    }

    #[tokio::test]
    async fn les_tons_directs_sortent_sur_leur_propre_plaque() {
        let dossier = std::env::temp_dir().join(format!("eko-rip-test-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let pdf = dossier.join("source.pdf");
        std::fs::write(&pdf, pdf_tons_directs()).unwrap();

        let dpi = 72;
        let r = rendre("gs", &pdf, &dossier, dpi, Duration::from_secs(60)).await.unwrap();

        assert_eq!(r.plaques, ["Black", "Cyan", "Magenta", "Silver", "White", "Yellow"]);
        // 0 = pleine encre, 255 = pas d'encre.
        assert_eq!(pixel(&dossier, "White", 140.0, dpi), 0, "blanc plein sous le carré blanc");
        assert_eq!(pixel(&dossier, "White", 50.0, dpi), 255, "pas de blanc ailleurs");
        let argent = pixel(&dossier, "Silver", 230.0, dpi);
        assert!((126..=129).contains(&argent), "argent à 50 % : {argent}");
        assert_eq!(pixel(&dossier, "Cyan", 50.0, dpi), 204, "cyan à 20 %");
        assert_eq!(pixel(&dossier, "Black", 140.0, dpi), 255, "le blanc ne doit pas fuir dans le noir");
        assert!(chemin_apercu_plaque(&dossier, "White").exists());
        assert!(dossier.join("apercu/composite.png").exists());

        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
