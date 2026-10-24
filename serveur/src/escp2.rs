//! Émetteur ESC/P2 de l'Epson SC-S80600 : plans tramés 2 bits (4 tailles de point) par
//! encre → flux prêt pour le port 9100. Ni tramage (item 2.3) ni envoi TCP (item 2.4) ici.
//!
//! Sources : documentation publique ESC/P2 d'Epson, structure du pilote libre Gutenprint
//! (lue, aucun code repris), et structure du flux que notre machine a reçu (capture réseau
//! de l'atelier du 24/10/2026, résumée dans `escp2_capture.txt`). Séquence :
//! préambule 0x11, sortie du mode paquet + EJL, deux blocs « remote mode », réglages
//! (`(G`, `U`, `(K`, `(i` microweave, `(e` point variable, `(U` unités, `(C/(c/(S` page,
//! `(D` résolution), puis des bandes de 16 lignes : `(v` avance, et par encre non vide
//! `($` position, `i` raster 2 bits PackBits, CR ; fin FF puis `LD`/`JE` et `JI`.

/// Encre de l'imprimante ; `code` = numéro de canal de la commande `ESC i`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encre {
    K,
    M,
    C,
    Y,
    Lk,
    Lm,
    Lc,
    /// Canal 7 : orange ou rouge, attribution à confirmer (premier essai papier).
    Or,
    /// Canal 10 : rouge ou orange, attribution à confirmer (premier essai papier).
    Rd,
}

impl Encre {
    pub fn code(self) -> u8 {
        match self {
            Encre::K => 0,
            Encre::M => 1,
            Encre::C => 2,
            Encre::Y => 4,
            Encre::Lk => 16,
            Encre::Lm => 17,
            Encre::Lc => 18,
            Encre::Or => 7,
            Encre::Rd => 10,
        }
    }
}

/// Ordre des canaux dans une bande, relevé sur la capture : C M Y K Lc Lm Lk 10 7.
pub const ORDRE: [u8; 9] = [2, 1, 4, 0, 18, 17, 16, 10, 7];
/// Lignes par bande ; l'imprimante fait elle-même ses passes (microweave).
pub const LIGNES: usize = 16;
/// Marge haute et basse de la page, en unités de page (1/360 po) : 5 mm, comme la capture.
const MARGE: u32 = 71;

/// Un plan tramé : une valeur 0..=3 par point (0 = rien, 3 = gros point), ligne par ligne.
pub struct Plan {
    pub encre: Encre,
    pub points: Vec<u8>,
}

pub struct Travail {
    /// Nom renvoyé par l'imprimante (`@BDC PS`) ; ASCII, tronqué.
    pub nom: String,
    pub poste: String,
    pub utilisateur: String,
    /// Taille de l'image, en points.
    pub largeur: usize,
    pub hauteur: usize,
    /// Résolution carrée (720 dans la capture ; les autres modes viennent avec 2.7).
    pub dpi: u32,
    /// Position de l'image sur la page, en points ; `x` multiple de 16 (alignement observé).
    pub x: u32,
    pub y: u32,
    /// Page, en points.
    pub page_largeur: u32,
    pub page_hauteur: u32,
    pub plans: Vec<Plan>,
}

/// PackBits (TIFF) : répétition `257 - n, octet` (n ≤ 128), littéral `n - 1, octets`.
pub fn packbits(src: &[u8], out: &mut Vec<u8>) {
    let mut i = 0;
    while i < src.len() {
        let mut n = 1;
        while i + n < src.len() && n < 128 && src[i + n] == src[i] {
            n += 1;
        }
        if n >= 2 {
            out.push((257 - n) as u8);
            out.push(src[i]);
            i += n;
            continue;
        }
        let debut = i;
        i += 1;
        while i < src.len() && i - debut < 128 && !(i + 2 < src.len() && src[i] == src[i + 1] && src[i] == src[i + 2]) {
            i += 1;
        }
        out.push((i - debut - 1) as u8);
        out.extend_from_slice(&src[debut..i]);
    }
}

fn cmd(f: &mut Vec<u8>, c: u8, donnees: &[u8]) {
    f.extend_from_slice(&[0x1b, b'(', c]);
    f.extend_from_slice(&(donnees.len() as u16).to_le_bytes());
    f.extend_from_slice(donnees);
}

