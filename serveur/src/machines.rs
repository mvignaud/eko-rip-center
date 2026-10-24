//! Catalogue des machines de l'atelier (`<données>/machines.json`) : modèle, laize,
//! connexion. Sans fichier, le catalogue d'origine : l'Epson SC-S80600 et le Mutoh Kona.
//! Chaque modèle sait imprimer, découper, ou les deux (Roland VersaCAMM).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modele {
    /// Epson SureColor SC-S80600 (impression ; sans pilote, plaques copiées en dossier).
    EpsonScS80600,
    /// Mutoh Kona (découpe, HP-GL à 0,01 mm).
    MutohKona,
    /// Graphtec FC7000 (découpe).
    GraphtecFc7000,
    /// Roland VersaCAMM VP-300 / VP-540 (impression et découpe par la machine).
    RolandVersacamm,
}

impl Modele {
    pub fn imprime(self) -> bool {
        matches!(self, Modele::EpsonScS80600 | Modele::RolandVersacamm)
    }

    pub fn decoupe(self) -> bool {
        !matches!(self, Modele::EpsonScS80600)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Impression,
    Decoupe,
}

/// Comment joindre la machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Connexion {
    /// Port série (adaptateur USB-série compris) réglé par `stty`.
    Serie { port: PathBuf, stty: String },
    /// Flux brut sur le réseau (TCP, port 9100 en général).
    Reseau { hote: String, port: u16 },
    /// Périphérique ou fichier écrit tel quel (`/dev/usb/lp0`…).
    Fichier { chemin: PathBuf },
    /// Pas de pilote : la sortie est copiée dans `<données>/sorties/<machine>/`.
    Dossier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    /// Identifiant stable (lettres, chiffres, `-`) : file d'attente, dossiers de sortie.
    pub id: String,
    pub nom: String,
    pub modele: Modele,
    /// Laize utile, en mm.
    pub laize_mm: f64,
    pub connexion: Connexion,
    /// Adresse SNMP pour lire le média chargé (Printer-MIB).
    #[serde(default)]
    pub snmp: Option<String>,
    /// Pas de la découpe en HP-GL, en mm (par défaut : 0,01 pour le Kona, 0,025 sinon).
    #[serde(default)]
    pub pas_mm: Option<f64>,
    /// Décalage de la découpe (x, y) en mm, ±20 : rattrape l'écart entre le couteau et
    /// l'impression mesuré sur une coupe d'essai.
    #[serde(default)]
    pub decalage_mm: [f64; 2],
    /// Cadence d'impression en m²/h, sur toute la laize (sinon : tableau de l'Epson).
    #[serde(default)]
    pub m2_heure: Option<f64>,
}

impl Machine {
    /// Minutes pour imprimer `longueur_mm` de rouleau à `dpi` ; `None` sans cadence connue.
    /// Epson SC-S80600 (fiche publique) : 360 dpi 94 m²/h (1 passe), 720 dpi 17 m²/h
    /// (720 × 1440, 6 passes), au-delà 6,6 m²/h (1440 × 1440, 16 passes).
    pub fn duree_impression_min(&self, longueur_mm: f64, dpi: u32) -> Option<f64> {
        let cadence = self.m2_heure.or_else(|| (self.modele == Modele::EpsonScS80600).then_some(if dpi <= 360 { 94.0 } else if dpi <= 720 { 17.0 } else { 6.6 }))?;
        (self.modele.imprime() && longueur_mm > 0.0).then(|| self.laize_mm * longueur_mm / 1e6 / cadence * 60.0)
    }

    pub fn pas_mm(&self) -> f64 {
        self.pas_mm.unwrap_or(if self.modele == Modele::MutohKona { 0.01 } else { 0.025 })
    }

    pub fn fait(&self, o: Operation) -> bool {
        match o {
            Operation::Impression => self.modele.imprime(),
            Operation::Decoupe => self.modele.decoupe(),
        }
    }
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("machines.json")
}

