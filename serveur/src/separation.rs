//! Séparation 9 encres de l'Epson SC-S80600 (roadmap 2.2) : canaux C, M, Y, K 8 bits du
//! rendu (+ Or et Rd s'ils existent) → K M C Y Lk Lm Lc Or Rd, 8 bits, avant tramage.
//!
//! Toutes les valeurs sont à nous, paramètres documentés et ajustables ; rien n'est repris
//! d'un autre RIP. Méthode : chaque canal logique C, M, K vise une **densité** `d` linéaire
//! en son entrée, partagée entre une encre claire (densité relative `clair`) et la foncée :
//!
//! - zone claire seule, `d ≤ clair × max` : clair = `d / clair`, foncé = 0 ;
//! - zone de transition, de largeur `transition` (en densité) : le clair descend en ligne
//!   droite de `max` à `end` pendant que le foncé prend le relais ;
//! - au-delà : clair = `end`, foncé = le reste.
//!
//! Le foncé est toujours calculé comme `d − clair × clair_arrondi` : la densité pondérée
//! `clair × Lx + X` égale `d` à ½ niveau près, donc elle est **monotone croissante** sur
//! 0→255 et vaut 255 à 100 % (la densité du foncé seul à 100 %). `start` est l'entrée sous
//! laquelle le canal ne pose rien (zone morte), la densité est ensuite remise à l'échelle.
//!
//! Y n'a pas d'encre claire : il passe tel quel. Or et Rd : sans profil de gamut étendu on
//! n'en invente pas ; absents en entrée, ils restent à 0 ; présents (ton direct du rendu,
//! ou séparation d'un profil à venir), ils passent tels quels, plafonnés par `limite_or`
//! et `limite_rd`. La limite d'encre totale (somme des 9 canaux, en %) s'applique en
//! dernier, par réduction proportionnelle de tous les canaux du point.

use crate::escp2::Encre;
use serde::{Deserialize, Serialize};

/// Partage d'un canal logique entre son encre claire et sa foncée. Fractions 0..=1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Couple {
    /// Densité d'un aplat 100 % de l'encre claire, relative à celle de la foncée.
    pub clair: f32,
    /// Entrée sous laquelle aucune encre ne se pose.
    pub start: f32,
    /// Encrage maximal de l'encre claire.
    pub max: f32,
    /// Largeur, en densité, de la zone où la foncée prend le relais de la claire.
    pub transition: f32,
    /// Encrage de la claire qui reste sous la foncée à 100 % (≤ `max`).
    pub end: f32,
}

impl Default for Couple {
    /// `clair` 0,3 : une encre claire de jet d'encre grand format est diluée environ trois
    /// fois ; valeur de départ, à mesurer au densitomètre (2.8). `max` 0,8 : la claire
    /// seule couvre les tons jusqu'à 24 % sans saturer le vinyle d'une encre peu dense.
    /// `transition` 0,3 : passage progressif de 24 % à 54 %, assez large pour que le grain
    /// de la foncée apparaisse sans saut visible. `end` 0 : pas de claire sous la foncée
    /// pleine, l'encre totale reste celle du foncé.
    fn default() -> Self {
        Couple { clair: 0.3, start: 0.0, max: 0.8, transition: 0.3, end: 0.0 }
    }
}

impl Couple {
    /// Table entrée → (clair, foncé), 8 bits.
    pub fn table(&self) -> [(u8, u8); 256] {
        let f = self.clair.clamp(0.01, 1.0);
        let max = self.max.clamp(0.0, 1.0);
        let end = self.end.clamp(0.0, max);
        let start = self.start.clamp(0.0, 0.99);
        let p = f * max;
        let t = self.transition.clamp(0.0, 1.0 - p);
        let mut table = [(0, 0); 256];
        for (n, case) in table.iter_mut().enumerate() {
            let d = ((n as f32 / 255.0 - start) / (1.0 - start)).max(0.0);
            let l = if d <= p {
                d / f
            } else if d >= p + t {
                end
            } else {
                max - (max - end) * (d - p) / t
            };
            let l8 = (l * 255.0).round().clamp(0.0, 255.0);
            let k8 = (d * 255.0 - f * l8).round().clamp(0.0, 255.0);
            *case = (l8 as u8, k8 as u8);
        }
        table
    }
}

