//! Heure de Paris sans base de fuseaux : la France applique la règle européenne (heure d'été
//! du dernier dimanche de mars au dernier dimanche d'octobre, à 01:00 UTC). Sert aux e-mails
//! (dates lisibles) et au résumé du lundi 8 h.

use crate::portail::civil;

const JOUR: i64 = 86_400;

/// Jours depuis 1970-01-01 pour une date civile (algorithme de H. Hinnant).
pub fn jours_depuis_civil(a: i64, m: u32, j: u32) -> i64 {
    let y = if m <= 2 { a - 1 } else { a };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(j) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Dernier dimanche du mois (en jours depuis 1970-01-01).
fn dernier_dimanche(annee: i64, mois: u32) -> i64 {
    let (a, m) = if mois == 12 { (annee + 1, 1) } else { (annee, mois + 1) };
    let dernier = jours_depuis_civil(a, m, 1) - 1;
    // 1970-01-01 est un jeudi : dimanche = 0.
    dernier - (dernier + 4).rem_euclid(7)
}

/// Décalage de Paris par rapport à UTC, en secondes (3600 l'hiver, 7200 l'été).
pub fn decalage_paris(t: u64) -> i64 {
    let t = t as i64;
    let (annee, _, _) = civil(t.div_euclid(JOUR));
    let debut = dernier_dimanche(annee, 3) * JOUR + 3600;
    let fin = dernier_dimanche(annee, 10) * JOUR + 3600;
    if (debut..fin).contains(&t) { 7200 } else { 3600 }
}

/// Heure locale de Paris (secondes depuis 1970 « à l'heure du mur »).
fn locale(t: u64) -> i64 {
    t as i64 + decalage_paris(t)
}

/// Lundi 8 h (Paris) du lundi civil `jour` (jours depuis 1970-01-01), en UTC.
fn lundi_8h(jour: i64) -> u64 {
    let mur = jour * JOUR + 8 * 3600;
    // Le changement d'heure a lieu le dimanche à 01:00 UTC : le décalage est le même toute la journée du lundi.
    (mur - decalage_paris((mur - 3 * 3600) as u64)) as u64
}

/// Dernier « lundi 8 h » de Paris qui n'est pas dans le futur de `t` (UTC).
pub fn lundi_8h_precedent(t: u64) -> u64 {
    let jour = locale(t).div_euclid(JOUR);
    // 1970-01-01 est un jeudi : lundi = 0.
    let lundi = jour - (jour + 3).rem_euclid(7);
    let b = lundi_8h(lundi);
    if b > t { lundi_8h(lundi - 7) } else { b }
}

const MOIS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

/// « 3 novembre 2026 » (jour de Paris).
pub fn date_fr(t: u64) -> String {
    let (a, m, j) = civil(locale(t).div_euclid(JOUR));
    format!("{j}{} {} {a}", if j == 1 { "er" } else { "" }, MOIS[m as usize - 1])
}

/// « lundi 3 novembre 2026 à 8 h 05 » (heure de Paris).
pub fn date_heure_fr(t: u64) -> String {
    let l = locale(t);
    let jour = l.div_euclid(JOUR);
    let s = l.rem_euclid(JOUR);
    format!("{} {} à {} h {:02}", JOURS[(jour + 3).rem_euclid(7) as usize], date_fr(t), s / 3600, s % 3600 / 60)
}

/// « 25 min », « 2 h 05 », « 1 j 3 h ».
pub fn duree_fr(secondes: u64) -> String {
    match secondes {
        0..=59 => "moins d'une minute".into(),
        60..=3599 => format!("{} min", secondes / 60),
        3600..=86_399 => {
            let (h, m) = (secondes / 3600, secondes % 3600 / 60);
            if m == 0 { format!("{h} h") } else { format!("{h} h {m:02}") }
        }
        _ => {
            let (j, h) = (secondes / 86_400, secondes % 86_400 / 3600);
            if h == 0 { format!("{j} j") } else { format!("{j} j {h} h") }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(a: i64, m: u32, j: u32, h: u64, min: u64) -> u64 {
        (jours_depuis_civil(a, m, j) as u64) * 86_400 + h * 3600 + min * 60
    }

    #[test]
    fn calendrier() {
        assert_eq!(jours_depuis_civil(1970, 1, 1), 0);
        assert_eq!(utc(2026, 10, 31, 0, 0), 1_793_404_800);
        assert_eq!(civil(jours_depuis_civil(2024, 2, 29)), (2024, 2, 29));
    }

    #[test]
    fn heure_d_ete_europeenne() {
        // 2026 : passage à l'heure d'été le 29 mars, retour le 25 octobre (01:00 UTC).
        assert_eq!(decalage_paris(utc(2026, 3, 29, 0, 59)), 3600);
        assert_eq!(decalage_paris(utc(2026, 3, 29, 1, 0)), 7200);
        assert_eq!(decalage_paris(utc(2026, 7, 14, 12, 0)), 7200);
        assert_eq!(decalage_paris(utc(2026, 10, 25, 0, 59)), 7200);
        assert_eq!(decalage_paris(utc(2026, 10, 25, 1, 0)), 3600);
        assert_eq!(decalage_paris(utc(2026, 12, 25, 12, 0)), 3600);
        assert_eq!(decalage_paris(utc(2027, 3, 28, 1, 0)), 7200);
    }

    #[test]
    fn lundi_8h_de_paris() {
        // Lundi 2 novembre 2026, 8 h à Paris (UTC+1) = 07:00 UTC.
        let nov = utc(2026, 11, 2, 7, 0);
        assert_eq!(lundi_8h_precedent(nov), nov);
        assert_eq!(lundi_8h_precedent(nov + 3 * 86_400), nov);
        // Juste avant : le lundi précédent (26 octobre, UTC+1 depuis le 25).
        assert_eq!(lundi_8h_precedent(nov - 1), utc(2026, 10, 26, 7, 0));
        // Dimanche 1er novembre (jour du test) : lundi 26 octobre.
        assert_eq!(lundi_8h_precedent(utc(2026, 11, 1, 12, 0)), utc(2026, 10, 26, 7, 0));
        // Été (UTC+2) : lundi 6 juillet 2026, 8 h = 06:00 UTC.
        assert_eq!(lundi_8h_precedent(utc(2026, 7, 8, 9, 0)), utc(2026, 7, 6, 6, 0));
        // Lundi 7 h 30 à Paris l'été (05:30 UTC) : pas encore 8 h, c'est le lundi d'avant.
        assert_eq!(lundi_8h_precedent(utc(2026, 7, 6, 5, 30)), utc(2026, 6, 29, 6, 0));
        // Lundi 30 mars 2026, au lendemain du passage à l'heure d'été : 8 h = 06:00 UTC.
        assert_eq!(lundi_8h_precedent(utc(2026, 3, 30, 6, 0)), utc(2026, 3, 30, 6, 0));
        // Le lundi d'avant le changement d'heure (23 mars) : UTC+1, 07:00 UTC.
        assert_eq!(lundi_8h_precedent(utc(2026, 3, 29, 12, 0)), utc(2026, 3, 23, 7, 0));
    }

    #[test]
    fn textes() {
        assert_eq!(date_fr(utc(2026, 11, 1, 12, 0)), "1er novembre 2026");
        assert_eq!(date_heure_fr(utc(2026, 11, 2, 7, 5)), "lundi 2 novembre 2026 à 8 h 05");
        assert_eq!(date_heure_fr(utc(2026, 7, 14, 21, 30)), "mardi 14 juillet 2026 à 23 h 30");
        assert_eq!(duree_fr(30), "moins d'une minute");
        assert_eq!(duree_fr(25 * 60), "25 min");
        assert_eq!(duree_fr(7500), "2 h 05");
        assert_eq!(duree_fr(7200), "2 h");
        assert_eq!(duree_fr(90_000), "1 j 1 h");
    }
}
