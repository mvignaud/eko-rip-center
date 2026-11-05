//! Langues du portail : français (langue source), anglais, espagnol, italien et allemand.
//!
//! Les pages sont écrites en français dans le code. Une couche placée devant le portail
//! traduit le HTML à la sortie, phrase par phrase, avec les catalogues `cloud/langues/<code>.json`
//! (clé : la phrase française telle qu'elle est dans le code ; valeur : sa traduction).
//!
//! Une « phrase » est le texte d'un bloc (paragraphe, bouton, cellule, libellé…), balises en
//! ligne comprises (`<a>`, `<strong>`, `<code>`…) : elles y sont notées `<0>…</0>`, `<1/>`, et
//! la traduction les replace où il faut. Les valeurs insérées par le code (`{}` d'un `format!`)
//! deviennent `{0}`, `{1}`… : la phrase de la page est reconnue malgré elles et elles sont
//! reportées telles quelles (traduites à leur tour si elles sont elles-mêmes au catalogue, dates
//! comprises). Les attributs `placeholder`, `title`, `aria-label`, `alt`, les chaînes des
//! scripts et des `confirm('…')` passent par le même catalogue. Un élément `translate="no"` (nom
//! de site, message d'un client…) n'est jamais traduit.
//!
//! Choix de la langue : cookie `langue` (sélecteur au pied des pages, route `/langue/<code>`),
//! sinon l'en-tête `Accept-Language` du navigateur (anglais pour une langue non prise en charge).
//! La langue choisie est aussi gardée sur le compte, pour ses e-mails.
//!
//! Le test `catalogues_complets` (tests/langues.rs) relève toutes les phrases du code et échoue
//! s'il en manque une dans un catalogue ; `EKO_LANGUES_ECRIRE=1` les y ajoute (vides, à traduire).

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Langue {
    Fr,
    En,
    Es,
    It,
    De,
}

pub const TOUTES: [Langue; 5] = [Langue::Fr, Langue::En, Langue::Es, Langue::It, Langue::De];
pub const COOKIE: &str = "langue";

impl Langue {
    pub fn code(self) -> &'static str {
        match self {
            Langue::Fr => "fr",
            Langue::En => "en",
            Langue::Es => "es",
            Langue::It => "it",
            Langue::De => "de",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Langue> {
        let c = code.trim().get(..2)?.to_ascii_lowercase();
        TOUTES.iter().copied().find(|l| l.code() == c)
    }

    /// Nom de la langue dans cette langue (sélecteur).
    pub fn nom(self) -> &'static str {
        match self {
            Langue::Fr => "Français",
            Langue::En => "English",
            Langue::Es => "Español",
            Langue::It => "Italiano",
            Langue::De => "Deutsch",
        }
    }
}

/// Nom du cookie : en HTTPS, préfixe `__Host-` (un site d'atelier ne peut pas le poser).
pub fn nom_cookie(https: bool) -> &'static str {
    if https { "__Host-langue" } else { COOKIE }
}

/// Langue d'une requête : cookie, sinon `Accept-Language`, sinon français.
pub fn de_la_requete(entetes: &axum::http::HeaderMap, https: bool) -> Langue {
    let nom = nom_cookie(https);
    let cookie = entetes
        .get_all(axum::http::header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|c| c.trim().split_once('='))
        .find(|(n, _)| *n == nom)
        .and_then(|(_, v)| Langue::depuis_code(v));
    if let Some(l) = cookie {
        return l;
    }
    match entetes.get(axum::http::header::ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok()) {
        Some(a) if !a.trim().is_empty() && a.trim() != "*" => du_navigateur(a),
        _ => Langue::Fr,
    }
}

/// Meilleure langue prise en charge d'un `Accept-Language` (anglais à défaut).
pub fn du_navigateur(entete: &str) -> Langue {
    let mut choix: Vec<(f32, usize, Langue)> = Vec::new();
    for (rang, partie) in entete.split(',').enumerate() {
        let mut morceaux = partie.split(';');
        let etiquette = morceaux.next().unwrap_or("").trim();
        let q = morceaux.find_map(|m| m.trim().strip_prefix("q=").and_then(|v| v.parse::<f32>().ok())).unwrap_or(1.0);
        if let Some(l) = Langue::depuis_code(etiquette)
            && q > 0.0
        {
            choix.push((q, rang, l));
        }
    }
    choix.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
    choix.first().map(|c| c.2).unwrap_or(Langue::En)
}

// ————— Catalogues —————

struct Motif {
    /// Morceaux littéraux entre les valeurs : `morceaux.len() == valeurs.len() + 1`.
    morceaux: Vec<String>,
    /// Numéro de chaque valeur, dans l'ordre de la phrase française.
    valeurs: Vec<usize>,
    longueur: usize,
    traduction: String,
}

