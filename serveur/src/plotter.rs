//! Envoi d'un fichier de découpe au plotter.
//!
//! Le Mutoh Kona se branche en USB, mais sa puce FTDI FT2232 (identifiant Mutoh
//! `0c30:6010`) est un convertisseur USB-série : sous Linux, le module `ftdi_sio` le
//! présente comme un port série (`/dev/ttyUSB0`, renommé `/dev/kona` par la règle udev
//! du paquet). On règle le port avec `stty`, puis on y écrit le HP-GL.

use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Réglages `stty` par défaut. À confirmer sur le Kona : la vitesse est celle de la
/// liaison entre la puce FTDI et la carte du plotter.
pub const STTY_DEFAUT: &str = "9600 cs8 -cstopb -parenb crtscts raw -echo";

/// Taille des écritures : petite, pour que le contrôle de flux du plotter freine l'envoi
/// au fil de la coupe au lieu de remplir d'un coup un tampon du noyau.
const MORCEAU: usize = 256;

pub fn envoyer(port: &Path, reglages: &str, donnees: &[u8]) -> Result<(), String> {
    let sortie = Command::new("stty")
        .arg("-F")
        .arg(port)
        .args(reglages.split_whitespace())
        .output()
        .map_err(|e| format!("stty introuvable : {e}"))?;
    if !sortie.status.success() {
        return Err(format!("réglage du port {} : {}", port.display(), String::from_utf8_lossy(&sortie.stderr).trim()));
    }
    // ponytail: écriture bloquante sans délai maximal ; un plotter éteint en cours d'envoi
    // bloque cet envoi seul (les rendus continuent). Délai avec O_NONBLOCK + poll si besoin.
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(port)
        .map_err(|e| format!("ouverture de {} : {e}", port.display()))?;
    for morceau in donnees.chunks(MORCEAU) {
        f.write_all(morceau).map_err(|e| format!("envoi au plotter : {e}"))?;
    }
    f.flush().map_err(|e| format!("envoi au plotter : {e}"))?;
    f.sync_all().ok();
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::io::Read;

    /// Un vrai terminal virtuel : `stty` doit l'accepter et chaque octet doit arriver.
    #[test]
    fn le_fichier_arrive_intact_sur_un_terminal() {
        let (mut maitre, mut esclave) = (0, 0);
        let mut nom = [0 as libc::c_char; 128];
        let r = unsafe { libc::openpty(&mut maitre, &mut esclave, nom.as_mut_ptr(), std::ptr::null(), std::ptr::null()) };
        assert_eq!(r, 0, "openpty");
        let chemin = unsafe { std::ffi::CStr::from_ptr(nom.as_ptr()) }.to_string_lossy().into_owned();

        let donnees: Vec<u8> = "IN;SP1;PU0,0;PD1016,0,1016,1016,0,1016,0,0;PU;SP0;".repeat(200).into_bytes();
        let attendu = donnees.clone();
        let lecteur = std::thread::spawn(move || {
            let mut f = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(maitre) };
            let mut recu = vec![0u8; attendu.len()];
            f.read_exact(&mut recu).unwrap();
            assert_eq!(recu, attendu);
        });
        // Sans crtscts : un pty n'a pas de lignes de contrôle de flux.
        envoyer(Path::new(&chemin), "9600 cs8 -cstopb -parenb raw -echo", &donnees).unwrap();
        lecteur.join().unwrap();
        unsafe { libc::close(esclave) };
    }

    #[test]
    fn un_port_absent_est_signale() {
        let e = envoyer(Path::new("/dev/ce-plotter-n-existe-pas"), STTY_DEFAUT, b"IN;").unwrap_err();
        assert!(e.contains("/dev/ce-plotter-n-existe-pas"), "{e}");
    }
}
