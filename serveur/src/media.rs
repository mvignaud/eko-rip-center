//! Média chargé dans l'imprimante, lu par SNMP v1 dans la Printer-MIB normalisée
//! (RFC 3805, table `prtInputTable`) : largeur du média en travers de l'avance
//! (`prtInputMediaDimXFeedDirChosen`), unité (`prtInputDimUnit`) et nom
//! (`prtInputMediaName`).
// ponytail: client SNMP v1 minimal (GetNext, BER à la main) pour trois colonnes ; à
// valider sur la SC-S80600 (capture à l'atelier) : si elle ne remplit pas la Printer-MIB,
// sa MIB privée Epson ou son canal USB prendront le relais.

use std::net::UdpSocket;
use std::time::Duration;

const PRT_INPUT: [u32; 10] = [1, 3, 6, 1, 2, 1, 43, 8, 2, 1];
const COL_UNITE: u32 = 3;
const COL_LARGEUR: u32 = 7;
const COL_NOM: u32 = 12;

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Media {
    /// Entrée papier (rouleau) de la Printer-MIB.
    pub entree: u32,
    pub largeur_mm: f64,
    pub nom: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
enum Valeur {
    Entier(i64),
    Texte(Vec<u8>),
    Autre,
}

fn longueur(n: usize) -> Vec<u8> {
    if n < 0x80 {
        return vec![n as u8];
    }
    let octets: Vec<u8> = n.to_be_bytes().into_iter().skip_while(|b| *b == 0).collect();
    let mut o = vec![0x80 | octets.len() as u8];
    o.extend(octets);
    o
}

fn tlv(t: u8, contenu: &[u8]) -> Vec<u8> {
    let mut o = vec![t];
    o.extend(longueur(contenu.len()));
    o.extend(contenu);
    o
}

fn entier(v: i64) -> Vec<u8> {
    let mut o: Vec<u8> = v.to_be_bytes().to_vec();
    while o.len() > 1 && ((o[0] == 0 && o[1] & 0x80 == 0) || (o[0] == 0xFF && o[1] & 0x80 != 0)) {
        o.remove(0);
    }
    tlv(0x02, &o)
}

fn oid(arcs: &[u32]) -> Vec<u8> {
    let mut o = vec![(arcs[0] * 40 + arcs[1]) as u8];
    for &a in &arcs[2..] {
        let mut septets = vec![(a & 0x7F) as u8];
        let mut x = a >> 7;
        while x > 0 {
            septets.push(0x80 | (x & 0x7F) as u8);
            x >>= 7;
        }
        o.extend(septets.into_iter().rev());
    }
    tlv(0x06, &o)
}

/// Message SNMP v1 : requête (`pdu` = 0xA1 GetNext) ou réponse (0xA2) d'une variable.
fn message(communaute: &str, pdu: u8, id: i64, arcs: &[u32], valeur: &[u8]) -> Vec<u8> {
    let variable = tlv(0x30, &[oid(arcs), valeur.to_vec()].concat());
    let corps = [entier(id), entier(0), entier(0), tlv(0x30, &variable)].concat();
    tlv(0x30, &[entier(0), tlv(0x04, communaute.as_bytes()), tlv(pdu, &corps)].concat())
}

/// Lit un TLV : (type, contenu, reste).
fn lire(o: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&t, o) = o.split_first()?;
    let (&l, mut o) = o.split_first()?;
    let n = if l & 0x80 == 0 {
        l as usize
    } else {
        let k = (l & 0x7F) as usize;
        let (octets, reste) = o.split_at_checked(k)?;
        o = reste;
        octets.iter().fold(0usize, |a, b| (a << 8) | *b as usize)
    };
    let (contenu, reste) = o.split_at_checked(n)?;
    Some((t, contenu, reste))
}

fn arcs(contenu: &[u8]) -> Vec<u32> {
    let Some((&premier, suite)) = contenu.split_first() else { return vec![] };
    let mut a = vec![(premier / 40) as u32, (premier % 40) as u32];
    let mut x = 0u32;
    for &b in suite {
        x = (x << 7) | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            a.push(x);
            x = 0;
        }
    }
    a
}

/// Réponse : (identifiant de requête, OID, valeur) de la première variable.
fn decoder(o: &[u8]) -> Option<(i64, Vec<u32>, Valeur)> {
    let (_, msg, _) = lire(o)?;
    let (_, _, msg) = lire(msg)?; // version
    let (_, _, msg) = lire(msg)?; // communauté
    let (t, pdu, _) = lire(msg)?;
    if t != 0xA2 {
        return None;
    }
    let (_, id, pdu) = lire(pdu)?;
    let id = id.iter().fold(if id.first().is_some_and(|b| b & 0x80 != 0) { -1i64 } else { 0 }, |a, b| (a << 8) | *b as i64);
    let (_, statut, pdu) = lire(pdu)?;
    if statut.iter().any(|b| *b != 0) {
        return None;
    }
    let (_, _, pdu) = lire(pdu)?; // index d'erreur
    let (_, liste, _) = lire(pdu)?;
    let (_, variable, _) = lire(liste)?;
    let (_, o_id, reste) = lire(variable)?;
    let (tv, v, _) = lire(reste)?;
    let valeur = match tv {
        0x02 => Valeur::Entier(v.iter().fold(if v.first().is_some_and(|b| b & 0x80 != 0) { -1i64 } else { 0 }, |a, b| (a << 8) | *b as i64)),
        0x04 => Valeur::Texte(v.to_vec()),
        _ => Valeur::Autre,
    };
    Some((id, arcs(o_id), valeur))
}

