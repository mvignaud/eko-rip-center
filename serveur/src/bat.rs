//! Bon à tirer : un PDF A4 paysage par travail, une page par page du travail, à envoyer
//! au client. L'image (épreuve écran du support), le tracé de découpe superposé, les
//! caractéristiques du travail et un cadre « Bon pour accord ».

use crate::decoupe::Trace;
use crate::imbrication::PT_PAR_MM;
use lopdf::{Document, Object, Stream, dictionary};

/// Une page du travail telle que le BAT la montre.
pub struct PageBat {
    /// Image RVB 8 bits de la page entière.
    pub image: image::RgbImage,
    /// Taille de la page, en points.
    pub largeur: f64,
    pub hauteur: f64,
    /// Tracés de découpe, en points de la page (origine en bas à gauche).
    pub traces: Vec<Trace>,
}

/// Texte du BAT, déjà traduit ; `lignes` décrit le travail (une ligne par caractéristique).
pub struct Textes {
    pub titre: String,
    pub sous_titre: String,
    pub lignes: Vec<String>,
    pub page: String,
    pub dimensions: String,
    pub accord: String,
    pub signature: String,
    pub legende_decoupe: String,
}

/// Chaîne PDF en WinAnsi (police de base Helvetica) : latin-1 direct, quelques signes
/// typographiques à leur place WinAnsi, le reste en « ? ».
pub fn winansi(s: &str) -> String {
    let mut o = String::from("(");
    for c in s.chars() {
        let code: u32 = match c {
            '€' => 0x80,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '–' => 0x96,
            '—' => 0x97,
            '…' => 0x85,
            'œ' => 0x9C,
            'Œ' => 0x8C,
            '\u{202F}' | '\u{A0}' => 0x20,
            c if (c as u32) < 0x80 || (0xA0..=0xFF).contains(&(c as u32)) => c as u32,
            _ => '?' as u32,
        };
        match code {
            0x28 | 0x29 | 0x5C => o.push_str(&format!("\\{}", char::from(code as u8))),
            0x20..=0x7E => o.push(char::from(code as u8)),
            _ => o.push_str(&format!("\\{code:03o}")),
        }
    }
    o.push(')');
    o
}

fn texte(x: f64, y: f64, taille: f64, gras: bool, s: &str) -> String {
    format!("BT /{} {taille} Tf {x:.2} {y:.2} Td {} Tj ET\n", if gras { "HB" } else { "H" }, winansi(s))
}

