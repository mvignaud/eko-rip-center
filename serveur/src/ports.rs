//! Ports de sortie par le réseau en plus du flux TCP brut (roadmap 9.4) : IPP (Print-Job
//! sur HTTP, vers une file CUPS ou une imprimante qui le parle) et renvoi vers un autre
//! E-KO RIP Center, qui passe le flux à sa machine. Le flux part au fil de l'eau, en HTTP
//! par morceaux (chunked) : un rouleau de plusieurs Go ne passe jamais en mémoire.

use crate::machines::Connexion;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PORT_IPP: u16 = 631;

/// Réponse attendue au plus tard après la fin de l'envoi : CUPS répond dès la file prise,
/// un autre RIP dès que sa machine a tout reçu.
const ATTENTE_REPONSE: Duration = Duration::from_secs(300);

fn chemin_absolu(c: &str) -> String {
    if c.starts_with('/') { c.to_string() } else { format!("/{c}") }
}

/// URI de l'imprimante, montrée à l'écran et envoyée dans `printer-uri`.
pub fn uri_ipp(hote: &str, port: u16, chemin: &str) -> String {
    format!("ipp://{hote}:{port}{}", chemin_absolu(chemin))
}

/// Nom d'hôte, IPv4 ou IPv6 entre crochets.
pub fn hote_valide(h: &str) -> bool {
    (1..=253).contains(&h.len()) && h.chars().all(|c| c.is_ascii_alphanumeric() || ".-_:[]".contains(c))
}

/// Chemin d'une URI : ASCII visible, sans espace.
pub fn chemin_valide(c: &str) -> bool {
    c.len() <= 512 && c.chars().all(|c| c.is_ascii_graphic())
}

fn tronquer(s: &str, n: usize) -> &str {
    let fin = s.char_indices().map(|(i, c)| i + c.len_utf8()).take_while(|&i| i <= n).last().unwrap_or(0);
    &s[..fin]
}

/// Requête IPP 1.1 Print-Job, document au format `application/octet-stream` : la file CUPS
/// « raw » ou l'imprimante le prend tel quel, le flux est déjà dans sa langue.
pub fn requete_print_job(uri: &str, nom: &str) -> Vec<u8> {
    // Version 1.1, opération 0x0002, requête n° 1, groupe des attributs d'opération.
    let mut o = vec![0x01, 0x01, 0x00, 0x02, 0, 0, 0, 1, 0x01];
    for (etiquette, attribut, valeur) in [
        (0x47, "attributes-charset", "utf-8"),
        (0x48, "attributes-natural-language", "fr"),
        (0x45, "printer-uri", uri),
        (0x42, "requesting-user-name", "eko-rip"),
        (0x42, "job-name", tronquer(nom, 255)),
        (0x49, "document-format", "application/octet-stream"),
    ] {
        o.push(etiquette);
        o.extend((attribut.len() as u16).to_be_bytes());
        o.extend(attribut.as_bytes());
        o.extend((valeur.len() as u16).to_be_bytes());
        o.extend(valeur.as_bytes());
    }
    o.push(0x03);
    o
}

/// Nom lisible des refus IPP les plus courants (RFC 8011, § 6.3).
fn statut_ipp(code: u16) -> String {
    let nom = match code {
        0x0400 => "requête refusée",
        0x0401 => "interdit",
        0x0402 => "authentification demandée",
        0x0406 => "file introuvable",
        0x040A => "format de document refusé",
        0x0500 => "erreur interne de l'imprimante",
        0x0506 => "file fermée aux travaux",
        0x0507 => "imprimante occupée",
        _ => "refus",
    };
    format!("IPP {code:#06x} ({nom})")
}

enum Fin {
    /// TCP brut : fermer en écriture suffit.
    Brut,
    Ipp,
    /// Renvoi : fin du corps multipart, puis réponse de l'autre RIP.
    Rip { suffixe: Vec<u8> },
}

/// Flux ouvert vers une machine, à écrire puis à `finir`.
pub struct Flux {
    tcp: BufWriter<TcpStream>,
    fin: Fin,
    cible: String,
}

fn morceau(w: &mut impl Write, o: &[u8]) -> std::io::Result<()> {
    write!(w, "{:x}\r\n", o.len())?;
    w.write_all(o)?;
    w.write_all(b"\r\n")
}

impl Write for Flux {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.fin {
            Fin::Brut => self.tcp.write(buf),
            // Un morceau vide terminerait le corps HTTP.
            _ if buf.is_empty() => Ok(0),
            _ => morceau(&mut self.tcp, buf).map(|()| buf.len()).map_err(|e| self.refus_en_cours(e)),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.tcp.flush()
    }
}