/// Parcourt une colonne de la table (GetNext) : (index de ligne, valeur).
fn colonne(socket: &UdpSocket, communaute: &str, col: u32) -> std::io::Result<Vec<(Vec<u32>, Valeur)>> {
    let prefixe: Vec<u32> = PRT_INPUT.iter().copied().chain([col]).collect();
    let mut courant = prefixe.clone();
    let mut lignes = vec![];
    let mut tampon = [0u8; 1500];
    for id in 1..=64 {
        socket.send(&message(communaute, 0xA1, id, &courant, &[0x05, 0x00]))?;
        let n = socket.recv(&mut tampon)?;
        let Some((rid, o, v)) = decoder(&tampon[..n]) else { break };
        if rid != id || !o.starts_with(&prefixe) || o.len() <= prefixe.len() {
            break;
        }
        lignes.push((o[prefixe.len()..].to_vec(), v));
        courant = o;
    }
    Ok(lignes)
}

/// Médias chargés, lus à l'adresse `hote` (port 161 par défaut), communauté `public`.
pub fn lire_medias(hote: &str, communaute: &str, delai: Duration) -> Result<Vec<Media>, String> {
    let adresse = if hote.contains(':') { hote.to_string() } else { format!("{hote}:161") };
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_read_timeout(Some(delai)).map_err(|e| e.to_string())?;
    socket.connect(&adresse).map_err(|e| format!("{adresse} : {e}"))?;
    let lire = |col| colonne(&socket, communaute, col).map_err(|e| format!("{adresse} : {e}"));
    let unites = lire(COL_UNITE)?;
    let largeurs = lire(COL_LARGEUR)?;
    let noms = lire(COL_NOM).unwrap_or_default();
    let mut medias = vec![];
    for (ligne, v) in largeurs {
        let Valeur::Entier(l) = v else { continue };
        // −1 : autre, −2 : inconnu, 0 : rien de chargé.
        if l <= 0 {
            continue;
        }
        let unite = unites.iter().find(|(x, _)| *x == ligne).map(|(_, v)| v.clone());
        let mm = match unite {
            Some(Valeur::Entier(3)) => l as f64 * 25.4 / 10_000.0,
            Some(Valeur::Entier(4)) => l as f64 / 1000.0,
            _ => continue,
        };
        let nom = noms.iter().find(|(x, _)| *x == ligne).and_then(|(_, v)| match v {
            Valeur::Texte(t) if !t.is_empty() => Some(String::from_utf8_lossy(t).trim().to_string()),
            _ => None,
        });
        medias.push(Media { entree: *ligne.last().unwrap_or(&0), largeur_mm: (mm * 10.0).round() / 10.0, nom });
    }
    Ok(medias)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ber_aller_retour() {
        assert_eq!(oid(&[1, 3, 6, 1, 2, 1, 43, 8, 2, 1, 7, 1, 1]), [0x06, 12, 0x2B, 6, 1, 2, 1, 43, 8, 2, 1, 7, 1, 1]);
        assert_eq!(entier(16150), [0x02, 2, 0x3F, 0x16]);
        assert_eq!(entier(-2), [0x02, 1, 0xFE]);
        assert_eq!(entier(200), [0x02, 2, 0x00, 0xC8]);
        let r = message("public", 0xA2, 7, &[1, 3, 6, 1, 2, 1, 43, 8, 2, 1, 7, 1, 1], &entier(640_000));
        assert_eq!(decoder(&r), Some((7, vec![1, 3, 6, 1, 2, 1, 43, 8, 2, 1, 7, 1, 1], Valeur::Entier(640_000))));
        assert_eq!(arcs(&oid(&[1, 3, 6, 1, 4, 1, 1248, 1])[2..]), [1, 3, 6, 1, 4, 1, 1248, 1], "arc > 127");
    }

    /// Agent SNMP factice : un rouleau de 1615 mm (micromètres) en entrée 1, rien en entrée 2.
    #[test]
    fn largeur_lue_sur_un_agent() {
        let agent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let adresse = agent.local_addr().unwrap().to_string();
        let table: Vec<(Vec<u32>, Vec<u8>)> = vec![
            (vec![3, 1, 1], entier(4)),
            (vec![3, 1, 2], entier(4)),
            (vec![7, 1, 1], entier(1_615_000)),
            (vec![7, 1, 2], entier(0)),
            (vec![12, 1, 1], tlv(0x04, b"Vinyle brillant")),
            (vec![12, 1, 2], tlv(0x04, b"")),
            (vec![13, 1, 1], entier(9)),
        ];
        std::thread::spawn(move || {
            let mut t = [0u8; 1500];
            while let Ok((n, de)) = agent.recv_from(&mut t) {
                // Lire la requête GetNext comme une réponse : même structure, autre type.
                let mut q = t[..n].to_vec();
                let pos = q.iter().position(|b| *b == 0xA1).unwrap();
                q[pos] = 0xA2;
                let (id, o, _) = decoder(&q).unwrap();
                let suite = table.iter().map(|(s, v)| (PRT_INPUT.iter().copied().chain(s.iter().copied()).collect::<Vec<u32>>(), v)).find(|(x, _)| x > &o);
                let r = match suite {
                    Some((x, v)) => message("public", 0xA2, id, &x, v),
                    None => message("public", 0xA2, id, &[1, 3, 6, 1, 2, 1, 44], &[0x05, 0x00]),
                };
                agent.send_to(&r, de).unwrap();
            }
        });
        let m = lire_medias(&adresse, "public", Duration::from_secs(2)).unwrap();
        assert_eq!(m, [Media { entree: 1, largeur_mm: 1615.0, nom: Some("Vinyle brillant".into()) }]);
        assert!(lire_medias("127.0.0.1:9", "public", Duration::from_millis(200)).is_err(), "pas d'agent : erreur, pas de blocage");
    }
}