struct Catalogue {
    exact: HashMap<String, String>,
    motifs: Vec<Motif>,
}

fn source(l: Langue) -> &'static str {
    match l {
        Langue::Fr => "{}",
        Langue::En => include_str!("../langues/en.json"),
        Langue::Es => include_str!("../langues/es.json"),
        Langue::It => include_str!("../langues/it.json"),
        Langue::De => include_str!("../langues/de.json"),
    }
}

fn catalogue(l: Langue) -> &'static Catalogue {
    static CATALOGUES: OnceLock<HashMap<Langue, Catalogue>> = OnceLock::new();
    let tous = CATALOGUES.get_or_init(|| TOUTES.iter().map(|&l| (l, construire(source(l)))).collect());
    &tous[&l]
}

fn construire(json: &str) -> Catalogue {
    let paires: HashMap<String, String> = serde_json::from_str(json).expect("catalogue de langue invalide");
    let mut exact = HashMap::new();
    let mut motifs = Vec::new();
    for (cle, trad) in paires {
        if trad.trim().is_empty() {
            continue;
        }
        let cle = normaliser(&cle);
        match decouper_motif(&cle) {
            Some((morceaux, valeurs)) => {
                let longueur = morceaux.iter().map(|m| m.len()).sum();
                motifs.push(Motif { morceaux, valeurs, longueur, traduction: trad });
            }
            None => {
                exact.insert(cle, trad);
            }
        }
    }
    // Le motif le plus précis d'abord.
    motifs.sort_by(|a, b| b.longueur.cmp(&a.longueur).then(a.morceaux.cmp(&b.morceaux)));
    Catalogue { exact, motifs }
}

/// `{0}`, `{12}`… à la position `i` : (numéro, longueur).
fn valeur_en(s: &str, i: usize) -> Option<(usize, usize)> {
    let reste = s.get(i..)?.strip_prefix('{')?;
    let fin = reste.find('}')?;
    let n = &reste[..fin];
    (!n.is_empty() && n.len() <= 2 && n.bytes().all(|b| b.is_ascii_digit())).then(|| (n.parse().unwrap_or(0), fin + 2))
}

fn decouper_motif(cle: &str) -> Option<(Vec<String>, Vec<usize>)> {
    let mut morceaux = Vec::new();
    let mut valeurs = Vec::new();
    let mut courant = String::new();
    let mut i = 0;
    while i < cle.len() {
        if let Some((n, l)) = valeur_en(cle, i) {
            morceaux.push(std::mem::take(&mut courant));
            valeurs.push(n);
            i += l;
        } else {
            let c = cle[i..].chars().next().unwrap_or(' ');
            courant.push(c);
            i += c.len_utf8();
        }
    }
    morceaux.push(courant);
    (!valeurs.is_empty()).then_some((morceaux, valeurs))
}

/// Forme de comparaison : blancs réduits à une espace, apostrophes et guillemets décodés.
pub fn normaliser(s: &str) -> String {
    let s = s.replace("&#39;", "'").replace("&#x27;", "'").replace("&quot;", "\"").replace("&nbsp;", "\u{a0}");
    let mut sortie = String::with_capacity(s.len());
    let mut blanc = false;
    for c in s.chars() {
        if c.is_whitespace() && c != '\u{a0}' && c != '\u{202f}' {
            blanc = true;
        } else {
            if blanc && !sortie.is_empty() {
                sortie.push(' ');
            }
            blanc = false;
            sortie.push(c);
        }
    }
    sortie
}

/// Traduction d'une phrase normalisée (balises notées `<0>`), ou `None`.
fn chercher(cat: &Catalogue, l: Langue, phrase: &str) -> Option<String> {
    if let Some(t) = cat.exact.get(phrase) {
        return Some(t.clone());
    }
    'motifs: for m in &cat.motifs {
        let premier = &m.morceaux[0];
        let dernier = &m.morceaux[m.morceaux.len() - 1];
        if !phrase.starts_with(premier.as_str()) || !phrase.ends_with(dernier.as_str()) || phrase.len() < premier.len() + dernier.len() {
            continue;
        }
        let mut pos = premier.len();
        let mut prises = Vec::with_capacity(m.valeurs.len());
        let fin_utile = phrase.len() - dernier.len();
        for k in 1..m.morceaux.len() {
            let morceau = &m.morceaux[k];
            let debut = if k == m.morceaux.len() - 1 {
                fin_utile
            } else if morceau.is_empty() {
                pos
            } else {
                if pos > fin_utile {
                    continue 'motifs;
                }
                match phrase[pos..fin_utile].find(morceau.as_str()) {
                    Some(d) => pos + d,
                    None => continue 'motifs,
                }
            };
            if debut < pos {
                continue 'motifs;
            }
            prises.push(&phrase[pos..debut]);
            pos = debut + morceau.len();
        }
        let mut t = m.traduction.clone();
        for (k, n) in m.valeurs.iter().enumerate() {
            let v = prises[k];
            let v = chercher(cat, l, v).unwrap_or_else(|| localiser_dates(v, l));
            t = t.replace(&format!("{{{n}}}"), &v);
        }
        return Some(t);
    }
    None
}