/// Catalogue d'origine : l'Epson (sorties en dossier) et le Kona sur son port série.
pub fn catalogue_initial(port_kona: &Path, stty: &str, snmp_epson: Option<String>) -> Vec<Machine> {
    vec![
        Machine { id: "epson".into(), nom: "Epson SC-S80600".into(), modele: Modele::EpsonScS80600, laize_mm: 1626.0, connexion: Connexion::Dossier, snmp: snmp_epson, pas_mm: None, decalage_mm: [0.0; 2], m2_heure: None },
        Machine { id: "kona".into(), nom: "Mutoh Kona 1650".into(), modele: Modele::MutohKona, laize_mm: 1650.0, connexion: Connexion::Serie { port: port_kona.into(), stty: stty.into() }, snmp: None, pas_mm: None, decalage_mm: [0.0; 2], m2_heure: None },
    ]
}

pub fn lire(donnees: &Path) -> Option<Vec<Machine>> {
    serde_json::from_slice(&std::fs::read(fichier(donnees)).ok()?).ok()
}

pub fn ecrire(donnees: &Path, machines: &[Machine]) -> std::io::Result<()> {
    let tmp = fichier(donnees).with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(machines).unwrap())?;
    std::fs::rename(tmp, fichier(donnees))
}

pub fn verifier(machines: &[Machine]) -> Result<(), &'static str> {
    let id_valide = |id: &str| (1..=30).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let bon = machines.iter().enumerate().all(|(i, m)| {
        id_valide(&m.id)
            && !machines[..i].iter().any(|x| x.id == m.id)
            && !m.nom.trim().is_empty()
            && (100.0..=5000.0).contains(&m.laize_mm)
            && m.pas_mm.is_none_or(|p| [0.005, 0.01, 0.025, 0.05, 0.1].iter().any(|x| (x - p).abs() < 1e-9))
            && m.decalage_mm.iter().all(|d| (-20.0..=20.0).contains(d))
            && m.m2_heure.is_none_or(|v| (0.1..=500.0).contains(&v))
    });
    if bon { Ok(()) } else { Err("machine_invalide") }
}

