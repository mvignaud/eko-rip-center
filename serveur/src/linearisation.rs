//! Préparation de la linéarisation (roadmap 8.11, avant 8.1 à 8.3 qui demandent la machine
//! et l'i1Pro) : paliers de mire resserrés dans les hautes lumières, et limite d'encre
//! totale proposée à partir d'une série mesurée. La limite n'est qu'une proposition :
//! l'opérateur la relit et la saisit lui-même dans le mode (validée à la main).
//!
//! Méthode et valeurs à nous. Les paliers suivent `t^GAMMA` : une rampe régulière laisse
//! une plage sur cinq sous 20 %, là où l'œil voit le mieux les écarts et où l'encre claire
//! travaille seule ; avec 1,7, il y en a presque deux sur cinq.

use serde::{Deserialize, Serialize};

const GAMMA: f64 = 1.7;
/// Pas d'arrondi des paliers (%), assez fin pour un tramage 2 bits sur 8 bits.
const PAS: f64 = 0.5;

/// Paliers d'une rampe de `n` plages (5..=101), en %, de 0 à 100, strictement croissants.
pub fn paliers(n: usize) -> Result<Vec<f64>, &'static str> {
    if !(5..=101).contains(&n) {
        return Err("mire_invalide");
    }
    let mut v: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        let brut = (i as f64 / (n - 1) as f64).powf(GAMMA) * 100.0;
        let mut p = (brut / PAS).round() * PAS;
        // Les premiers paliers d'une longue rampe tombent sur le même arrondi : un pas plus haut.
        if let Some(&avant) = v.last() {
            p = p.max(avant + PAS);
        }
        v.push(p);
    }
    Ok(v)
}

/// Mire de linéarisation au format CGATS (texte), une plage par encre × palier, à lire au
/// spectro (8.1, 8.2). Une colonne par encre, en %.
pub fn mire_cgats(encres: &[&str], n: usize) -> Result<String, &'static str> {
    if encres.is_empty() || encres.iter().any(|e| e.is_empty() || !e.chars().all(|c| c.is_ascii_alphanumeric())) {
        return Err("mire_invalide");
    }
    let paliers = paliers(n)?;
    let mut s = String::from("CGATS.17\nORIGINATOR \"E-KO RIP Center\"\nDESCRIPTOR \"Mire de linearisation par encre\"\n");
    s += &format!("NUMBER_OF_FIELDS {}\nBEGIN_DATA_FORMAT\nSAMPLE_ID {}\nEND_DATA_FORMAT\n", encres.len() + 1, encres.iter().map(|e| format!("{e}_PCT")).collect::<Vec<_>>().join(" "));
    s += &format!("NUMBER_OF_SETS {}\nBEGIN_DATA\n", encres.len() * paliers.len());
    let mut id = 1;
    for (j, _) in encres.iter().enumerate() {
        for p in &paliers {
            let valeurs: Vec<String> = (0..encres.len()).map(|k| format!("{:.1}", if k == j { *p } else { 0.0 })).collect();
            s += &format!("{id} {}\n", valeurs.join(" "));
            id += 1;
        }
    }
    s += "END_DATA\n";
    Ok(s)
}

/// Gain de foncé (en L*) en dessous duquel 10 % d'encre de plus ne valent plus la peine :
/// 1 L* par 10 %, à peine visible, alors que l'encre en plus allonge le séchage.
pub const SEUIL_DEFAUT: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposition {
    /// Limite d'encre totale proposée (%), arrondie aux 5 % inférieurs.
    pub limite: f64,
    /// Gain de foncé par 10 % d'encre entre deux mesures successives : (total de fin, gain).
    pub gains: Vec<(f64, f64)>,
}

