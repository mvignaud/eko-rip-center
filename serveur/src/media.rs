//! Média chargé dans l'imprimante, lu par SNMP v1 dans la Printer-MIB normalisée
//! (RFC 3805, table `prtInputTable`) : largeur du média en travers de l'avance
//! (`prtInputMediaDimXFeedDirChosen`), unité (`prtInputDimUnit`) et nom
//! (`prtInputMediaName`).
//! Validé sur la SC-S80600 le 24/10 : largeur imprimable (1592,7 mm d'un rouleau de 1600),
//! longueur restante (`prtInputCurrentLevel`, décroît pendant l'impression), alertes
//! (`prtAlertDescription`). En plus, la réponse d'état Epson « @BDC ST2 » de sa MIB
//! privée (lue par ONYX toutes les 10 s) : niveaux d'encre, n° de support, chauffages.
// ponytail: client SNMP v1 minimal (GetNext, BER à la main).

use std::net::UdpSocket;
use std::time::Duration;

const PRT_INPUT: [u32; 10] = [1, 3, 6, 1, 2, 1, 43, 8, 2, 1];
const COL_UNITE: u32 = 3;
const COL_LARGEUR: u32 = 7;
const COL_UNITE_CAPACITE: u32 = 8;
const COL_NIVEAU: u32 = 10;
const COL_NOM: u32 = 12;
/// `prtAlertDescription`.
const ALERTES: [u32; 11] = [1, 3, 6, 1, 2, 1, 43, 18, 1, 1, 8];
/// `prtAlertSeverityLevel` (RFC 3805) : 3 = critique, 4 = avertissement.
const GRAVITES: [u32; 11] = [1, 3, 6, 1, 2, 1, 43, 18, 1, 1, 2];
/// `hrPrinterStatus` (RFC 2790, Host Resources MIB) : 3 au repos, 4 imprime, 5 chauffe.
const STATUT_IMPRIMANTE: [u32; 11] = [1, 3, 6, 1, 2, 1, 25, 3, 5, 1, 1];
/// Textes d'alerte relevés sur notre Epson qui arrêtent l'impression, quelle que soit la
/// gravité annoncée (en minuscules, comparés en contenu).
const TEXTES_BLOQUANTS: [&str; 1] = ["cover open"];
/// Réponse d'état Epson (« @BDC ST2 »), MIB privée 1248.
const ETAT_EPSON: [u32; 14] = [1, 3, 6, 1, 4, 1, 1248, 1, 2, 2, 1, 1, 1, 4];

#[derive(Debug, PartialEq, Clone, serde::Serialize)]
pub struct Media {
    /// Entrée papier (rouleau) de la Printer-MIB.
    pub entree: u32,
    pub largeur_mm: f64,
    pub nom: Option<String>,
    /// Longueur restante du rouleau, en m (celle saisie au chargement, décomptée).
    pub longueur_m: Option<f64>,
}

/// État lu dans la réponse « @BDC ST2 » (format public, décodé à l'atelier le 24/10).
#[derive(Debug, PartialEq, Clone, Default, serde::Serialize)]
pub struct EtatEpson {
    /// (encre, % restant), dans l'ordre de la réponse.
    pub encres: Vec<(String, u8)>,
    /// N° du support enregistré dans l'imprimante.
    pub support: Option<u8>,
    /// Chauffages : deux valeurs en °C ; la première est celle de l'écran de l'imprimante.
    pub chauffages: Vec<(u8, u8)>,
}

/// Code couleur Epson → encre (codes confirmés sur l'écran de la S80600 le 24/10).
fn encre(code: u8) -> String {
    match code {
        0x00 => "BK",
        0x01 => "C",
        0x02 => "M",
        0x03 => "Y",
        0x04 => "LC",
        0x05 => "LM",
        0x08 => "LK",
        0x09 => "R",
        0x0D => "OR",
        0x10 => "CL",
        _ => return format!("0x{code:02X}"),
    }
    .into()
}

/// Champs [type, longueur, données] après « @BDC ST2\r\n » et une longueur sur 16 bits.
fn decoder_st2(o: &[u8]) -> Option<EtatEpson> {
    let debut = o.windows(5).position(|w| w == b"ST2\r\n")? + 5;
    let n = *o.get(debut)? as usize | (*o.get(debut + 1)? as usize) << 8;
    let (mut i, fin) = (debut + 2, (debut + 2 + n).min(o.len()));
    let mut e = EtatEpson::default();
    while i + 2 <= fin {
        let (t, l) = (o[i], o[i + 1] as usize);
        let v = o.get(i + 2..i + 2 + l)?;
        match t {
            // Encres : taille d'une entrée puis entrées (repère, code, %, capacité).
            0x0f if !v.is_empty() && v[0] >= 3 => e.encres = v[1..].chunks(v[0] as usize).filter(|c| c.len() >= 3).map(|c| (encre(c[1]), c[2])).collect(),
            0x2d if l >= 2 => e.support = Some(v[1]),
            0x2e => e.chauffages = v.chunks(3).filter(|c| c.len() == 3).map(|c| (c[1], c[2])).collect(),
            _ => {}
        }
        i += 2 + l;
    }
    Some(e)
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
    parcourir(socket, communaute, &PRT_INPUT.iter().copied().chain([col]).collect::<Vec<u32>>())
}