/// Convertit le HP-GL produit par le RIP (pas de 0,01 mm, commandes `PU`/`PD` à coordonnées
/// x,y absolues) au pas `pas_mm` de la machine, décalé de `decalage_mm` (x, y).
pub fn changer_de_pas(hpgl: &str, pas_mm: f64, decalage_mm: [f64; 2]) -> String {
    let f = 0.01 / pas_mm;
    if (f - 1.0).abs() < 1e-12 && decalage_mm == [0.0; 2] {
        return hpgl.to_string();
    }
    hpgl.split(';')
        .map(|cmd| match cmd.get(..2) {
            Some(op @ ("PU" | "PD")) if cmd.len() > 2 => {
                let nombres: Vec<String> = cmd[2..]
                    .split(',')
                    .enumerate()
                    .map(|(i, n)| n.trim().parse::<f64>().map_or_else(|_| n.to_string(), |v| format!("{}", ((v + decalage_mm[i % 2] * 100.0) * f).round() as i64)))
                    .collect();
                format!("{op}{}", nombres.join(","))
            }
            _ => cmd.to_string(),
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Envoie les octets par la connexion de la machine (bloquant).
pub fn envoyer(c: &Connexion, octets: &[u8]) -> Result<(), String> {
    match c {
        Connexion::Serie { port, stty } => crate::plotter::envoyer(port, stty, octets),
        Connexion::Reseau { hote, port } => {
            use std::net::ToSocketAddrs;
            let adresse = (hote.as_str(), *port).to_socket_addrs().map_err(|e| format!("{hote}:{port} : {e}"))?.next().ok_or(format!("{hote} : adresse introuvable"))?;
            let mut flux = std::net::TcpStream::connect_timeout(&adresse, Duration::from_secs(5)).map_err(|e| format!("{adresse} : {e}"))?;
            flux.set_write_timeout(Some(Duration::from_secs(60))).map_err(|e| e.to_string())?;
            flux.write_all(octets).map_err(|e| format!("{adresse} : {e}"))?;
            flux.shutdown(std::net::Shutdown::Write).map_err(|e| e.to_string())
        }
        Connexion::Fichier { chemin } => std::fs::OpenOptions::new().write(true).open(chemin).and_then(|mut f| f.write_all(octets)).map_err(|e| format!("{} : {e}", chemin.display())),
        Connexion::Dossier => Err("machine sans connexion : sortie en dossier seulement".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_et_capacites() {
        let c = catalogue_initial(Path::new("/dev/kona"), "9600", None);
        assert!(verifier(&c).is_ok());
        // 1 m de rouleau sur 1626 mm à 720 dpi : 1,626 m² à 17 m²/h ≈ 5,7 min.
        assert!((c[0].duree_impression_min(1000.0, 720).unwrap() - 1.626 / 17.0 * 60.0).abs() < 1e-9);
        assert_eq!(c[1].duree_impression_min(1000.0, 720), None, "le Kona n'imprime pas");
        assert!(!c[1].fait(Operation::Impression) && c[1].fait(Operation::Decoupe));
        assert!(Modele::RolandVersacamm.imprime() && Modele::RolandVersacamm.decoupe());
        let mut double = c.clone();
        double[1].id = "epson".into();
        assert_eq!(verifier(&double), Err("machine_invalide"));
        // Format du fichier : la connexion est étiquetée par son type.
        let json = serde_json::to_value(&c[1]).unwrap();
        assert_eq!(json["connexion"]["type"], "serie");
        assert_eq!(json["modele"], "mutoh_kona");
    }

    #[test]
    fn conversion_de_pas() {
        let rip = "IN;SP1;PU0,0;PD2540,0,2540,2540;PU;SP0;";
        // 25,4 mm : 2540 pas de 0,01 mm, 1016 pas de 0,025 mm (HP-GL standard).
        assert_eq!(changer_de_pas(rip, 0.025, [0.0; 2]), "IN;SP1;PU0,0;PD1016,0,1016,1016;PU;SP0;");
        assert_eq!(changer_de_pas(rip, 0.01, [0.0; 2]), rip);
        // 1 mm à droite, 0,5 mm en arrière, au pas de 0,025 mm : +40 et −20.
        assert_eq!(changer_de_pas(rip, 0.025, [1.0, -0.5]), "IN;SP1;PU40,-20;PD1056,-20,1056,996;PU;SP0;");
        let mut m = catalogue_initial(Path::new("/dev/kona"), "9600", None).remove(1);
        assert_eq!(m.pas_mm(), 0.01);
        m.modele = Modele::GraphtecFc7000;
        assert_eq!(m.pas_mm(), 0.025);
        m.pas_mm = Some(0.03);
        assert!(verifier(&[m]).is_err(), "pas inconnu refusé");
    }

    #[test]
    fn envoi_reseau_et_fichier() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let recu = std::thread::spawn(move || {
            let (mut f, _) = ecoute.accept().unwrap();
            let mut o = vec![];
            std::io::Read::read_to_end(&mut f, &mut o).unwrap();
            o
        });
        envoyer(&Connexion::Reseau { hote: "127.0.0.1".into(), port }, b"IN;PU0,0;").unwrap();
        assert_eq!(recu.join().unwrap(), b"IN;PU0,0;");
        let f = std::env::temp_dir().join(format!("eko-rip-peripherique-{}", std::process::id()));
        std::fs::write(&f, b"").unwrap();
        envoyer(&Connexion::Fichier { chemin: f.clone() }, b"abc").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"abc");
        std::fs::remove_file(&f).unwrap();
        assert!(envoyer(&Connexion::Reseau { hote: "127.0.0.1".into(), port: 9 }, b"x").is_err());
        assert!(envoyer(&Connexion::Dossier, b"x").is_err());
    }
}