/// Compose le BAT.
pub fn composer(t: &Textes, pages: &[PageBat]) -> Result<Vec<u8>, String> {
    let (l, h) = (297.0 * PT_PAR_MM, 210.0 * PT_PAR_MM);
    let m = 12.0 * PT_PAR_MM;
    let mut doc = Document::with_version("1.6");
    let pages_id = doc.new_object_id();
    let helv = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding" });
    let gras = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold", "Encoding" => "WinAnsiEncoding" });
    let mut enfants = vec![];
    for (i, p) in pages.iter().enumerate() {
        // Cadre de l'image à gauche (2/3 de la largeur), colonne d'informations à droite.
        let (cx, cy, cl, ch) = (m, m + 28.0, (l - 2.0 * m) * 0.66, h - 2.0 * m - 60.0);
        let e = (cl / p.largeur).min(ch / p.hauteur);
        let (il, ih) = (p.largeur * e, p.hauteur * e);
        let (ix, iy) = (cx + (cl - il) / 2.0, cy + (ch - ih) / 2.0);
        let mut c = String::new();
        c += &texte(m, h - m - 14.0, 16.0, true, &t.titre);
        c += &texte(m, h - m - 30.0, 9.0, false, &t.sous_titre);
        c += &texte(l - m - 60.0, h - m - 14.0, 9.0, false, &format!("{} {}/{}", t.page, i + 1, pages.len()));
        // Fond quadrillé léger sous l'image : on voit où finit la page.
        c += &format!("q 0.6 G 0.3 w {cx:.2} {cy:.2} {cl:.2} {ch:.2} re S Q\n");
        c += &format!("q {il:.3} 0 0 {ih:.3} {ix:.3} {iy:.3} cm /Im Do Q\n");
        c += &format!("q 0 G 0.5 w {ix:.2} {iy:.2} {il:.2} {ih:.2} re S Q\n");
        if !p.traces.is_empty() {
            c += "q 0.85 0 0.85 RG 0.9 w 1 J 1 j ";
            for tr in &p.traces {
                for (k, &(x, y)) in tr.points.iter().enumerate() {
                    c += &format!("{:.2} {:.2} {} ", ix + x * e, iy + y * e, if k == 0 { "m" } else { "l" });
                }
                c += if tr.ferme { "h S " } else { "S " };
            }
            c += "Q\n";
            c += &format!("q 0.85 0 0.85 RG 0.9 w {:.2} {:.2} m {:.2} {:.2} l S Q\n", cx, cy - 12.0, cx + 20.0, cy - 12.0);
            c += &texte(cx + 26.0, cy - 15.0, 8.0, false, &t.legende_decoupe);
        }
        c += &texte(cx, cy - 2.0 - 26.0, 9.0, true, &t.dimensions.replace("{l}", &format!("{:.0}", p.largeur / PT_PAR_MM)).replace("{h}", &format!("{:.0}", p.hauteur / PT_PAR_MM)));
        // Colonne d'informations.
        let colx = cx + cl + 14.0;
        let mut y = cy + ch - 4.0;
        for ligne in &t.lignes {
            c += &texte(colx, y, 8.5, false, ligne);
            y -= 13.0;
        }
        // Bon pour accord.
        let (bx, by, bl, bh) = (colx, m, l - m - colx, 70.0);
        c += &format!("q 0 G 0.6 w {bx:.2} {by:.2} {bl:.2} {bh:.2} re S Q\n");
        c += &texte(bx + 6.0, by + bh - 14.0, 10.0, true, &t.accord);
        c += &texte(bx + 6.0, by + 10.0, 8.0, false, &t.signature);

        let mut im = Stream::new(
            dictionary! { "Type" => "XObject", "Subtype" => "Image", "Width" => p.image.width() as i64, "Height" => p.image.height() as i64, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8 },
            p.image.as_raw().clone(),
        );
        im.compress().map_err(|e| e.to_string())?;
        let im = doc.add_object(im);
        let mut flux = Stream::new(dictionary! {}, c.into_bytes());
        flux.compress().map_err(|e| e.to_string())?;
        let flux = doc.add_object(flux);
        enfants.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(l as f32), Object::Real(h as f32)],
            "Contents" => flux,
            "Resources" => dictionary! { "Font" => dictionary! { "H" => helv, "HB" => gras }, "XObject" => dictionary! { "Im" => im } },
        })));
    }
    let compte = enfants.len() as i64;
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => enfants, "Count" => compte }));
    let catalogue = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalogue);
    let mut sortie = vec![];
    doc.save_to(&mut sortie).map_err(|e| e.to_string())?;
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaines_winansi() {
        assert_eq!(winansi("Bâche (œillets) 12 €"), "(B\\342che \\(\\234illets\\) 12 \\200)");
        assert_eq!(winansi("a\\b — ok"), "(a\\\\b \\227 ok)");
        assert_eq!(winansi("日本"), "(??)");
    }

    #[test]
    fn bat_lisible_et_rendu() {
        let image = image::RgbImage::from_pixel(60, 30, image::Rgb([0, 160, 230]));
        let trace = Trace { ton: "CutContour".into(), points: vec![(5.0, 5.0), (95.0, 5.0), (95.0, 45.0), (5.0, 45.0)], ferme: true };
        let textes = Textes {
            titre: "Bon à tirer — Bâche œillets".into(),
            sous_titre: "Travail 1234 · 24/10/2026".into(),
            lignes: vec!["Mode : imprimer".into(), "Résolution : 720 dpi".into()],
            page: "Page".into(),
            dimensions: "{l} × {h} mm".into(),
            accord: "Bon pour accord".into(),
            signature: "Date et signature :".into(),
            legende_decoupe: "Découpe".into(),
        };
        let p = PageBat { image, largeur: 100.0, hauteur: 50.0, traces: vec![trace] };
        let pdf = composer(&textes, &[p]).unwrap();
        let doc = Document::load_mem(&pdf).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
        let d = std::env::temp_dir().join(format!("eko-rip-bat-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("b.pdf"), &pdf).unwrap();
        let st = std::process::Command::new("gs")
            .args(["-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=png16m", "-r30"])
            .arg(format!("-sOutputFile={}", d.join("b.png").display()))
            .arg(d.join("b.pdf"))
            .status()
            .unwrap();
        assert!(st.success());
        let png = image::open(d.join("b.png")).unwrap().to_rgb8();
        // A4 paysage à 30 dpi : 351 × 248 px ; l'image bleue occupe le centre du cadre.
        assert_eq!(png.dimensions(), (351, 248));
        assert!(png.pixels().any(|p| p.0 == [0, 160, 230]), "image du travail présente");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
