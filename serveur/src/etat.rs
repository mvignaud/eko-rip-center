//! Fichiers d'état JSON (`file.json`, `machines.json`, `reglages.json`, `tarifs.json`,
//! `utilisateurs.json`, `job.json`) résistants aux coupures de courant.
//!
//! Écriture : fichier temporaire synchronisé sur le disque, copie de la version en place
//! (N-1) dans `<nom>.prec` si elle est lisible, renommage, synchronisation du dossier.
//! Lecture : un fichier illisible est mis de côté (`<nom>.illisible`), la version N-1 est
//! reprise et une alerte est gardée pour l'écran Système. Jamais une valeur vide en silence.

use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Fichier d'état trouvé illisible ; `reprise` : la version N-1 a été reprise.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Alerte {
    pub fichier: String,
    pub reprise: bool,
}

static ALERTES: Mutex<Vec<Alerte>> = Mutex::new(Vec::new());

/// Alertes de lecture depuis le démarrage, pour l'écran Système.
pub fn alertes() -> Vec<Alerte> {
    ALERTES.lock().unwrap().clone()
}

fn alerter(fichier: String, reprise: bool) {
    let a = Alerte { fichier, reprise };
    let mut v = ALERTES.lock().unwrap();
    if !v.contains(&a) {
        eprintln!("ATTENTION : {} illisible (coupure ?) : {}", a.fichier, if reprise { "version précédente reprise" } else { "sans version précédente lisible, copie gardée en .illisible" });
        v.push(a);
    }
}

fn avec_suffixe(chemin: &Path, suffixe: &str) -> PathBuf {
    let mut n = chemin.as_os_str().to_owned();
    n.push(suffixe);
    n.into()
}

pub fn precedent(chemin: &Path) -> PathBuf {
    avec_suffixe(chemin, ".prec")
}

/// Écrit `v` en JSON à `chemin` ; `prive` : droits 0600 (comptes).
pub fn ecrire<T: Serialize + ?Sized>(chemin: &Path, v: &T, prive: bool) -> std::io::Result<()> {
    let tmp = avec_suffixe(chemin, ".tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        #[cfg(unix)]
        if prive {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        f.write_all(&serde_json::to_vec_pretty(v)?)?;
        f.sync_all()?;
    }
    // N-1 seulement si la version en place se lit : une copie illisible écraserait la bonne.
    if std::fs::read(chemin).ok().is_some_and(|o| serde_json::from_slice::<serde_json::Value>(&o).is_ok()) {
        std::fs::copy(chemin, precedent(chemin))?;
    }
    std::fs::rename(&tmp, chemin)?;
    if let Some(d) = chemin.parent() {
        let _ = std::fs::File::open(d).and_then(|d| d.sync_all());
    }
    Ok(())
}

/// `Ok(None)` : fichier absent (premier démarrage) ; `Err` : illisible, sans N-1 lisible.
pub fn lire<T: DeserializeOwned>(chemin: &Path) -> Result<Option<T>, ()> {
    let lu = |c: &Path| std::fs::read(c).ok().map(|o| serde_json::from_slice::<T>(&o));
    let nom = chemin.file_name().unwrap_or_default().to_string_lossy().into_owned();
    match lu(chemin) {
        Some(Ok(v)) => return Ok(Some(v)),
        None if !precedent(chemin).exists() => return Ok(None),
        Some(Err(_)) => {
            let _ = std::fs::copy(chemin, avec_suffixe(chemin, ".illisible"));
        }
        None => {}
    }
    match lu(&precedent(chemin)) {
        Some(Ok(v)) => {
            alerter(nom, true);
            Ok(Some(v))
        }
        _ => {
            alerter(nom, false);
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_tronque_reprend_la_version_precedente() {
        let d = std::env::temp_dir().join(format!("etat-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("file.json");
        assert_eq!(lire::<Vec<u32>>(&f), Ok(None), "absent = premier démarrage");
        ecrire(&f, &vec![1u32], false).unwrap();
        ecrire(&f, &vec![1u32, 2], false).unwrap();
        // Coupure au milieu d'une écriture : JSON tronqué.
        std::fs::write(&f, b"[1, 2, 3").unwrap();
        assert_eq!(lire::<Vec<u32>>(&f), Ok(Some(vec![1])), "N-1 reprise");
        assert!(d.join("file.json.illisible").exists());
        assert!(alertes().contains(&Alerte { fichier: "file.json".into(), reprise: true }));
        // Une écriture sur un fichier abîmé ne remplace pas la bonne N-1.
        ecrire(&f, &vec![7u32], false).unwrap();
        assert_eq!(std::fs::read_to_string(precedent(&f)).unwrap().split_whitespace().collect::<String>(), "[1]");
        // Tout illisible : erreur, pas de valeur vide.
        std::fs::write(&f, b"x").unwrap();
        std::fs::write(precedent(&f), b"y").unwrap();
        assert_eq!(lire::<Vec<u32>>(&f), Err(()));
        std::fs::remove_dir_all(d).unwrap();
    }
}