/// Limite d'encre totale proposée à partir d'aplats mesurés : (encre totale %, L*). Elle
/// s'arrête au dernier palier qui fonce encore d'au moins `seuil` L* par 10 % ; un palier
/// qui ne fonce plus, ou qui s'éclaircit (bronzage, bavure), ne compte pas, même suivi
/// d'une mesure bruitée plus foncée sous le seuil. Même entrée, même réponse, quel que
/// soit l'ordre des mesures.
pub fn limite_proposee(mesures: &[(f64, f64)], seuil: f64) -> Result<Proposition, &'static str> {
    let mut m = mesures.to_vec();
    if m.len() < 4 || !seuil.is_finite() || seuil <= 0.0 || m.iter().any(|&(t, l)| !(0.0..=900.0).contains(&t) || !(0.0..=100.0).contains(&l)) {
        return Err("mesures_invalides");
    }
    m.sort_by(|a, b| a.0.total_cmp(&b.0));
    if m.windows(2).any(|w| w[0].0 == w[1].0) {
        return Err("mesures_invalides");
    }
    let gains: Vec<(f64, f64)> = m.windows(2).map(|w| (w[1].0, (w[0].1 - w[1].1) / (w[1].0 - w[0].0) * 10.0)).collect();
    let fin = gains.iter().rev().find(|g| g.1 >= seuil).map_or(m[0].0, |g| g.0);
    Ok(Proposition { limite: (fin / 5.0).floor() * 5.0, gains })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mire_plus_dense_dans_les_hautes_lumieres() {
        for n in [11, 21, 33, 51, 101] {
            let p = paliers(n).unwrap();
            assert_eq!(p.len(), n);
            assert_eq!((p[0], p[n - 1]), (0.0, 100.0));
            assert!(p.windows(2).all(|w| w[1] > w[0]), "{n} : {p:?}");
            let sous_20 = p.iter().filter(|&&x| x < 20.0).count();
            let reguliere = (0..n).filter(|&i| (i as f64 / (n - 1) as f64) < 0.2).count();
            assert!(sous_20 > reguliere, "{n} : {sous_20} plages < 20 % contre {reguliere}");
        }
        assert_eq!(paliers(21).unwrap().iter().filter(|&&x| x < 20.0).count(), 8, "8 plages < 20 % sur 21, contre 4");
        assert!(paliers(4).is_err() && paliers(102).is_err());
        let c = mire_cgats(&["C", "Lc"], 11).unwrap();
        assert!(c.contains("SAMPLE_ID C_PCT Lc_PCT") && c.contains("NUMBER_OF_SETS 22"));
        assert_eq!(c.lines().filter(|l| l.split(' ').count() == 3 && l.chars().next().unwrap().is_ascii_digit()).count(), 22);
        assert!(mire_cgats(&[], 11).is_err() && mire_cgats(&["C D"], 11).is_err());
    }

    /// Aplats simulés : L* descend vite puis plafonne vers 18 (noir d'un vinyle).
    fn simulees() -> Vec<(f64, f64)> {
        (0..=15).map(|i| {
            let t = 100.0 + 20.0 * i as f64;
            (t, 18.0 + 40.0 * (-(t - 100.0) / 70.0).exp())
        }).collect()
    }

    #[test]
    fn limite_proposee_reproductible() {
        let m = simulees();
        let p = limite_proposee(&m, SEUIL_DEFAUT).unwrap();
        // Gain par 10 % = 20 × e^(−(t−120)/70) × (1 − e^(−20/70)) : 1,19 L* de 200 à 220 %,
        // 0,89 de 220 à 240 %.
        assert_eq!(p.limite, 220.0, "{:?}", p.gains);
        assert_eq!(p.gains.len(), 15);
        // Même réponse dans le désordre, et un seuil plus exigeant propose moins d'encre.
        let mut melange = m.clone();
        melange.reverse();
        melange.swap(2, 9);
        assert_eq!(limite_proposee(&melange, SEUIL_DEFAUT).unwrap(), p);
        assert!(limite_proposee(&m, 2.0).unwrap().limite < p.limite);
        // Bronzage : au-delà de 160 %, L* remonte ; une mesure bruitée plus loin ne compte pas.
        let mut bronze: Vec<(f64, f64)> = m.iter().map(|&(t, l)| (t, if t > 160.0 { l + (t - 160.0) / 5.0 } else { l })).collect();
        bronze.push((420.0, bronze.last().unwrap().1 - 0.5));
        assert_eq!(limite_proposee(&bronze, SEUIL_DEFAUT).unwrap().limite, 160.0);
        // Arrondi aux 5 % inférieurs.
        let m2 = [(100.0, 60.0), (150.0, 40.0), (233.0, 25.0), (260.0, 24.9)];
        assert_eq!(limite_proposee(&m2, SEUIL_DEFAUT).unwrap().limite, 230.0);
        // Entrées refusées.
        assert!(limite_proposee(&m[..3], 1.0).is_err());
        assert!(limite_proposee(&[(100.0, 50.0), (100.0, 40.0), (200.0, 30.0), (300.0, 20.0)], 1.0).is_err());
        assert!(limite_proposee(&[(100.0, 50.0), (150.0, f64::NAN), (200.0, 30.0), (300.0, 20.0)], 1.0).is_err());
        assert!(limite_proposee(&m, 0.0).is_err());
    }
}
