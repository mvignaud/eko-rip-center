//! Catalogue des supports par machine, et leurs modes d'impression (`supports.json`).
//!
//! Support = ce qui est chargé dans la machine : nom, configuration d'encres, laize.
//! Mode = comment on imprime dessus : résolution, passes, chauffage. Chaque support et
//! chaque mode porte un identifiant stable (UUID), distinct de son nom.
//! ponytail: le prix reste dans `tarifs.json`, relié par le nom du support (une seule
//! source de prix) ; le travail passera aux identifiants (roadmap 1.2), puis restriction,
//! linéarisation, limites et profil ICC viendront se ranger dans le mode.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Version du format de `supports.json`.
pub const VERSION: u32 = 1;

/// Codes d'encre admis dans une configuration (ceux de l'Epson SC-S80600, plus blanc et métal).
pub const ENCRES: [&str; 11] = ["OR", "LK", "BK", "C", "M", "Y", "R", "LC", "LM", "WH", "MS"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeImpression {
    /// Absent à la création depuis le client : attribué par le serveur, puis immuable.
    #[serde(default)]
    pub id: Option<Uuid>,
    pub nom: String,
    /// Résolution horizontale × verticale (points par pouce).
    pub dpi: [u32; 2],
    pub passes: u8,
    /// Chauffage demandé (°C), si le mode l'impose.
    #[serde(default)]
    pub chauffage_c: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Support {
    #[serde(default)]
    pub id: Option<Uuid>,
    /// Machine du catalogue (`epson`…).
    pub machine: String,
    /// Nom affiché ; relie aussi le support à son prix dans `tarifs.json`.
    pub nom: String,
    /// Encres utilisées, dans l'ordre de la machine.
    pub encres: Vec<String>,
    /// Laize du rouleau (mm), si connue.
    #[serde(default)]
    pub laize_mm: Option<f64>,
    pub modes: Vec<ModeImpression>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalogue {
    pub version: u32,
    pub supports: Vec<Support>,
}

fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("supports.json")
}

/// Mode de la capture de l'atelier (720 × 720, 8 passes), donné aux supports repris.
pub fn mode_par_defaut() -> ModeImpression {
    ModeImpression { id: None, nom: "720 × 720 · 8 passes".into(), dpi: [720, 720], passes: 8, chauffage_c: None }
}

/// Catalogue en place ; au premier démarrage, repris des supports de `tarifs.json`
/// (rattachés à `machine`, configuration d'encres de l'Epson, mode par défaut) et écrit
/// aussitôt, pour que les identifiants attribués ne changent plus. Un fichier illisible
/// n'est pas réécrit (l'alerte de `etat` le signale) : la reprise ne sert qu'en mémoire.
pub fn lire(donnees: &Path, machine_par_defaut: &str) -> Catalogue {
    let lu = crate::etat::lire::<Catalogue>(&fichier(donnees));
    let absent = matches!(lu, Ok(None));
    if let Ok(Some(c)) = lu {
        return c;
    }
    let mut c = Catalogue {
        version: VERSION,
        supports: crate::couverture::lire(donnees)
            .supports
            .into_iter()
            .map(|s| Support {
                id: None,
                machine: machine_par_defaut.into(),
                laize_mm: s.rouleau.as_ref().map(|r| r.largeur_mm),
                nom: s.nom,
                encres: ENCRES[..9].iter().map(|e| e.to_string()).collect(),
                modes: vec![mode_par_defaut()],
            })
            .collect(),
    };
    attribuer_ids(&mut c.supports, &[]);
    if absent
        && let Err(e) = ecrire(donnees, &c)
    {
        eprintln!("supports.json : {e}");
    }
    c
}

pub fn ecrire(donnees: &Path, c: &Catalogue) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), c, false)
}

/// Donne un identifiant aux supports et modes qui n'en ont pas ; refuse un identifiant
/// inconnu (on ne choisit pas son UUID).
pub fn attribuer_ids(supports: &mut [Support], connus: &[Support]) -> bool {
    let connu_s = |id: &Uuid| connus.iter().any(|s| s.id.as_ref() == Some(id));
    let connu_m = |id: &Uuid| connus.iter().flat_map(|s| &s.modes).any(|m| m.id.as_ref() == Some(id));
    for s in supports.iter_mut() {
        match s.id {
            Some(id) if !connu_s(&id) => return false,
            None => s.id = Some(Uuid::new_v4()),
            _ => {}
        }
        for m in s.modes.iter_mut() {
            match m.id {
                Some(id) if !connu_m(&id) => return false,
                None => m.id = Some(Uuid::new_v4()),
                _ => {}
            }
        }
    }
    true
}