/// Parcourt tout ce qui suit `prefixe` (GetNext) : (suffixe, valeur).
fn parcourir(socket: &UdpSocket, communaute: &str, prefixe: &[u32]) -> std::io::Result<Vec<(Vec<u32>, Valeur)>> {
    let prefixe = prefixe.to_vec();
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

fn connecter(hote: &str, delai: Duration) -> Result<(UdpSocket, String), String> {
    let adresse = if hote.contains(':') { hote.to_string() } else { format!("{hote}:161") };
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    socket.set_read_timeout(Some(delai)).map_err(|e| e.to_string())?;
    socket.connect(&adresse).map_err(|e| format!("{adresse} : {e}"))?;
    Ok((socket, adresse))
}

/// Alertes en cours (« printer cover open », « ready »…).
pub fn lire_alertes(hote: &str, communaute: &str, delai: Duration) -> Result<Vec<String>, String> {
    let (socket, adresse) = connecter(hote, delai)?;
    let lignes = parcourir(&socket, communaute, &ALERTES).map_err(|e| format!("{adresse} : {e}"))?;
    Ok(lignes.into_iter().filter_map(|(_, v)| if let Valeur::Texte(t) = v { Some(String::from_utf8_lossy(&t).trim().to_string()) } else { None }).filter(|t| !t.is_empty()).collect())
}

/// Alertes qui empêchent d'imprimer : gravité « critique » de la Printer-MIB (RFC 3805),
/// ou texte relevé sur notre Epson (« printer cover open », 24/10). Une alerte sans gravité
/// lisible ne compte que par son texte.
pub fn lire_alertes_bloquantes(hote: &str, communaute: &str, delai: Duration) -> Result<Vec<String>, String> {
    let (socket, adresse) = connecter(hote, delai)?;
    let textes = parcourir(&socket, communaute, &ALERTES).map_err(|e| format!("{adresse} : {e}"))?;
    let gravites = parcourir(&socket, communaute, &GRAVITES).unwrap_or_default();
    Ok(textes
        .into_iter()
        .filter_map(|(ligne, v)| {
            let Valeur::Texte(t) = v else { return None };
            let t = String::from_utf8_lossy(&t).trim().to_string();
            let critique = gravites.iter().any(|(l, g)| *l == ligne && *g == Valeur::Entier(3));
            let connu = TEXTES_BLOQUANTS.iter().any(|b| t.to_lowercase().contains(b));
            (!t.is_empty() && (critique || connu)).then_some(t)
        })
        .collect())
}

/// L'imprimante imprime-t-elle ? `None` sans réponse lisible.
pub fn imprime(hote: &str, communaute: &str, delai: Duration) -> Option<bool> {
    let (socket, _) = connecter(hote, delai).ok()?;
    let statut = parcourir(&socket, communaute, &STATUT_IMPRIMANTE).ok()?.into_iter().find_map(|(_, v)| if let Valeur::Entier(n) = v { Some(n) } else { None });
    en_impression(statut, &lire_alertes(hote, communaute, delai).unwrap_or_default())
}

/// En impression : `hrPrinterStatus` = 4, ou, pendant la montée en température, statut 1
/// (« other ») avec les alertes « printing » / « heater warm up » (S80600, relevé le
/// 25/10 : 1 min 45 de chauffe avant que le statut passe à 4).
pub fn en_impression(statut: Option<i64>, alertes: &[String]) -> Option<bool> {
    let occupe = alertes.iter().any(|a| {
        let a = a.to_ascii_lowercase();
        a == "printing" || a.contains("warm up")
    });
    match statut {
        Some(4) => Some(true),
        Some(_) => Some(occupe),
        None if occupe => Some(true),
        None => None,
    }
}

/// Encres, support et chauffages d'une Epson (MIB privée) ; `None` si elle ne répond pas.
pub fn lire_etat_epson(hote: &str, communaute: &str, delai: Duration) -> Option<EtatEpson> {
    let (socket, _) = connecter(hote, delai).ok()?;
    let lignes = parcourir(&socket, communaute, &ETAT_EPSON).ok()?;
    lignes.into_iter().find_map(|(_, v)| if let Valeur::Texte(t) = v { decoder_st2(&t) } else { None })
}

/// Médias chargés, lus à l'adresse `hote` (port 161 par défaut), communauté `public`.
pub fn lire_medias(hote: &str, communaute: &str, delai: Duration) -> Result<Vec<Media>, String> {
    let (socket, adresse) = connecter(hote, delai)?;
    let lire = |col| colonne(&socket, communaute, col).map_err(|e| format!("{adresse} : {e}"));
    let unites = lire(COL_UNITE)?;
    let largeurs = lire(COL_LARGEUR)?;
    let noms = lire(COL_NOM).unwrap_or_default();
    let unites_capacite = lire(COL_UNITE_CAPACITE).unwrap_or_default();
    let niveaux = lire(COL_NIVEAU).unwrap_or_default();
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
        // Niveau : µm (unité 4) ou 1/10 000 de pouce (3) ; négatif = inconnu.
        let longueur_m = match (unites_capacite.iter().find(|(x, _)| *x == ligne).map(|(_, v)| v), niveaux.iter().find(|(x, _)| *x == ligne).map(|(_, v)| v)) {
            (Some(Valeur::Entier(4)), Some(Valeur::Entier(n))) if *n >= 0 => Some(*n as f64 / 1e6),
            (Some(Valeur::Entier(3)), Some(Valeur::Entier(n))) if *n >= 0 => Some(*n as f64 * 25.4 / 1e7),
            _ => None,
        };
        medias.push(Media { entree: *ligne.last().unwrap_or(&0), largeur_mm: (mm * 10.0).round() / 10.0, nom, longueur_m });
    }
    Ok(medias)
}