/// La phrase normalisée `cle` a-t-elle une traduction (directe ou par motif) ?
pub fn connue(l: Langue, cle: &str) -> bool {
    l == Langue::Fr || chercher(catalogue(l), l, cle).is_some()
}

/// Traduit un texte brut (sujet d'e-mail, ligne de texte) ; inchangé s'il n'est pas au catalogue.
pub fn texte(l: Langue, s: &str) -> String {
    if l == Langue::Fr {
        return s.to_string();
    }
    let n = normaliser(s);
    if n.is_empty() {
        return s.to_string();
    }
    let debut = &s[..s.len() - s.trim_start().len()];
    let fin = &s[s.trim_end().len()..];
    match chercher(catalogue(l), l, &n) {
        Some(t) => format!("{debut}{t}{fin}"),
        None => s.replace(n.as_str(), &localiser_dates(&n, l)),
    }
}

/// Une ligne de la version texte d'un e-mail : entière, sinon par morceaux (« Libellé : valeur »,
/// « - élément », « Bouton : »).
fn ligne(l: Langue, s: &str) -> String {
    let n = normaliser(s);
    if n.is_empty() || n == "--" || chercher(catalogue(l), l, &n).is_some() {
        return texte(l, s);
    }
    if let Some(reste) = n.strip_prefix("- ") {
        return format!("- {}", ligne(l, reste));
    }
    if let Some(debut) = n.strip_suffix(" :") {
        return format!("{}:", ligne(l, debut));
    }
    if let Some((a, b)) = n.split_once(" : ") {
        return format!("{}: {}", ligne(l, a), ligne(l, b));
    }
    texte(l, s)
}

// ————— Dates —————

const MOIS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];

fn mois(l: Langue) -> [&'static str; 12] {
    match l {
        Langue::Fr => MOIS,
        Langue::En => ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"],
        Langue::Es => ["enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre"],
        Langue::It => ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"],
        Langue::De => ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"],
    }
}

fn jours(l: Langue) -> [&'static str; 7] {
    match l {
        Langue::Fr => JOURS,
        Langue::En => ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"],
        Langue::Es => ["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"],
        Langue::It => ["lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato", "domenica"],
        Langue::De => ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"],
    }
}

/// Dates écrites par `temps` (« lundi 3 novembre 2026 à 8 h 05 », « 1er mars 2026 ») dans la langue `l`.
pub fn localiser_dates(s: &str, l: Langue) -> String {
    let unites = [" ko", " Mo", " Go", " To"];
    if l == Langue::Fr || !(MOIS.iter().any(|m| s.contains(m)) || s.contains(" j") || unites.iter().any(|u| s.contains(u))) {
        return s.to_string();
    }
    let mots: Vec<&str> = s.split(' ').collect();
    let mut sortie: Vec<String> = Vec::with_capacity(mots.len());
    let mut i = 0;
    while i < mots.len() {
        let m = mots[i];
        if let Some(k) = MOIS.iter().position(|x| *x == m) {
            if l == Langue::Es {
                sortie.push("de".into());
            }
            sortie.push(mois(l)[k].to_string());
            if l == Langue::Es && mots.get(i + 1).is_some_and(|a| a.len() == 4 && a.bytes().all(|b| b.is_ascii_digit())) {
                sortie.push("de".into());
            }
            // Allemand : « 3. November ».
            if l == Langue::De
                && let Some(prec) = sortie.len().checked_sub(2).and_then(|p| sortie.get_mut(p))
                && prec.bytes().all(|b| b.is_ascii_digit())
            {
                prec.push('.');
            }
        } else if let Some(k) = JOURS.iter().position(|x| *x == m) {
            let j = jours(l)[k];
            sortie.push(if l == Langue::Fr { j.to_string() } else if l == Langue::De { format!("{j},") } else { j.to_string() });
        } else if let Some(k) = ["ko", "Mo", "Go", "To"].iter().position(|u| *u == m).filter(|_| i > 0 && mots[i - 1].bytes().any(|b| b.is_ascii_digit())) {
            // Tailles : « 1 Go » → « 1 GB ».
            sortie.push(["KB", "MB", "GB", "TB"][k].to_string());
        } else if m == "j" && i > 0 && mots[i - 1].bytes().all(|b| b.is_ascii_digit()) {
            // Durées : « 1 j 3 h ».
            sortie.push(match l {
                Langue::It => "g",
                Langue::De => "T",
                _ => "d",
            }.to_string());
        } else if let Some(n) = m.strip_suffix("er").filter(|n| *n == "1") {
            sortie.push(n.to_string());
        } else if m == "à" && i + 3 < mots.len() && mots[i + 2] == "h" && mots[i + 1].bytes().all(|b| b.is_ascii_digit()) && mots[i + 3].len() == 2 && mots[i + 3].bytes().all(|b| b.is_ascii_digit()) {
            let (h, min) = (mots[i + 1], mots[i + 3]);
            sortie.push(match l {
                Langue::En => format!("at {h}:{min}"),
                Langue::Es => format!("a las {h}:{min}"),
                Langue::It => format!("alle {h}:{min}"),
                Langue::De => format!("um {h}:{min} Uhr"),
                Langue::Fr => format!("à {h} h {min}"),
            });
            i += 4;
            continue;
        } else {
            sortie.push(m.to_string());
        }
        i += 1;
    }
    let mut r = sortie.join(" ");
    if l == Langue::En {
        // « Monday 3 November 2026 » → « Monday, 3 November 2026 ».
        for j in jours(Langue::En) {
            r = r.replace(&format!("{j} "), &format!("{j}, "));
        }
    }
    r
}