impl Flux {
    /// Le serveur a coupé avant la fin (jeton refusé, file introuvable) : sa réponse, si
    /// elle est arrivée, dit pourquoi mieux que « connexion rompue ».
    fn refus_en_cours(&self, e: std::io::Error) -> std::io::Error {
        let reponse = self.tcp.get_ref().try_clone().ok().and_then(|t| {
            t.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
            lire_reponse(t).ok()
        });
        match reponse {
            Some((statut, corps)) => std::io::Error::other(format!("{} : HTTP {statut} {}", self.cible, tronquer(String::from_utf8_lossy(&corps).trim(), 300))),
            None => e,
        }
    }

    /// Double de la connexion, pour la couper d'un autre fil (alerte de l'imprimante).
    pub fn coupure(&self) -> std::io::Result<TcpStream> {
        self.tcp.get_ref().try_clone()
    }

    /// Termine l'envoi ; en IPP et en renvoi, attend la réponse et dit si le flux est pris.
    pub fn finir(self) -> Result<(), String> {
        let Flux { mut tcp, fin, cible } = self;
        let e = |e: std::io::Error| format!("{cible} : {e}");
        if let Fin::Rip { suffixe } = &fin {
            morceau(&mut tcp, suffixe).map_err(e)?;
        }
        if !matches!(fin, Fin::Brut) {
            tcp.write_all(b"0\r\n\r\n").map_err(e)?;
        }
        let tcp = tcp.into_inner().map_err(|x| e(x.into_error()))?;
        if matches!(fin, Fin::Brut) {
            return tcp.shutdown(std::net::Shutdown::Write).map_err(e);
        }
        tcp.set_read_timeout(Some(ATTENTE_REPONSE)).map_err(e)?;
        let (statut, corps) = lire_reponse(tcp).map_err(e)?;
        match fin {
            Fin::Ipp if statut != 200 => Err(format!("{cible} : HTTP {statut}")),
            Fin::Ipp => match corps.get(2..4).map(|c| u16::from_be_bytes([c[0], c[1]])) {
                Some(code) if code < 0x0100 => Ok(()),
                Some(code) => Err(format!("{cible} : {}", statut_ipp(code))),
                None => Err(format!("{cible} : réponse IPP illisible")),
            },
            _ if (200..300).contains(&statut) => Ok(()),
            _ => Err(format!("{cible} : HTTP {statut} {}", tronquer(String::from_utf8_lossy(&corps).trim(), 300))),
        }
    }
}

/// Statut et corps d'une réponse HTTP/1.1 (corps à longueur donnée, par morceaux, ou
/// jusqu'à la fermeture) ; les réponses 1xx sont passées.
fn lire_reponse(tcp: TcpStream) -> std::io::Result<(u16, Vec<u8>)> {
    let invalide = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, m.to_string());
    let mut r = BufReader::new(tcp);
    loop {
        let mut ligne = String::new();
        r.read_line(&mut ligne)?;
        let statut: u16 = ligne.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or_else(|| invalide("réponse HTTP illisible"))?;
        let (mut longueur, mut morceaux) = (None, false);
        loop {
            let mut l = String::new();
            if r.read_line(&mut l)? == 0 || l.trim().is_empty() {
                break;
            }
            let (nom, valeur) = l.split_once(':').unwrap_or((&l, ""));
            match nom.trim().to_ascii_lowercase().as_str() {
                "content-length" => longueur = valeur.trim().parse::<u64>().ok(),
                "transfer-encoding" => morceaux = valeur.to_ascii_lowercase().contains("chunked"),
                _ => {}
            }
        }
        if (100..200).contains(&statut) {
            continue;
        }
        // Corps borné à 1 Mo : seul le début compte (statut IPP, code d'erreur du RIP).
        let mut corps = vec![];
        if morceaux {
            loop {
                let mut l = String::new();
                r.read_line(&mut l)?;
                let n = usize::from_str_radix(l.trim().split(';').next().unwrap_or(""), 16).map_err(|_| invalide("morceau illisible"))?;
                if n == 0 || corps.len() + n > 1 << 20 {
                    break;
                }
                let debut = corps.len();
                corps.resize(debut + n, 0);
                r.read_exact(&mut corps[debut..])?;
                r.read_line(&mut String::new())?;
            }
        } else {
            r.take(longueur.unwrap_or(1 << 20).min(1 << 20)).read_to_end(&mut corps)?;
        }
        return Ok((statut, corps));
    }
}