/// Bloc « remote mode » : `ESC (R 08 00 00 REMOTE1`, sous-commandes `XX` + longueur 16 bits,
/// fin `ESC 00 00 00`.
fn remote(f: &mut Vec<u8>, sous: &[(&[u8; 2], Vec<u8>)]) {
    cmd(f, b'R', b"\0REMOTE1");
    for (nom, d) in sous {
        f.extend_from_slice(*nom);
        f.extend_from_slice(&(d.len() as u16).to_le_bytes());
        f.extend_from_slice(d);
    }
    f.extend_from_slice(&[0x1b, 0, 0, 0]);
}

/// Texte ASCII imprimable, tronqué à `n` octets ; `n` atteint par des espaces si `bourrer`.
fn ascii(s: &str, n: usize, bourrer: bool) -> Vec<u8> {
    let mut v: Vec<u8> = s.chars().map(|c| if c.is_ascii_graphic() || c == ' ' { c as u8 } else { b'_' }).take(n).collect();
    if bourrer {
        v.resize(n, b' ');
    }
    v
}

fn prefixe(zeros: usize, texte: Vec<u8>) -> Vec<u8> {
    let mut v = vec![0; zeros];
    v.extend(texte);
    v
}

fn u32s(a: u32, b: u32) -> Vec<u8> {
    [a.to_le_bytes(), b.to_le_bytes()].concat()
}