#[cfg(test)]
pub(crate) mod tests {
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

    /// Réponse « @BDC ST2 » relevée sur la S80600 le 24/10 (écran : OR 10 %, BK 93 %…,
    /// support 6, chauffages 38/38/50 °C).
    #[test]
    fn etat_epson_de_l_atelier() {
        let h = "40424443205354320D0AB9000101040C0200000D01280F2904100D0A640A08366401005D640301176404024B6405034C640C094F64060435640705296413101632130101190C0000000000756E6B6E6F776E1F0A58323753303033353433203D00787D8C857B827B6F74747887807B7D766A6F7B7E8D847E7D797179838C82767B827D7B8D8074786E747B767B85786C71797B837D7E887E767E838C822103002E0F2404000000002804080000002C02A51B2D0202062E0A0026270026260032320043011D";
        let o: Vec<u8> = (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect();
        let e = decoder_st2(&o).unwrap();
        let noms: Vec<&str> = e.encres.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(noms, ["OR", "LK", "BK", "C", "M", "Y", "R", "LC", "LM", "CL"]);
        assert_eq!((e.encres[0].1, e.encres[2].1), (10, 93));
        assert_eq!(e.support, Some(6));
        assert_eq!(e.chauffages, [(38, 39), (38, 38), (50, 50)]);
    }

    /// Agent SNMP v1 factice sur un port local : répond aux GetNext d'après `table`
    /// (OID complet, valeur BER), triée ou non. Rend son adresse.
    pub(crate) fn agent_factice(table: Vec<(Vec<u32>, Vec<u8>)>) -> String {
        agent_modifiable(std::sync::Arc::new(std::sync::Mutex::new(table)))
    }

    pub(crate) type Table = std::sync::Arc<std::sync::Mutex<Vec<(Vec<u32>, Vec<u8>)>>>;

    /// Agent factice dont la table peut changer pendant l'essai (niveaux qui baissent).
    pub(crate) fn agent_modifiable(partagee: Table) -> String {
        let agent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let adresse = agent.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let mut t = [0u8; 1500];
            while let Ok((n, de)) = agent.recv_from(&mut t) {
                let mut table = partagee.lock().unwrap().clone();
                table.sort();
                // Lire la requête GetNext comme une réponse : même structure, autre type.
                let mut q = t[..n].to_vec();
                let pos = q.iter().position(|b| *b == 0xA1).unwrap();
                q[pos] = 0xA2;
                let (id, o, _) = decoder(&q).unwrap();
                let r = match table.iter().find(|(x, _)| x > &o) {
                    Some((x, v)) => message("public", 0xA2, id, x, v),
                    None => message("public", 0xA2, id, &[1, 3, 6, 1, 2, 1, 44], &[0x05, 0x00]),
                };
                agent.send_to(&r, de).unwrap();
            }
        });
        adresse
    }

    /// Une Epson qui répond : encres `(code Epson, %)` dans la réponse « @BDC ST2 », rouleau
    /// de 1615 mm dont il reste `longueur_um` micromètres.
    pub(crate) fn epson_d_essai(encres: &[(u8, u8)], longueur_um: i64) -> Vec<(Vec<u32>, Vec<u8>)> {
        let mut champ = vec![0x0f, 1 + 3 * encres.len() as u8, 3];
        champ.extend(encres.iter().flat_map(|&(code, pc)| [0, code, pc]));
        let mut st2 = b"@BDC ST2\r\n".to_vec();
        st2.extend([champ.len() as u8, 0]);
        st2.extend(champ);
        let entree = |col: u32| PRT_INPUT.iter().copied().chain([col, 1, 1]).collect::<Vec<u32>>();
        vec![
            (ETAT_EPSON.iter().copied().chain([1]).collect(), tlv(0x04, &st2)),
            (entree(COL_UNITE), entier(4)),
            (entree(COL_LARGEUR), entier(1_615_000)),
            (entree(COL_UNITE_CAPACITE), entier(4)),
            (entree(COL_NIVEAU), entier(longueur_um)),
        ]
    }

    #[test]
    fn epson_d_essai_relue() {
        let a = agent_factice(epson_d_essai(&[(0x01, 48), (0x00, 80)], 49_426_000));
        let d = Duration::from_secs(2);
        assert_eq!(lire_etat_epson(&a, "public", d).unwrap().encres, [("C".to_string(), 48), ("BK".to_string(), 80)]);
        assert_eq!(lire_medias(&a, "public", d).unwrap()[0].longueur_m, Some(49.426));
    }

    /// Alertes en cours : capot ouvert (texte relevé), papier épuisé (gravité critique),
    /// et un avertissement qui ne bloque pas ; statut « imprime » de la Host Resources MIB.
    pub(crate) fn alertes_d_essai() -> Vec<(Vec<u32>, Vec<u8>)> {
        let a = |col: u32, i: u32| ALERTES[..10].iter().copied().chain([col, 1, i]).collect::<Vec<u32>>();
        vec![
            (a(2, 1), entier(4)),
            (a(2, 2), entier(3)),
            (a(2, 3), entier(4)),
            (a(8, 1), tlv(0x04, b"printer cover open")),
            (a(8, 2), tlv(0x04, b"media empty")),
            (a(8, 3), tlv(0x04, b"ink low")),
            (STATUT_IMPRIMANTE.iter().copied().chain([1]).collect(), entier(4)),
        ]
    }

    #[test]
    fn alertes_bloquantes_et_statut() {
        let adresse = agent_factice(alertes_d_essai());
        let d = Duration::from_secs(2);
        assert_eq!(lire_alertes_bloquantes(&adresse, "public", d).unwrap(), ["printer cover open", "media empty"]);
        assert_eq!(lire_alertes(&adresse, "public", d).unwrap().len(), 3, "l'écran garde toutes les alertes");
        assert_eq!(imprime(&adresse, "public", d), Some(true));
        let vide = agent_factice(vec![]);
        assert_eq!(lire_alertes_bloquantes(&vide, "public", d).unwrap(), Vec::<String>::new());
        assert_eq!(imprime(&vide, "public", d), None);
        // Relevé réel du 25/10 : chauffe (statut 1 + « printing » + « heater warm up »),
        // impression (4), puis prête (3 + « ready »).
        let a = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(en_impression(Some(1), &a(&["printing", "heater warm up"])), Some(true), "chauffe = en cours");
        assert_eq!(en_impression(Some(4), &a(&["printing"])), Some(true));
        assert_eq!(en_impression(Some(3), &a(&["ready"])), Some(false), "finie");
        assert_eq!(en_impression(None, &[]), None);
    }

    /// Agent SNMP factice : un rouleau de 1615 mm (micromètres) en entrée 1, rien en entrée 2.
    #[test]
    fn largeur_lue_sur_un_agent() {
        let table: Vec<(Vec<u32>, Vec<u8>)> = [
            (vec![3, 1, 1], entier(4)),
            (vec![3, 1, 2], entier(4)),
            (vec![7, 1, 1], entier(1_615_000)),
            (vec![7, 1, 2], entier(0)),
            (vec![8, 1, 1], entier(4)),
            (vec![10, 1, 1], entier(38_926_000)),
            (vec![12, 1, 1], tlv(0x04, b"Vinyle brillant")),
            (vec![12, 1, 2], tlv(0x04, b"")),
            (vec![13, 1, 1], entier(9)),
        ]
        .into_iter()
        .map(|(s, v)| (PRT_INPUT.iter().copied().chain(s).collect(), v))
        .collect();
        let adresse = agent_factice(table);
        let m = lire_medias(&adresse, "public", Duration::from_secs(2)).unwrap();
        assert_eq!(m, [Media { entree: 1, largeur_mm: 1615.0, nom: Some("Vinyle brillant".into()), longueur_m: Some(38.926) }]);
        assert!(lire_medias("127.0.0.1:9", "public", Duration::from_millis(200)).is_err(), "pas d'agent : erreur, pas de blocage");
    }
}