/// Requête HTTP par morceaux, en-têtes écrits ; le corps suit.
fn post(hote: &str, port: u16, chemin: &str, type_contenu: &str, autres: &str) -> Result<BufWriter<TcpStream>, String> {
    let tcp = crate::machines::ouvrir_reseau(hote, port)?;
    let mut w = BufWriter::with_capacity(1 << 16, tcp);
    write!(w, "POST {chemin} HTTP/1.1\r\nHost: {hote}:{port}\r\nUser-Agent: E-KO RIP Center/{}\r\nContent-Type: {type_contenu}\r\n{autres}Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n", env!("CARGO_PKG_VERSION")).map_err(|e| format!("{hote}:{port} : {e}"))?;
    Ok(w)
}

/// Ouvre le flux vers la machine (réseau brut, IPP, autre RIP). `nom` : nom du travail
/// pour la file de l'imprimante ou de l'autre RIP.
pub fn ouvrir(c: &Connexion, nom: &str) -> Result<Flux, String> {
    match c {
        Connexion::Reseau { hote, port } => Ok(Flux { tcp: BufWriter::with_capacity(1 << 16, crate::machines::ouvrir_reseau(hote, *port)?), fin: Fin::Brut, cible: format!("{hote}:{port}") }),
        Connexion::Ipp { hote, port, chemin } => {
            let uri = uri_ipp(hote, *port, chemin);
            let mut tcp = post(hote, *port, &chemin_absolu(chemin), "application/ipp", "")?;
            morceau(&mut tcp, &requete_print_job(&uri, nom)).map_err(|e| format!("{uri} : {e}"))?;
            Ok(Flux { tcp, fin: Fin::Ipp, cible: uri })
        }
        Connexion::RipEko { hote, port, machine, jeton } => {
            let limite = uuid::Uuid::new_v4().simple().to_string();
            let mut tcp = post(hote, *port, &format!("/api/renvoi/{machine}"), &format!("multipart/form-data; boundary={limite}"), &format!("Authorization: Bearer {jeton}\r\n"))?;
            let nom: String = tronquer(nom, 255).chars().map(|c| if c.is_control() || c == '"' { ' ' } else { c }).collect();
            let entete = format!("--{limite}\r\nContent-Disposition: form-data; name=\"nom\"\r\n\r\n{nom}\r\n--{limite}\r\nContent-Disposition: form-data; name=\"flux\"; filename=\"flux.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n");
            let cible = format!("E-KO RIP {hote}:{port}/{machine}");
            morceau(&mut tcp, entete.as_bytes()).map_err(|e| format!("{cible} : {e}"))?;
            Ok(Flux { tcp, fin: Fin::Rip { suffixe: format!("\r\n--{limite}--\r\n").into_bytes() }, cible })
        }
        _ => Err("connexion sans flux réseau".into()),
    }
}

/// Jeton qui ouvre `/api/renvoi/` aux autres RIP : seule son empreinte est gardée.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ConfigRenvoi {
    pub jeton_sha256: Option<String>,
}

pub fn fichier(donnees: &Path) -> PathBuf {
    donnees.join("renvoi.json")
}

pub fn lire(donnees: &Path) -> ConfigRenvoi {
    crate::etat::lire(&fichier(donnees)).ok().flatten().unwrap_or_default()
}

pub fn ecrire(donnees: &Path, c: &ConfigRenvoi) -> std::io::Result<()> {
    crate::etat::ecrire(&fichier(donnees), c, false)
}