/// Construit le flux complet d'une page.
pub fn flux(t: &Travail) -> Result<Vec<u8>, String> {
    if t.dpi < 360 || !2880u32.is_multiple_of(t.dpi) {
        return Err(format!("résolution {} dpi non prise en charge", t.dpi));
    }
    if t.largeur == 0 || t.hauteur == 0 || !t.x.is_multiple_of(16) {
        return Err("image vide ou position horizontale non multiple de 16 points".into());
    }
    if t.x as usize + t.largeur > t.page_largeur as usize || t.y as usize + t.hauteur > t.page_hauteur as usize {
        return Err("l'image dépasse la page".into());
    }
    let mut plans: Vec<&Plan> = t.plans.iter().collect();
    plans.sort_by_key(|p| ORDRE.iter().position(|&c| c == p.encre.code()));
    for (i, p) in plans.iter().enumerate() {
        if p.points.len() != t.largeur * t.hauteur || p.points.iter().any(|&v| v > 3) {
            return Err(format!("plan {:?} : taille ou valeur hors 0..=3", p.encre));
        }
        if i > 0 && plans[i - 1].encre == p.encre {
            return Err(format!("encre {:?} en double", p.encre));
        }
    }

    let mut f = vec![0x11; 16384];
    f.extend_from_slice(b"\0\0\0\x1b\x01@EJL 1284.4\n@EJL     \n\x1b@");
    let logiciel = format!("E-KO RIP Center {}", env!("CARGO_PKG_VERSION"));
    remote(
        &mut f,
        &[
            (b"JS", [prefixe(3, ascii(&t.nom, 200, false)), vec![0]].concat()),
            (b"JH", prefixe(6, ascii(&t.poste, 64, false))),
            (b"JU", prefixe(3, ascii(&t.utilisateur, 64, false))),
            (b"HD", prefixe(1, [vec![1], ascii(&logiciel, 40, true)].concat())),
            (b"HD", prefixe(1, [vec![2], ascii(std::env::consts::OS, 10, true)].concat())),
            (b"HD", prefixe(1, [vec![4], ascii("", 10, true)].concat())),
        ],
    );
    // Valeurs relevées telles quelles sur la capture ; sens exact non documenté.
    remote(
        &mut f,
        &[
            (b"SN", vec![0, 8, 0]),
            (b"SN", vec![0, 0x82, 0x47]),
            (b"AC", vec![0, 0x40]),
            (b"EX", vec![0, 0, 0, 0, 0x14, 0]),
            (b"DR", vec![0, 1, 0, 0]),
        ],
    );
    f.extend_from_slice(b"\x1b@");
    cmd(&mut f, b'G', &[1]);
    f.extend_from_slice(&[0x1b, b'U', 0]);
    cmd(&mut f, b'K', &[0, 0]);
    cmd(&mut f, b'i', &[1, 0xf3, 0]);
    cmd(&mut f, b'e', &[0, 0x11]);
    let pas = (2880 / t.dpi) as u8;
    cmd(&mut f, b'U', &[8, pas, pas, 0x40, 0x0b]);
    let page = |points: u32| (points * 360).div_ceil(t.dpi);
    let longueur = page(t.page_hauteur);
    cmd(&mut f, b'C', &longueur.to_le_bytes());
    cmd(&mut f, b'c', &u32s(MARGE, MARGE + longueur));
    cmd(&mut f, b'S', &u32s(page(t.page_largeur), longueur + 2 * MARGE));
    let d = (14400 / t.dpi) as u8;
    cmd(&mut f, b'D', &[0x40, 0x38, d, d]);

    let octets = t.largeur.div_ceil(4);
    let mut ligne = vec![0u8; LIGNES * octets];
    let mut courant = 0u32;
    for y0 in (0..t.hauteur).step_by(LIGNES) {
        let mut avance_faite = false;
        for p in &plans {
            ligne.fill(0);
            let (mut gauche, mut droite) = (usize::MAX, 0);
            for r in 0..LIGNES.min(t.hauteur - y0) {
                let src = &p.points[(y0 + r) * t.largeur..(y0 + r + 1) * t.largeur];
                for (i, &v) in src.iter().enumerate().filter(|(_, v)| **v != 0) {
                    ligne[r * octets + i / 4] |= v << (6 - 2 * (i % 4));
                    gauche = gauche.min(i);
                    droite = droite.max(i / 4);
                }
            }
            if gauche == usize::MAX {
                continue;
            }
            if !avance_faite {
                let y = t.y + y0 as u32;
                cmd(&mut f, b'v', &(y - courant).to_le_bytes());
                courant = y;
                avance_faite = true;
            }
            let gauche = gauche / 16 * 16;
            let debut = gauche / 4;
            // Compressé : largeur paire (toujours le cas dans la capture) ; brut : au plus juste.
            let brut = droite + 1 - debut;
            let paire = brut.next_multiple_of(2);
            let rangee = |r: usize, n: usize| {
                let mut v = ligne[r * octets + debut..r * octets + (debut + n).min(octets)].to_vec();
                v.resize(n, 0);
                v
            };
            let mut comp = Vec::new();
            for r in 0..LIGNES {
                packbits(&rangee(r, paire), &mut comp);
            }
            let (mode, largeur_octets, donnees) = if comp.len() < brut * LIGNES {
                (1, paire, comp)
            } else {
                (0, brut, (0..LIGNES).flat_map(|r| rangee(r, brut)).collect())
            };
            cmd(&mut f, b'$', &(t.x + gauche as u32).to_le_bytes());
            f.extend_from_slice(&[0x1b, b'i', p.encre.code(), mode, 2]);
            f.extend_from_slice(&(largeur_octets as u16).to_le_bytes());
            f.extend_from_slice(&(LIGNES as u16).to_le_bytes());
            f.extend_from_slice(&donnees);
            f.push(b'\r');
        }
    }
    f.extend_from_slice(b"\x0c\x1b@");
    remote(&mut f, &[(b"LD", vec![]), (b"JE", vec![0])]);
    f.extend_from_slice(b"\x1b@\x1b@");
    remote(&mut f, &[(b"JI", vec![1, 0])]);
    f.extend_from_slice(b"\x1b@");
    Ok(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn depackbits(src: &[u8], n: usize) -> (Vec<u8>, usize) {
        let (mut out, mut p) = (Vec::new(), 0);
        while out.len() < n {
            let c = src[p] as usize;
            p += 1;
            if c < 128 {
                out.extend_from_slice(&src[p..p + c + 1]);
                p += c + 1;
            } else {
                out.extend(std::iter::repeat_n(src[p], 257 - c));
                p += 1;
            }
        }
        assert_eq!(out.len(), n, "PackBits déborde la ligne");
        (out, p)
    }

    struct Bloc {
        y: u32,
        x: u32,
        code: u8,
        octets: usize,
        donnees: Vec<u8>,
    }

    fn hex(d: &[u8]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
    }

    fn u32le(d: &[u8], i: usize) -> u32 {
        u32::from_le_bytes(d[i..i + 4].try_into().unwrap())
    }

    /// Petit décodeur : rend une ligne par commande au format de `escp2_capture.txt`
    /// (bandes résumées) et les blocs raster décompressés.
    fn decoder(d: &[u8]) -> (Vec<String>, Vec<Bloc>) {
        let (mut l, mut blocs) = (Vec::new(), Vec::new());
        let mut i = d.iter().take_while(|&&b| b == 0x11).count();
        l.push(format!("preambule 0x11 {i}"));
        let nuls = d[i..].iter().take_while(|&&b| b == 0).count();
        l.push(format!("nul {nuls}"));
        i += nuls;
        let (mut y, mut x, mut longueur) = (0u32, 0u32, 0u32);
        let mut bandes: Vec<Vec<u8>> = Vec::new();
        let (mut lignes, mut bits, mut x16) = (Vec::<u16>::new(), Vec::<u8>::new(), true);
        let ordre_vu = |b: &Vec<Vec<u8>>| {
            let o = b.iter().max_by_key(|v| v.len()).unwrap().clone();
            let rang = |c: &u8| o.iter().position(|x| x == c);
            let ok = b.iter().all(|v| v.windows(2).all(|w| rang(&w[0]) < rang(&w[1])));
            assert!(ok, "ordre des canaux incohérent entre bandes");
            o.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
        };
        while i < d.len() {
            let en_bande = !bandes.is_empty();
            let c = d[i];
            let bande = c == b'\r' || (c == 0x1b && (d[i + 1] == b'i' || (d[i + 1] == b'(' && matches!(d[i + 2], b'v' | b'$'))));
            if en_bande && !bande {
                let jn = |v: &mut Vec<u16>| {
                    v.sort();
                    v.dedup();
                    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")
                };
                let mut b16: Vec<u16> = bits.iter().map(|&b| b as u16).collect();
                l.push(format!("BANDES lignes={} bits={} x%16={} ordre={}", jn(&mut lignes), jn(&mut b16), if x16 { 0 } else { 1 }, ordre_vu(&bandes)));
                bandes.clear();
            }
            match c {
                b'\r' => i += 1,
                0x0c => {
                    l.push("FF".into());
                    i += 1;
                }
                0x1b => match d[i + 1] {
                    b'@' => {
                        l.push("ESC @".into());
                        i += 2;
                    }
                    0x01 => {
                        let a = i + 2 + d[i + 2..].iter().position(|&b| b == b'\n').unwrap();
                        let b = a + 1 + d[a + 1..].iter().position(|&b| b == b'\n').unwrap();
                        let s = |r: &[u8]| String::from_utf8_lossy(r).into_owned();
                        l.push(format!("EJL [{}] [{}]", s(&d[i + 2..a]), s(&d[a + 1..b])));
                        i = b + 1;
                    }
                    b'U' => {
                        l.push(format!("ESC U {:02x}", d[i + 2]));
                        i += 3;
                    }
                    b'i' => {
                        let (code, comp) = (d[i + 2], d[i + 3]);
                        bits.push(d[i + 4]);
                        let octets = u16::from_le_bytes([d[i + 5], d[i + 6]]) as usize;
                        let n = u16::from_le_bytes([d[i + 7], d[i + 8]]);
                        lignes.push(n);
                        let total = octets * n as usize;
                        let (donnees, lu) = if comp == 1 { depackbits(&d[i + 9..], total) } else { (d[i + 9..i + 9 + total].to_vec(), total) };
                        bandes.last_mut().unwrap().push(code);
                        x16 &= x % 16 == 0;
                        blocs.push(Bloc { y, x, code, octets, donnees });
                        i += 9 + lu;
                    }
                    b'(' => {
                        let (nom, n) = (d[i + 2], u16::from_le_bytes([d[i + 3], d[i + 4]]) as usize);
                        let a = &d[i + 5..i + 5 + n];
                        i += 5 + n;
                        match nom {
                            b'R' => {
                                assert_eq!(a, b"\0REMOTE1");
                                let mut s = String::from("REMOTE1");
                                while d[i..i + 4] != [0x1b, 0, 0, 0] {
                                    let k = String::from_utf8_lossy(&d[i..i + 2]).into_owned();
                                    let n = u16::from_le_bytes([d[i + 2], d[i + 3]]) as usize;
                                    let p = &d[i + 4..i + 4 + n];
                                    if matches!(k.as_str(), "JS" | "JH" | "JU" | "HD") {
                                        s += &format!(" {k}");
                                    } else {
                                        s += &format!(" {k}({})", hex(p));
                                    }
                                    i += 4 + n;
                                }
                                i += 4;
                                l.push(s);
                            }
                            b'v' => {
                                y += u32le(a, 0);
                                bandes.push(Vec::new());
                            }
                            b'$' => x = u32le(a, 0),
                            b'C' => {
                                longueur = u32le(a, 0);
                                l.push("ESC (C *".into());
                            }
                            b'c' => {
                                assert_eq!(u32le(a, 4), u32le(a, 0) + longueur, "(c : bas ≠ haut + (C");
                                l.push(format!("ESC (c marge={}", u32le(a, 0)));
                            }
                            b'S' => l.push(format!("ESC (S marge={}", (u32le(a, 4) - longueur) / 2)),
                            _ => l.push(format!("ESC ({} {}", nom as char, hex(a))),
                        }
                    }
                    o => panic!("commande inconnue ESC {o:02x} à {i}"),
                },
                o => panic!("octet inattendu {o:02x} à {i}"),
            }
        }
        (l, blocs)
    }

    fn attendu() -> Vec<String> {
        include_str!("escp2_capture.txt").lines().filter(|l| !l.starts_with('#')).map(String::from).collect()
    }

    /// Image synthétique : 9 encres, bords vides, une encre absente de la 1re bande,
    /// dernière bande partielle, aplats (compressés) et bruit (laissé brut).
    fn travail() -> Travail {
        let (largeur, hauteur) = (90, 40);
        let encres = [Encre::K, Encre::Rd, Encre::M, Encre::C, Encre::Lk, Encre::Y, Encre::Or, Encre::Lm, Encre::Lc];
        let mut graine = 12345u32;
        let plans = encres
            .iter()
            .enumerate()
            .map(|(n, &encre)| {
                let points = (0..largeur * hauteur)
                    .map(|i| {
                        let (px, py) = (i % largeur, i / largeur);
                        graine = graine.wrapping_mul(1103515245).wrapping_add(12345);
                        if px < 5 + n || px > 80 || (encre == Encre::Y && py < 16) {
                            0
                        } else if encre == Encre::K {
                            3
                        } else {
                            ((graine >> 16) % 4) as u8
                        }
                    })
                    .collect();
                Plan { encre, points }
            })
            .collect();
        Travail { nom: "Essai é".into(), poste: "atelier".into(), utilisateur: "mathieu".into(), largeur, hauteur, dpi: 720, x: 32, y: 560, page_largeur: 200, page_hauteur: 700, plans }
    }

    #[test]
    fn packbits_aller_retour() {
        let mut graine = 7u32;
        let bruit: Vec<u8> = (0..300)
            .map(|_| {
                graine = graine.wrapping_mul(1103515245).wrapping_add(12345);
                (graine >> 24) as u8
            })
            .collect();
        let cas: Vec<Vec<u8>> = vec![vec![], vec![5], vec![0; 129], vec![0; 300], bruit.clone(), [vec![1, 1], bruit, vec![9; 3], vec![2, 3, 3]].concat()];
        for c in cas {
            let mut z = Vec::new();
            packbits(&c, &mut z);
            assert_eq!(depackbits(&z, c.len()).0, c);
        }
        let mut z = Vec::new();
        packbits(&[0; 300], &mut z);
        assert_eq!(z, [0x81, 0, 0x81, 0, 0xd5, 0], "répétitions de 128 au plus, comme la capture");
    }

    #[test]
    fn flux_a_la_structure_de_la_capture() {
        let t = travail();
        let (lignes, _) = decoder(&flux(&t).unwrap());
        assert_eq!(lignes, attendu());
    }

    #[test]
    fn flux_redonne_les_plans() {
        let t = travail();
        let (_, blocs) = decoder(&flux(&t).unwrap());
        assert!(blocs.iter().any(|b| b.octets % 2 == 1), "au moins un bloc brut au plus juste");
        for p in &t.plans {
            let mut plan = vec![0u8; t.largeur * t.hauteur];
            for b in blocs.iter().filter(|b| b.code == p.encre.code()) {
                for r in 0..LIGNES {
                    for k in 0..b.octets * 4 {
                        let (px, py) = ((b.x - t.x) as usize + k, (b.y - t.y) as usize + r);
                        let v = (b.donnees[r * b.octets + k / 4] >> (6 - 2 * (k % 4))) & 3;
                        if px < t.largeur && py < t.hauteur {
                            plan[py * t.largeur + px] = v;
                        } else {
                            assert_eq!(v, 0, "bourrage non nul hors de l'image");
                        }
                    }
                }
            }
            assert!(plan == p.points, "plan {:?} différent après décodage", p.encre);
        }
        // Y est vide sur la première bande : elle ne doit pas y figurer.
        assert!(!blocs.iter().any(|b| b.code == 4 && b.y == t.y));
    }

    #[test]
    fn refus() {
        let mut t = travail();
        t.x = 8;
        assert!(flux(&t).is_err());
        let mut t = travail();
        t.plans[1].encre = Encre::K;
        assert!(flux(&t).is_err());
        let mut t = travail();
        t.plans[0].points[0] = 4;
        assert!(flux(&t).is_err());
    }

    /// Vérifie que `escp2_capture.txt` résume fidèlement la capture (flux TCP 9100
    /// reconstitué, jamais dans le dépôt) : `ESCP2_CAPTURE=chemin cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn fichier_de_structure_conforme_a_la_capture() {
        let d = std::fs::read(std::env::var("ESCP2_CAPTURE").unwrap()).unwrap();
        let (lignes, blocs) = decoder(&d);
        assert_eq!(lignes, attendu());
        assert_eq!(blocs.len(), 2039);
    }
}