// ————— Découpage du HTML —————

/// Balises qui restent dans une phrase.
const EN_LIGNE: [&str; 17] = ["a", "abbr", "b", "br", "code", "em", "i", "kbd", "mark", "s", "small", "span", "strong", "sub", "sup", "time", "u"];
const VIDES: [&str; 14] = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];
/// Attributs traduits.
const ATTRIBUTS: [&str; 4] = ["placeholder", "title", "aria-label", "alt"];

#[derive(Debug, Clone, PartialEq)]
pub enum Jeton {
    Texte(String),
    /// Balise brute, nom en minuscules, fermante ?
    Balise(String, String, bool),
    /// Commentaire, doctype : recopiés.
    Brut(String),
    /// Contenu d'un `<script>` ou d'un `<style>` (nom de la balise).
    Contenu(String, String),
}

fn nom_balise(brut: &str) -> (String, bool) {
    let b = brut.trim_start_matches('<');
    let (fermante, b) = match b.strip_prefix('/') {
        Some(r) => (true, r),
        None => (false, b),
    };
    (b.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>().to_ascii_lowercase(), fermante)
}

/// Découpe un HTML (ou un fragment) en jetons.
pub fn jetons(html: &str) -> Vec<Jeton> {
    let mut v = Vec::new();
    let o = html.as_bytes();
    let mut texte_debut = 0;
    let mut i = 0;
    while i < o.len() {
        if o[i] != b'<' {
            i += 1;
            continue;
        }
        let suivant = o.get(i + 1).copied().unwrap_or(b' ');
        let fin = if html[i..].starts_with("<!--") {
            html[i..].find("-->").map(|f| i + f + 3).unwrap_or(o.len())
        } else if suivant == b'!' || suivant == b'?' {
            html[i..].find('>').map(|f| i + f + 1).unwrap_or(o.len())
        } else if suivant.is_ascii_alphabetic() || (suivant == b'/' && o.get(i + 2).is_some_and(|c| c.is_ascii_alphabetic())) {
            let mut j = i + 1;
            let mut guillemet = 0u8;
            while j < o.len() {
                let c = o[j];
                if guillemet != 0 {
                    if c == guillemet {
                        guillemet = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    // Un guillemet n'ouvre une valeur qu'après `=`.
                    let avant = html[i..j].trim_end();
                    if avant.ends_with('=') {
                        guillemet = c;
                    }
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            (j + 1).min(o.len())
        } else {
            i += 1;
            continue;
        };
        if texte_debut < i {
            v.push(Jeton::Texte(html[texte_debut..i].to_string()));
        }
        let brut = &html[i..fin];
        if suivant == b'!' || suivant == b'?' {
            v.push(Jeton::Brut(brut.to_string()));
            i = fin;
        } else {
            let (nom, fermante) = nom_balise(brut);
            v.push(Jeton::Balise(brut.to_string(), nom.clone(), fermante));
            i = fin;
            if !fermante && (nom == "script" || nom == "style") {
                let fermeture = format!("</{nom}");
                let bas = html[i..].to_ascii_lowercase();
                let f = bas.find(&fermeture).map(|f| i + f).unwrap_or(o.len());
                if f > i {
                    v.push(Jeton::Contenu(html[i..f].to_string(), nom));
                }
                i = f;
            }
        }
        texte_debut = i;
    }
    if texte_debut < o.len() {
        v.push(Jeton::Texte(html[texte_debut..].to_string()));
    }
    v
}

/// Valeur d'un attribut dans une balise brute.
pub fn attribut(brut: &str, nom: &str) -> Option<String> {
    attributs(brut).into_iter().find(|(n, _, _)| n == nom).map(|(_, v, _)| v)
}

/// (nom, valeur, plage de la valeur dans `brut`) de chaque attribut entre guillemets.
fn attributs(brut: &str) -> Vec<(String, String, std::ops::Range<usize>)> {
    let o = brut.as_bytes();
    let mut v = Vec::new();
    let mut i = 1;
    // Saute le nom de la balise.
    while i < o.len() && !o[i].is_ascii_whitespace() && o[i] != b'>' {
        i += 1;
    }
    while i < o.len() {
        while i < o.len() && (o[i].is_ascii_whitespace() || o[i] == b'/') {
            i += 1;
        }
        let debut = i;
        while i < o.len() && !o[i].is_ascii_whitespace() && o[i] != b'=' && o[i] != b'>' && o[i] != b'/' {
            i += 1;
        }
        if i == debut {
            i += 1;
            continue;
        }
        let nom = brut[debut..i].to_ascii_lowercase();
        if o.get(i) == Some(&b'=') && matches!(o.get(i + 1), Some(b'"') | Some(b'\'')) {
            let g = o[i + 1];
            let vd = i + 2;
            let vf = brut[vd..].find(g as char).map(|f| vd + f).unwrap_or(o.len());
            v.push((nom, brut[vd..vf].to_string(), vd..vf));
            i = vf + 1;
        }
    }
    v
}

fn en_ligne(nom: &str) -> bool {
    EN_LIGNE.contains(&nom)
}

fn vide(nom: &str, brut: &str) -> bool {
    VIDES.contains(&nom) || brut.ends_with("/>")
}

/// Une phrase : clé normalisée (balises notées), et de quoi la reconstruire.
pub struct Phrase {
    pub cle: String,
    /// Balises en ligne par numéro de repère : (ouvrante, fermante).
    balises: Vec<(String, String)>,
    /// Jetons recopiés avant et après le cœur (blancs, balises englobantes).
    avant: String,
    apres: String,
}

/// Construit la phrase d'une suite de jetons `Texte` et de balises en ligne.
fn phrase(suite: &[Jeton]) -> Option<Phrase> {
    let blanc = |j: &Jeton| matches!(j, Jeton::Texte(t) if t.trim().is_empty());
    let mut debut = 0;
    let mut fin = suite.len();
    while debut < fin && blanc(&suite[debut]) {
        debut += 1;
    }
    while fin > debut && blanc(&suite[fin - 1]) {
        fin -= 1;
    }
    if debut == fin {
        return None;
    }
    // Retire les balises de bordure : celles qui englobent toute la phrase (`<span>…</span>`),
    // les `<br>`, et les balises orphelines (fin d'un lien précédent, début du suivant).
    let partenaire = |debut: usize, fin: usize, k: usize| -> Option<usize> {
        let Jeton::Balise(_, nom, fermante) = &suite[k] else { return None };
        let mut profondeur = 0i32;
        if *fermante {
            for i in (debut..=k).rev() {
                if let Jeton::Balise(_, n, f) = &suite[i]
                    && n == nom
                {
                    profondeur += if *f { 1 } else { -1 };
                    if profondeur == 0 {
                        return Some(i);
                    }
                }
            }
        } else {
            for (i, j) in suite.iter().enumerate().take(fin).skip(k) {
                if let Jeton::Balise(_, n, f) = j
                    && n == nom
                {
                    profondeur += if *f { -1 } else { 1 };
                    if profondeur == 0 {
                        return Some(i);
                    }
                }
            }
        }
        None
    };
    loop {
        while debut < fin && blanc(&suite[debut]) {
            debut += 1;
        }
        while fin > debut && blanc(&suite[fin - 1]) {
            fin -= 1;
        }
        if debut == fin {
            return None;
        }
        let avant = (debut, fin);
        if let Jeton::Balise(b, n, f) = &suite[debut] {
            if *f || vide(n, b) {
                debut += 1;
            } else {
                match partenaire(debut, fin, debut) {
                    None => debut += 1,
                    Some(m) if m == fin - 1 => {
                        debut += 1;
                        fin -= 1;
                    }
                    _ => {}
                }
            }
        }
        if debut < fin
            && let Jeton::Balise(b, n, f) = &suite[fin - 1]
            && (!*f || vide(n, b) || partenaire(debut, fin, fin - 1).is_none())
        {
            fin -= 1;
        }
        if (debut, fin) == avant {
            break;
        }
    }
    let mut cle = String::new();
    let mut balises: Vec<(String, String)> = Vec::new();
    let mut pile: Vec<(String, usize)> = Vec::new();
    for j in &suite[debut..fin] {
        match j {
            Jeton::Texte(t) => cle.push_str(t),
            Jeton::Balise(b, n, false) if vide(n, b) => {
                cle.push_str(&format!("<{}/>", balises.len()));
                balises.push((b.clone(), String::new()));
            }
            Jeton::Balise(b, n, false) => {
                cle.push_str(&format!("<{}>", balises.len()));
                pile.push((n.clone(), balises.len()));
                balises.push((b.clone(), String::new()));
            }
            Jeton::Balise(b, n, true) => match pile.iter().rposition(|(p, _)| p == n) {
                Some(k) => {
                    let (_, num) = pile.remove(k);
                    cle.push_str(&format!("</{num}>"));
                    balises[num].1 = b.clone();
                }
                None => {
                    cle.push_str(&format!("</{}>", balises.len()));
                    balises.push((String::new(), b.clone()));
                }
            },
            _ => {}
        }
    }
    let cle = normaliser(&cle);
    if !sans_entites(&cle).chars().any(char::is_alphabetic) {
        return None;
    }
    let brut = |js: &[Jeton]| -> String {
        js.iter()
            .map(|j| match j {
                Jeton::Texte(t) => t.as_str(),
                Jeton::Balise(b, _, _) => b.as_str(),
                _ => "",
            })
            .collect()
    };
    let mut avant = brut(&suite[..debut]);
    let mut apres = brut(&suite[fin..]);
    // Blancs en bordure du cœur : gardés hors de la phrase.
    if let Jeton::Texte(t) = &suite[debut] {
        avant.push_str(&t[..t.len() - t.trim_start().len()]);
    }
    if let Jeton::Texte(t) = &suite[fin - 1] {
        apres.insert_str(0, &t[t.trim_end().len()..]);
    }
    Some(Phrase { cle, balises, avant, apres })
}

/// Texte sans les entités HTML (`&zwnj;`…), pour savoir s'il reste des lettres.
pub fn sans_entites(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    let mut reste = s;
    while let Some(p) = reste.find('&') {
        r.push_str(&reste[..p]);
        let apres = &reste[p..];
        match apres.find(';').filter(|&f| f <= 10 && apres[1..f].bytes().all(|b| b.is_ascii_alphanumeric() || b == b'#')) {
            Some(f) => reste = &apres[f + 1..],
            None => {
                r.push('&');
                reste = &apres[1..];
            }
        }
    }
    r.push_str(reste);
    r
}

/// Remet les balises d'une phrase dans sa traduction.
fn reconstruire(p: &Phrase, traduction: &str) -> String {
    let mut s = String::with_capacity(traduction.len() + 64);
    let o = traduction.as_bytes();
    let mut i = 0;
    while i < o.len() {
        if o[i] == b'<' {
            let reste = &traduction[i + 1..];
            let (fermante, reste2) = match reste.strip_prefix('/') {
                Some(r) => (true, r),
                None => (false, reste),
            };
            let chiffres: String = reste2.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !chiffres.is_empty() {
                let apres = &reste2[chiffres.len()..];
                let num: usize = chiffres.parse().unwrap_or(usize::MAX);
                let (long, vide) = if apres.starts_with("/>") { (2, true) } else if apres.starts_with('>') { (1, false) } else { (0, false) };
                if long > 0 && num < p.balises.len() && !(fermante && vide) {
                    let (ouv, ferm) = &p.balises[num];
                    s.push_str(if fermante { ferm } else { ouv });
                    i += 1 + usize::from(fermante) + chiffres.len() + long;
                    continue;
                }
            }
        }
        let c = traduction[i..].chars().next().unwrap_or(' ');
        s.push(c);
        i += c.len_utf8();
    }
    s
}

/// Chaînes d'un script (ou d'un `onsubmit`) : (plage du contenu entre guillemets).
pub fn chaines_js(js: &str) -> Vec<std::ops::Range<usize>> {
    let o = js.as_bytes();
    let mut v = Vec::new();
    let mut i = 0;
    while i < o.len() {
        match o[i] {
            b'/' if o.get(i + 1) == Some(&b'/') => {
                while i < o.len() && o[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if o.get(i + 1) == Some(&b'*') => {
                i = js[i + 2..].find("*/").map(|f| i + 2 + f + 2).unwrap_or(o.len());
            }
            g @ (b'"' | b'\'' | b'`') => {
                let debut = i + 1;
                let mut j = debut;
                while j < o.len() && o[j] != g {
                    if o[j] == b'\\' {
                        j += 1;
                    } else if o[j] == b'\n' && g != b'`' {
                        break;
                    }
                    j += 1;
                }
                v.push(debut..j.min(o.len()));
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    v
}

fn traduire_js(cat: &Catalogue, l: Langue, js: &str, entites: bool) -> String {
    let mut s = String::with_capacity(js.len());
    let mut dernier = 0;
    for r in chaines_js(js) {
        if r.end > js.len() || r.start == 0 {
            continue;
        }
        let contenu = &js[r.clone()];
        let g = js.as_bytes()[r.start - 1];
        let cle = normaliser(&contenu.replace("\\'", "'").replace("\\\"", "\""));
        if !cle.chars().any(char::is_alphabetic) {
            continue;
        }
        if let Some(t) = chercher(cat, l, &cle) {
            let mut t = t.replace('\\', "\\\\");
            match g {
                b'\'' => t = t.replace('\'', "\\'"),
                b'"' => t = t.replace('"', "\\\""),
                _ => t = t.replace('`', "\\`"),
            }
            if entites {
                t = t.replace('&', "&amp;").replace('"', "&quot;");
            }
            s.push_str(&js[dernier..r.start]);
            s.push_str(&t);
            dernier = r.end;
        }
    }
    s.push_str(&js[dernier..]);
    s
}

fn traduire_balise(cat: &Catalogue, l: Langue, brut: &str) -> String {
    let mut s = String::with_capacity(brut.len());
    let mut dernier = 0;
    for (nom, valeur, plage) in attributs(brut) {
        let nouvelle = if ATTRIBUTS.contains(&nom.as_str()) {
            chercher(cat, l, &normaliser(&valeur)).map(|t| t.replace('"', "&quot;"))
        } else if nom.starts_with("on") {
            let t = traduire_js(cat, l, &valeur.replace("&#39;", "'"), true);
            (t != valeur.replace("&#39;", "'")).then_some(t)
        } else {
            None
        };
        if let Some(n) = nouvelle {
            s.push_str(&brut[dernier..plage.start]);
            s.push_str(&n);
            dernier = plage.end;
        }
    }
    s.push_str(&brut[dernier..]);
    s
}

/// Phrases d'un HTML : utilisées par la traduction et par le relevé des clés (tests).
pub fn parcourir(html: &str, mut phrase_trouvee: impl FnMut(&Phrase) -> Option<String>, mut balise: impl FnMut(&str) -> String, mut contenu: impl FnMut(&str, &str) -> String) -> String {
    let js = jetons(html);
    let mut sortie = String::with_capacity(html.len() + html.len() / 5);
    let mut suite: Vec<Jeton> = Vec::new();
    let vider = |suite: &mut Vec<Jeton>, sortie: &mut String, f: &mut dyn FnMut(&Phrase) -> Option<String>| {
        if suite.is_empty() {
            return;
        }
        let traduite = phrase(suite).and_then(|p| f(&p).map(|t| format!("{}{}{}", p.avant, reconstruire(&p, &t), p.apres)));
        match traduite {
            Some(t) => sortie.push_str(&t),
            None => {
                for j in suite.iter() {
                    match j {
                        Jeton::Texte(t) | Jeton::Balise(t, _, _) => sortie.push_str(t),
                        _ => {}
                    }
                }
            }
        }
        suite.clear();
    };
    let mut k = 0;
    while k < js.len() {
        let j = &js[k];
        match j {
            Jeton::Texte(_) => suite.push(j.clone()),
            Jeton::Balise(b, n, f) => {
                let b2 = balise(b);
                // `translate="no"` : l'élément et son contenu sont recopiés tels quels.
                if !*f && attribut(b, "translate").as_deref() == Some("no") {
                    vider(&mut suite, &mut sortie, &mut phrase_trouvee);
                    sortie.push_str(&b2);
                    if !vide(n, b) {
                        let mut profondeur = 1;
                        k += 1;
                        while k < js.len() && profondeur > 0 {
                            match &js[k] {
                                Jeton::Balise(bb, nn, ff) if nn == n => {
                                    profondeur += if *ff { -1 } else { 1 };
                                    sortie.push_str(bb);
                                }
                                Jeton::Texte(t) | Jeton::Balise(t, _, _) | Jeton::Brut(t) | Jeton::Contenu(t, _) => sortie.push_str(t),
                            }
                            k += 1;
                        }
                        continue;
                    }
                } else if en_ligne(n) {
                    suite.push(Jeton::Balise(b2, n.clone(), *f));
                } else {
                    vider(&mut suite, &mut sortie, &mut phrase_trouvee);
                    sortie.push_str(&b2);
                }
            }
            Jeton::Brut(t) => {
                vider(&mut suite, &mut sortie, &mut phrase_trouvee);
                sortie.push_str(t);
            }
            Jeton::Contenu(t, nom) => {
                vider(&mut suite, &mut sortie, &mut phrase_trouvee);
                sortie.push_str(&contenu(t, nom));
            }
        }
        k += 1;
    }
    vider(&mut suite, &mut sortie, &mut phrase_trouvee);
    sortie
}

/// Traduit une page (ou un fragment) HTML dans la langue `l`.
pub fn html(l: Langue, page: &str) -> String {
    if l == Langue::Fr {
        return page.to_string();
    }
    let cat = catalogue(l);
    let sortie = parcourir(page, |p| chercher(cat, l, &p.cle).or_else(|| Some(localiser_dates(&p.cle, l)).filter(|d| *d != p.cle)), |b| traduire_balise(cat, l, b), |t, nom| if nom == "script" { traduire_js(cat, l, t, false) } else { t.to_string() });
    sortie.replacen(r#"<html lang="fr""#, &format!(r#"<html lang="{}""#, l.code()), 1)
}

/// Traduit un texte sur plusieurs lignes (version texte des e-mails), ligne par ligne.
pub fn lignes(l: Langue, s: &str) -> String {
    if l == Langue::Fr {
        return s.to_string();
    }
    s.split('\n').map(|x| ligne(l, x)).collect::<Vec<_>>().join("\n")
}

/// Marque la langue courante dans le sélecteur du pied des pages.
pub fn marquer(l: Langue, page: &str) -> String {
    let cherche = format!(r#"<a href="/langue/{0}" hreflang="{0}""#, l.code());
    let page = page.replace(&cherche, &format!(r#"<a href="/langue/{0}" hreflang="{0}" aria-current="true""#, l.code()));
    // Documentation : version anglaise pour les autres langues.
    if l == Langue::Fr { page } else { page.replace(r#"href="https://e-ko.rip/documentation/cloud/""#, r#"href="https://e-ko.rip/documentation/en/cloud/""#) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(paires: &[(&str, &str)]) -> Catalogue {
        let m: HashMap<String, String> = paires.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        construire(&serde_json::to_string(&m).unwrap())
    }

    fn traduire_avec(c: &Catalogue, page: &str) -> String {
        parcourir(page, |p| chercher(c, Langue::En, &p.cle), |b| traduire_balise(c, Langue::En, b), |t, n| if n == "script" { traduire_js(c, Langue::En, t, false) } else { t.to_string() })
    }

    #[test]
    fn navigateur() {
        assert_eq!(du_navigateur("de-DE,de;q=0.9,en;q=0.8"), Langue::De);
        assert_eq!(du_navigateur("nl-NL,nl;q=0.9"), Langue::En);
        assert_eq!(du_navigateur("nl,fr;q=0.5,it;q=0.7"), Langue::It);
        assert_eq!(du_navigateur("es;q=0,fr"), Langue::Fr);
    }

    #[test]
    fn phrases_et_balises() {
        let c = cat(&[
            ("Mes sites", "My sites"),
            ("Pas encore de compte ? <0>Créer un compte</0>", "No account yet? <0>Create an account</0>"),
            ("Créer un compte", "Create an account"),
            ("Un lien vous a été envoyé à {0} ; il est valable {1}.", "A link was sent to {0}; it is valid for {1}."),
            ("1 heure", "1 hour"),
            ("Retirer ce site ?", "Remove this site?"),
            ("Votre nom", "Your name"),
        ]);
        assert_eq!(traduire_avec(&c, "<h1> Mes sites </h1>"), "<h1> My sites </h1>");
        assert_eq!(traduire_avec(&c, r#"<p class="meta">Pas encore de compte ? <a href="/inscription">Créer un compte</a></p>"#), r#"<p class="meta">No account yet? <a href="/inscription">Create an account</a></p>"#);
        assert_eq!(traduire_avec(&c, "<p>Un lien vous a été envoyé à <strong>a@b.fr</strong> ; il est valable 1 heure.</p>"), "<p>A link was sent to <strong>a@b.fr</strong>; it is valid for 1 hour.</p>");
        assert_eq!(traduire_avec(&c, r#"<form onsubmit="return confirm('Retirer ce site ?')"><input placeholder="Votre nom"></form>"#), r#"<form onsubmit="return confirm('Remove this site?')"><input placeholder="Your name"></form>"#);
        assert_eq!(traduire_avec(&c, r#"<td translate="no">Mes sites</td><td><span>Mes sites</span></td>"#), r#"<td translate="no">Mes sites</td><td><span>My sites</span></td>"#);
        assert_eq!(traduire_avec(&c, "<script>alert('Mes sites'); // l'astuce\n</script>"), "<script>alert('My sites'); // l'astuce\n</script>");
        assert_eq!(traduire_avec(&c, "<p>Inconnu</p>"), "<p>Inconnu</p>");
    }

    #[test]
    fn dates() {
        assert_eq!(localiser_dates("lundi 3 novembre 2026 à 8 h 05", Langue::En), "Monday, 3 November 2026 at 8:05");
        assert_eq!(localiser_dates("1er mars 2026", Langue::De), "1. März 2026");
        assert_eq!(localiser_dates("0 ko / 1 Go", Langue::It), "0 KB / 1 GB");
        assert_eq!(localiser_dates("3 novembre 2026", Langue::It), "3 novembre 2026");
        assert_eq!(localiser_dates("4 août 2026", Langue::Es), "4 de agosto de 2026");
    }

    #[test]
    fn catalogues_lisibles() {
        for l in TOUTES {
            let _ = catalogue(l);
        }
    }
}