pub fn jeton_valide(donnees: &Path, presente: &str) -> bool {
    lire(donnees).jeton_sha256.as_deref().is_some_and(|attendu| crate::erp::egal(crate::erp::empreinte(presente).as_bytes(), attendu.as_bytes()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Lit une requête HTTP par morceaux : (ligne de requête et en-têtes, corps).
    pub(crate) fn lire_requete(f: &mut TcpStream) -> (String, Vec<u8>) {
        let mut r = BufReader::new(f);
        let mut tete = String::new();
        loop {
            let mut l = String::new();
            r.read_line(&mut l).unwrap();
            if l.trim().is_empty() {
                break;
            }
            tete += &l;
        }
        assert!(tete.contains("Transfer-Encoding: chunked"), "{tete}");
        let mut corps = vec![];
        loop {
            let mut l = String::new();
            r.read_line(&mut l).unwrap();
            let n = usize::from_str_radix(l.trim(), 16).unwrap();
            if n == 0 {
                r.read_line(&mut l).unwrap();
                break;
            }
            let debut = corps.len();
            corps.resize(debut + n, 0);
            r.read_exact(&mut corps[debut..]).unwrap();
            r.read_line(&mut l).unwrap();
        }
        (tete, corps)
    }

    /// Imprimante IPP d'essai : garde la requête, répond `code`.
    fn imprimante(code: u16) -> (u16, std::thread::JoinHandle<(String, Vec<u8>)>) {
        let ecoute = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let fil = std::thread::spawn(move || {
            let (mut f, _) = ecoute.accept().unwrap();
            let recu = lire_requete(&mut f);
            let [a, b] = code.to_be_bytes();
            let ipp = [0x01, 0x01, a, b, 0, 0, 0, 1, 0x03];
            // Réponse par morceaux, comme CUPS.
            write!(f, "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n", ipp.len()).unwrap();
            f.write_all(&ipp).unwrap();
            f.write_all(b"\r\n0\r\n\r\n").unwrap();
            recu
        });
        (port, fil)
    }

    #[test]
    fn uri_et_requete_print_job() {
        assert_eq!(uri_ipp("192.168.1.20", 631, "printers/Epson"), "ipp://192.168.1.20:631/printers/Epson");
        assert_eq!(uri_ipp("cups.local", 8631, "/ipp/print"), "ipp://cups.local:8631/ipp/print");
        assert!(hote_valide("192.168.1.20") && hote_valide("[fe80::1]") && !hote_valide("") && !hote_valide("a b") && !hote_valide("a/b"));
        assert!(chemin_valide("/printers/Epson_S80600") && !chemin_valide("/a b"));
        let r = requete_print_job("ipp://h:631/p", "Affiche été");
        assert_eq!(&r[..9], &[0x01, 0x01, 0x00, 0x02, 0, 0, 0, 1, 0x01]);
        assert_eq!(r.last(), Some(&0x03));
        // Attribut lu comme le ferait l'imprimante : étiquette, nom, valeur.
        let i = r.windows(11).position(|w| w == b"printer-uri").unwrap();
        assert_eq!((r[i - 3], &r[i - 2..i]), (0x45, &[0u8, 11][..]));
        assert_eq!(&r[i + 11..i + 13], &[0, 13]);
        assert_eq!(&r[i + 13..i + 26], b"ipp://h:631/p");
        assert!(r.windows("Affiche été".len()).any(|w| w == "Affiche été".as_bytes()));
        assert_eq!((tronquer("été", 2), tronquer("été", 1)), ("é", ""), "pas de caractère coupé");
    }

    #[test]
    fn envoi_ipp() {
        let (port, fil) = imprimante(0x0000);
        let c = Connexion::Ipp { hote: "127.0.0.1".into(), port, chemin: "printers/raw".into() };
        let mut f = ouvrir(&c, "Bâche 3 m").unwrap();
        f.write_all(b"ESC/P2 ").unwrap();
        f.write_all(&[]).unwrap();
        f.write_all(b"flux").unwrap();
        f.finir().unwrap();
        let (tete, corps) = fil.join().unwrap();
        assert!(tete.starts_with("POST /printers/raw HTTP/1.1\r\n") && tete.contains("Content-Type: application/ipp"), "{tete}");
        let requete = requete_print_job(&format!("ipp://127.0.0.1:{port}/printers/raw"), "Bâche 3 m");
        assert_eq!(&corps[..requete.len()], &requete[..]);
        assert_eq!(&corps[requete.len()..], b"ESC/P2 flux");

        // Imprimante occupée : refus nommé.
        let (port, fil) = imprimante(0x0507);
        let e = crate::machines::envoyer(&Connexion::Ipp { hote: "127.0.0.1".into(), port, chemin: "/ipp/print".into() }, b"x").unwrap_err();
        assert!(e.contains("0x0507") && e.contains("occupée"), "{e}");
        fil.join().unwrap();
    }

    /// Essai sur un vrai CUPS : file brute `brut` dont un backend d'essai garde le flux dans
    /// `/tmp/cups-sortie/brut.bin` (voir `outils/cups-essai.sh`), puis
    /// `cargo test -- --ignored cups`.
    #[test]
    #[ignore]
    fn cups_file_brute() {
        let sortie = Path::new("/tmp/cups-sortie/brut.bin");
        let _ = std::fs::remove_file(sortie);
        let flux: Vec<u8> = (0..300_000u32).flat_map(|i| i.to_le_bytes()).collect();
        let c = Connexion::Ipp { hote: "localhost".into(), port: PORT_IPP, chemin: "/printers/brut".into() };
        let mut f = ouvrir(&c, "Essai E-KO RIP").unwrap();
        for bloc in flux.chunks(70_000) {
            f.write_all(bloc).unwrap();
        }
        f.finir().unwrap();
        for _ in 0..100 {
            if std::fs::read(sortie).is_ok_and(|o| o == flux) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("{} : flux absent ou différent", sortie.display());
    }
}