/// Contrôle les supports d'une machine (déjà identifiés).
pub fn verifier(supports: &[Support]) -> Result<(), &'static str> {
    let nom_bon = |n: &str| (1..=60).contains(&n.chars().count()) && n.trim() == n;
    let bon = supports.iter().enumerate().all(|(i, s)| {
        nom_bon(&s.nom)
            && !supports[..i].iter().any(|x| x.nom == s.nom || x.id == s.id)
            && !s.encres.is_empty()
            && s.encres.iter().enumerate().all(|(j, e)| ENCRES.contains(&e.as_str()) && !s.encres[..j].contains(e))
            && !(s.encres.iter().any(|e| e == "WH") && s.encres.iter().any(|e| e == "MS"))
            && s.laize_mm.is_none_or(|l| (50.0..=5000.0).contains(&l))
            && !s.modes.is_empty()
            && s.modes.iter().enumerate().all(|(j, m)| {
                nom_bon(&m.nom)
                    && !s.modes[..j].iter().any(|x| x.nom == m.nom || x.id == m.id)
                    && m.dpi.iter().all(|d| (90..=2880).contains(d))
                    && (1..=64).contains(&m.passes)
                    && m.chauffage_c.is_none_or(|c| (0.0..=120.0).contains(&c))
            })
    });
    if bon { Ok(()) } else { Err("supports_invalides") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reprise_des_supports_de_tarifs_sans_perte() {
        let d = std::env::temp_dir().join(format!("supports-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let mut t = crate::couverture::Tarifs::default();
        t.supports.push(crate::couverture::Support { nom: "Bâche 510 g".into(), prix_m2: Some(4.2), rouleau: None });
        crate::couverture::ecrire(&d, &t).unwrap();
        let c = lire(&d, "epson");
        let noms: Vec<&str> = c.supports.iter().map(|s| s.nom.as_str()).collect();
        assert_eq!(noms, t.supports.iter().map(|s| s.nom.as_str()).collect::<Vec<_>>(), "un support par support tarifé, même nom");
        assert_eq!(c.supports[0].laize_mm, t.supports[0].rouleau.as_ref().map(|r| r.largeur_mm));
        assert!(c.supports.iter().all(|s| s.id.is_some() && s.machine == "epson" && s.modes.len() == 1 && s.modes[0].id.is_some()));
        assert_eq!(verifier(&c.supports), Ok(()));
        // Une fois écrit, le catalogue est relu tel quel (mêmes identifiants).
        ecrire(&d, &c).unwrap();
        assert_eq!(lire(&d, "epson"), c);
        assert_eq!(crate::couverture::lire(&d), t, "tarifs.json intact");
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn identifiants_et_controles() {
        let mut s = vec![Support { id: None, machine: "epson".into(), nom: "Vinyle".into(), encres: vec!["C".into(), "M".into()], laize_mm: Some(1600.0), modes: vec![mode_par_defaut()] }];
        assert!(attribuer_ids(&mut s, &[]));
        let connu = s.clone();
        let mut renomme = connu.clone();
        renomme[0].nom = "Vinyle blanc".into();
        assert!(attribuer_ids(&mut renomme, &connu), "renommer garde l'identifiant");
        assert_eq!(renomme[0].id, connu[0].id);
        let mut invente = connu.clone();
        invente[0].id = Some(Uuid::new_v4());
        assert!(!attribuer_ids(&mut invente, &connu), "UUID choisi par le client refusé");
        assert_eq!(verifier(&s), Ok(()));
        for casse in [
            |s: &mut Support| s.encres.push("ZZ".into()),
            |s: &mut Support| s.encres.extend(["WH".into(), "MS".into()]),
            |s: &mut Support| s.modes.clear(),
            |s: &mut Support| s.modes[0].dpi = [720, 10],
            |s: &mut Support| s.modes[0].passes = 0,
            |s: &mut Support| s.nom = " Vinyle".into(),
        ] {
            let mut x = s.clone();
            casse(&mut x[0]);
            assert_eq!(verifier(&x), Err("supports_invalides"), "{x:?}");
        }
        let mut double = s.clone();
        double.push(s[0].clone());
        assert_eq!(verifier(&double), Err("supports_invalides"));
    }
}