/// Préréglages de bascule des encres claires (roadmap 8.11) : où et comment la foncée
/// prend le relais de la claire sur C, M et K. Noms et valeurs à nous ; la densité de la
/// claire (`clair`, mesurée) et la zone morte (`start`) du réglage en place sont gardées.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bascule {
    /// Claire pleine, relais long (de 30 % à 70 % avec `clair` 0,3) et un peu de claire
    /// sous la foncée : le moins de grain, pour les dégradés et la peau vus de près.
    Douce,
    /// Les valeurs par défaut de `Couple`.
    Equilibree,
    /// Claire limitée à 60 %, relais court (de 18 % à 33 %) : moins d'encre posée et un
    /// séchage plus rapide, pour l'affichage vu de loin.
    Franche,
}

impl Bascule {
    pub const TOUTES: [Bascule; 3] = [Bascule::Douce, Bascule::Equilibree, Bascule::Franche];

    pub fn appliquer(self, c: Couple) -> Couple {
        let (max, transition, end) = match self {
            Bascule::Douce => (1.0, 0.4, 0.15),
            Bascule::Equilibree => {
                let d = Couple::default();
                (d.max, d.transition, d.end)
            }
            Bascule::Franche => (0.6, 0.15, 0.0),
        };
        Couple { max, transition, end, ..c }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Separation {
    pub cyan: Couple,
    pub magenta: Couple,
    pub noir: Couple,
    /// Plafond de l'orange et du rouge, fraction 0..=1.
    pub limite_or: f32,
    pub limite_rd: f32,
    /// Encre totale maximale d'un point, en % cumulé des 9 canaux.
    pub limite_totale: f32,
    /// Plafond de chaque canal, fraction 0..=1, appliqué avant la limite totale. 0,85 : au
    /// 1er tirage (25/10), 100 % donnait ~98 % de gros points et des jonctions de passes
    /// visibles ; le flux de référence observé ne pose pas de gros points en C, M, J et
    /// couvre ~0,89 en noir plein. Départ prudent, à étalonner sur papier.
    #[serde(default = "limite_canal_defaut")]
    pub limite_canal: f32,
}

fn limite_canal_defaut() -> f32 {
    0.85
}

impl Default for Separation {
    /// Or et Rd non plafonnés (la limite totale les tient). 300 % : un noir riche du rendu
    /// atteint 400 %, trop pour sécher sur vinyle en éco-solvant ; les fiches publiques de
    /// supports éco-solvant parlent de 250 à 320 %. Départ prudent, à étalonner (2.8).
    fn default() -> Self {
        let c = Couple::default();
        Separation { cyan: c, magenta: c, noir: c, limite_or: 1.0, limite_rd: 1.0, limite_totale: 300.0, limite_canal: limite_canal_defaut() }
    }
}

impl Separation {
    /// Séparation d'un mode d'impression : la bascule et la limite totale qu'il porte
    /// (8.11), le reste par défaut.
    pub fn du_mode(bascule: Option<Bascule>, limite_totale: Option<f32>) -> Self {
        let mut p = Separation::default();
        if let Some(b) = bascule {
            p.cyan = b.appliquer(p.cyan);
            p.magenta = b.appliquer(p.magenta);
            p.noir = b.appliquer(p.noir);
        }
        if let Some(l) = limite_totale {
            p.limite_totale = l;
        }
        p
    }
}

/// Canaux du rendu (Ghostscript tiffsep), un octet par point, même taille.
pub struct Cmjn<'a> {
    pub c: &'a [u8],
    pub m: &'a [u8],
    pub y: &'a [u8],
    pub k: &'a [u8],
    pub or: Option<&'a [u8]>,
    pub rd: Option<&'a [u8]>,
}

/// Ordre des plans rendus par `separer`.
pub const ENCRES: [Encre; 9] = [Encre::C, Encre::Lc, Encre::M, Encre::Lm, Encre::Y, Encre::K, Encre::Lk, Encre::Or, Encre::Rd];

pub fn separer(e: &Cmjn, p: &Separation) -> Result<Vec<(Encre, Vec<u8>)>, String> {
    let n = e.c.len();
    let tailles = [Some(e.m), Some(e.y), Some(e.k), e.or, e.rd];
    if tailles.iter().flatten().any(|s| s.len() != n) {
        return Err("canaux CMJN de tailles différentes".into());
    }
    let (tc, tm, tk) = (p.cyan.table(), p.magenta.table(), p.noir.table());
    let plafond = |v: u8, l: f32| (v as f32 * l.clamp(0.0, 1.0)).floor() as u8;
    let limite = (p.limite_totale.max(0.0) / 100.0 * 255.0).floor() as u32;
    let mut plans: Vec<Vec<u8>> = (0..9).map(|_| Vec::with_capacity(n)).collect();
    for i in 0..n {
        let (lc, c) = tc[e.c[i] as usize];
        let (lm, m) = tm[e.m[i] as usize];
        let (lk, k) = tk[e.k[i] as usize];
        let or = e.or.map_or(0, |s| plafond(s[i], p.limite_or));
        let rd = e.rd.map_or(0, |s| plafond(s[i], p.limite_rd));
        let mut v = [c, lc, m, lm, e.y[i], k, lk, or, rd].map(|x| plafond(x, p.limite_canal));
        let somme: u32 = v.iter().map(|&x| x as u32).sum();
        if somme > limite {
            for x in &mut v {
                *x = (*x as u32 * limite / somme) as u8;
            }
        }
        for (plan, x) in plans.iter_mut().zip(v) {
            plan.push(x);
        }
    }
    Ok(ENCRES.into_iter().zip(plans).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn densite(table: &[(u8, u8); 256], f: f32) -> Vec<f32> {
        table.iter().map(|&(l, k)| f * l as f32 + k as f32).collect()
    }

    #[test]
    fn rampe_monotone_et_fin_egale_au_fonce_seul() {
        let reglages = [
            Couple::default(),
            Couple { clair: 0.25, start: 0.05, max: 1.0, transition: 0.1, end: 0.2 },
            Couple { clair: 0.5, start: 0.0, max: 0.6, transition: 0.0, end: 0.6 },
            Couple { clair: 0.4, start: 0.0, max: 1.0, transition: 1.0, end: 0.0 },
        ];
        for c in reglages {
            let d = densite(&c.table(), c.clair);
            assert!(d.windows(2).all(|w| w[1] >= w[0] - 1e-3), "densité non monotone : {c:?} {d:?}");
            assert!((d[255] - 255.0).abs() <= 0.5, "fin {} ≠ 255 : {c:?}", d[255]);
            assert_eq!(d[0], 0.0);
        }
        // Par canal logique, à travers `separer`, sans plafond par canal (les courbes seules).
        let rampe: Vec<u8> = (0..=255).collect();
        let zero = vec![0u8; 256];
        let p = Separation { limite_canal: 1.0, ..Separation::default() };
        for canal in 0..3 {
            let mut e = Cmjn { c: &zero, m: &zero, y: &zero, k: &zero, or: None, rd: None };
            match canal {
                0 => e.c = &rampe,
                1 => e.m = &rampe,
                _ => e.k = &rampe,
            }
            let s = separer(&e, &p).unwrap();
            let (fonce, clair) = [(0, 1), (2, 3), (5, 6)][canal];
            let d: Vec<f32> = (0..256).map(|i| 0.3 * s[clair].1[i] as f32 + s[fonce].1[i] as f32).collect();
            assert!(d.windows(2).all(|w| w[1] >= w[0] - 1e-3), "canal {canal} non monotone");
            assert_eq!((s[fonce].1[255], s[clair].1[255]), (255, 0), "canal {canal} : 100 % = foncé seul");
            assert!(s[7].1.iter().chain(&s[8].1).all(|&v| v == 0), "Or/Rd inventés");
        }
        // Plafond par canal par défaut (0,85) : 100 % ne pose plus 255.
        let s = separer(&Cmjn { c: &rampe, m: &zero, y: &rampe, k: &zero, or: None, rd: None }, &Separation::default()).unwrap();
        assert_eq!((s[0].1[255], s[4].1[255]), (216, 216), "C et J plafonnés à 85 %");
        assert!(s.iter().all(|(_, v)| v.iter().all(|&x| x <= 216)));
    }

    #[test]
    fn limite_d_encre_totale_respectee() {
        let plein = vec![255u8; 256];
        let rampe: Vec<u8> = (0..=255).collect();
        for limite in [300.0, 150.0, 80.0] {
            let p = Separation { limite_totale: limite, ..Separation::default() };
            let e = Cmjn { c: &rampe, m: &plein, y: &rampe, k: &rampe, or: Some(&plein), rd: Some(&rampe) };
            let s = separer(&e, &p).unwrap();
            let max = (limite / 100.0 * 255.0).floor() as u32;
            for i in 0..256 {
                let somme: u32 = s.iter().map(|(_, v)| v[i] as u32).sum();
                assert!(somme <= max, "point {i} : {somme} > {max}");
            }
            // Réduction proportionnelle : l'Or plein reste le plus encré du point plein.
            assert!(s[7].1[255] >= s[0].1[255]);
        }
        // Plafonds Or et Rd, et tailles incohérentes refusées.
        let p = Separation { limite_or: 0.5, limite_rd: 0.25, limite_canal: 1.0, ..Separation::default() };
        let zero = vec![0u8; 256];
        let s = separer(&Cmjn { c: &zero, m: &zero, y: &zero, k: &zero, or: Some(&plein), rd: Some(&plein) }, &p).unwrap();
        assert_eq!((s[7].1[0], s[8].1[0]), (127, 63));
        assert!(separer(&Cmjn { c: &zero, m: &zero[..10], y: &zero, k: &zero, or: None, rd: None }, &p).is_err());
    }

    /// Entrée (0..=255) à partir de laquelle la foncée se pose.
    fn debut_du_fonce(c: Couple) -> usize {
        c.table().iter().position(|&(_, k)| k > 0).unwrap()
    }

    #[test]
    fn prereglages_de_bascule_donnent_les_courbes_attendues() {
        let base = Couple { clair: 0.3, start: 0.02, ..Couple::default() };
        for b in Bascule::TOUTES {
            let c = b.appliquer(base);
            assert_eq!((c.clair, c.start), (0.3, 0.02), "{b:?} garde la densité mesurée");
            let d = densite(&c.table(), c.clair);
            assert!(d.windows(2).all(|w| w[1] >= w[0] - 1e-3), "{b:?} non monotone");
            assert!((d[255] - 255.0).abs() <= 0.5);
        }
        assert_eq!(Bascule::Equilibree.appliquer(Couple::default()), Couple::default());
        // La foncée arrive le plus tôt en franche, le plus tard en douce.
        let [douce, equilibree, franche] = Bascule::TOUTES.map(|b| debut_du_fonce(b.appliquer(Couple::default())));
        assert!(franche < equilibree && equilibree < douce, "{franche} {equilibree} {douce}");
        // Valeurs repères, `clair` 0,3 : la claire seule va jusqu'à 30 % (douce), 24 %
        // (équilibrée), 18 % (franche) de densité ; à 100 %, la douce garde 15 % de claire.
        assert_eq!((douce, equilibree, franche), (77, 62, 47));
        let fin = |b: Bascule| b.appliquer(Couple::default()).table()[255];
        assert_eq!(fin(Bascule::Douce), (38, 244));
        assert_eq!(fin(Bascule::Equilibree), (0, 255));
        assert_eq!(fin(Bascule::Franche), (0, 255));
        // La douce pose plus de claire que la franche sur toute la rampe.
        let (td, tf) = (Bascule::Douce.appliquer(Couple::default()).table(), Bascule::Franche.appliquer(Couple::default()).table());
        assert!((0..256).all(|i| td[i].0 >= tf[i].0));
        // Appliquée au mode : C, M et K suivent la bascule, Y, Or et Rd ne changent pas.
        let p = Separation::du_mode(Some(Bascule::Franche), Some(260.0));
        assert_eq!((p.cyan, p.magenta, p.noir), (Bascule::Franche.appliquer(Couple::default()), Bascule::Franche.appliquer(Couple::default()), Bascule::Franche.appliquer(Couple::default())));
        assert_eq!(p.limite_totale, 260.0);
        assert_eq!(Separation::du_mode(None, None), Separation::default());
        assert_eq!(serde_json::to_string(&Bascule::Equilibree).unwrap(), "\"equilibree\"");
    }

    #[test]
    fn reglages_serialises_et_completes_par_defaut() {
        let p: Separation = serde_json::from_str(r#"{"cyan":{"clair":0.25},"limite_totale":280}"#).unwrap();
        assert_eq!(p.cyan.clair, 0.25);
        assert_eq!(p.cyan.max, 0.8);
        assert_eq!(p.limite_totale, 280.0);
        assert_eq!(serde_json::from_str::<Separation>(&serde_json::to_string(&p).unwrap()).unwrap(), p);
    }
}
