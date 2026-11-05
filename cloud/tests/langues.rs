//! Catalogues de langues du portail : chaque phrase française du code a sa traduction dans
//! `cloud/langues/<code>.json`, avec les mêmes repères de balises (`<0>`) et de valeurs (`{0}`).
//!
//! `EKO_LANGUES_ECRIRE=1 cargo test -p eko-rip-cloud --test langues` ajoute les phrases
//! nouvelles (vides, à traduire) et retire celles qui ne sont plus dans le code.

use eko_rip_cloud::langues::{self, Langue};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Fichiers sans texte affiché (stockage, protocole, ligne de commande, rapports pour l'équipe).
const SANS_TEXTE: [&str; 14] = ["rapports.rs", "base.rs", "client.rs", "lib.rs", "main.rs", "montant.rs", "s3.rs", "sites.rs", "temps.rs", "webpush.rs", "ws.rs", "appairage.rs", "installations.rs", "langues.rs"];

/// Chaînes littérales d'un fichier Rust (hors tests et lignes de journal).
fn litteraux(code: &str) -> Vec<String> {
    let code = code.split("#[cfg(test)]").next().unwrap_or(code);
    let o = code.as_bytes();
    let mut v = Vec::new();
    let mut i = 0;
    let debut_ligne = |i: usize| code[..i].rfind('\n').map_or(0, |p| p + 1);
    while i < o.len() {
        let c = o[i];
        if c == b'/' && o.get(i + 1) == Some(&b'/') {
            while i < o.len() && o[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && o.get(i + 1) == Some(&b'*') {
            i = code[i + 2..].find("*/").map_or(o.len(), |f| i + 4 + f);
            continue;
        }
        if c == b'\'' {
            // Caractère (`'x'`, `'\''`) ou durée de vie (`'a`).
            if o.get(i + 1) == Some(&b'\\') {
                i = code[i + 2..].find('\'').map_or(o.len(), |f| i + 3 + f);
            } else {
                let ch = code[i + 1..].chars().next().map_or(1, char::len_utf8);
                i += if o.get(i + 1 + ch) == Some(&b'\'') { ch + 2 } else { 1 };
            }
            continue;
        }
        let ident_avant = i > 0 && (o[i - 1].is_ascii_alphanumeric() || o[i - 1] == b'_');
        let (brute, diezes, debut) = if c == b'r' && !ident_avant && matches!(o.get(i + 1), Some(b'#') | Some(b'"')) {
            let mut j = i + 1;
            while o.get(j) == Some(&b'#') {
                j += 1;
            }
            if o.get(j) != Some(&b'"') {
                i += 1;
                continue;
            }
            (true, j - i - 1, j + 1)
        } else if c == b'"' {
            (false, 0, i + 1)
        } else if c == b'b' && !ident_avant && o.get(i + 1) == Some(&b'"') {
            // Chaîne d'octets : sautée.
            let mut j = i + 2;
            while j < o.len() && o[j] != b'"' {
                j += if o[j] == b'\\' { 2 } else { 1 };
            }
            i = j + 1;
            continue;
        } else {
            i += 1;
            continue;
        };
        let ligne = &code[debut_ligne(i)..i];
        let (texte, fin) = if brute {
            let fermeture = format!("\"{}", "#".repeat(diezes));
            let f = code[debut..].find(&fermeture).map_or(o.len(), |f| debut + f);
            (code[debut..f].to_string(), f + fermeture.len())
        } else {
            let mut s = String::new();
            let mut j = debut;
            while j < o.len() && o[j] != b'"' {
                if o[j] == b'\\' {
                    match o.get(j + 1) {
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'\n') => {
                            j += 2;
                            while j < o.len() && o[j].is_ascii_whitespace() {
                                j += 1;
                            }
                            continue;
                        }
                        Some(b'u') => {
                            let f = code[j..].find('}').map_or(j + 2, |f| j + f);
                            if let Some(ch) = u32::from_str_radix(code[j + 3..f].trim_matches(|c| c == '{' || c == '}'), 16).ok().and_then(char::from_u32) {
                                s.push(ch);
                            }
                            j = f + 1;
                            continue;
                        }
                        Some(&x) => s.push(x as char),
                        None => {}
                    }
                    j += 2;
                } else {
                    let ch = code[j..].chars().next().unwrap_or(' ');
                    s.push(ch);
                    j += ch.len_utf8();
                }
            }
            (s, j + 1)
        };
        i = fin;
        let journal = ["eprintln!(", "println!(", "panic!(", ".expect(", "unreachable!(", "assert", "#[", "debug!(", "log::"].iter().any(|m| ligne.contains(m));
        if !journal {
            v.push(texte);
        }
    }
    v
}

/// Remplace les valeurs d'un `format!` (`{}`, `{nom}`, `{:02}`) par `{0}`, `{1}`… ; `{{` → `{`.
fn valeurs_numerotees(s: &str) -> String {
    let mut r = String::with_capacity(s.len());
    let o = s.as_bytes();
    let mut i = 0;
    while i < o.len() {
        if s[i..].starts_with("{{") {
            r.push('{');
            i += 2;
        } else if s[i..].starts_with("}}") {
            r.push('}');
            i += 2;
        } else if o[i] == b'{'
            && let Some(f) = s[i + 1..].find('}')
            && s[i + 1..i + 1 + f].bytes().all(|b| b.is_ascii_alphanumeric() || b"_:?.#<>$".contains(&b))
        {
            r.push_str("{#}");
            i += f + 2;
        } else {
            let ch = s[i..].chars().next().unwrap_or(' ');
            r.push(ch);
            i += ch.len_utf8();
        }
    }
    r
}

/// Renumérote `{#}` (ou des `{n}` désordonnés) dans l'ordre d'apparition.
fn renumeroter(cle: &str) -> String {
    let mut r = String::with_capacity(cle.len());
    let mut n = 0;
    let mut reste = cle;
    while let Some(p) = reste.find("{#}") {
        r.push_str(&reste[..p]);
        r.push_str(&format!("{{{n}}}"));
        n += 1;
        reste = &reste[p + 3..];
    }
    r.push_str(reste);
    r
}

/// Une phrase qui vaut d'être traduite ?
fn a_traduire(cle: &str) -> bool {
    let sans: String = {
        // Texte hors repères et valeurs.
        let mut s = String::new();
        let mut dans = false;
        for c in cle.chars() {
            match c {
                '<' | '{' => dans = true,
                '>' | '}' => dans = false,
                _ if !dans => s.push(c),
                _ => {}
            }
        }
        s
    };
    let sans = langues::sans_entites(&sans);
    // Unités seules (« 15 min », « 2 h 05 ») : rien à traduire.
    let sans: String = sans.split(' ').filter(|m| !["min", "h", "j", "ko", "Mo", "Go", "%"].contains(m)).collect::<Vec<_>>().join(" ");
    let lettres = sans.chars().filter(|c| c.is_alphabetic()).count();
    if lettres < 2 {
        return false;
    }
    let t = sans.trim();
    if !t.contains(' ') {
        // Un seul mot : identifiant, chemin ou code, sauf s'il commence par une majuscule ou porte un accent.
        if t.chars().any(|c| "/_=@#?&%\\".contains(c)) || t.contains("://") {
            return false;
        }
        let premiere = t.chars().find(|c| c.is_alphabetic()).unwrap_or('a');
        // Mots seuls affichés tels quels (valeurs de tableau, états).
        const MOTS: [&str; 33] = ["oui", "non", "aucun", "aucune", "jamais", "inconnu", "inconnue", "indisponible", "introuvable", "normale", "haute", "lecture", "suspendu", "interrompu", "utilisés", "récente", "alerte", "coupure", "coupures", "occupation", "mensuel", "annuel", "mois", "gratuit", "active", "valide", "vide", "identique", "libre", "technique", "commercial", "bloquante", "urgence"];
        if MOTS.contains(&t) {
            return true;
        }
        if (!premiere.is_uppercase() && t.is_ascii()) || (!cle.contains('{') && t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')) {
            return false;
        }
    }
    let sql = ["SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE ", "ALTER ", "PRAGMA ", "WITH ", "BEGIN", "COMMIT"];
    !sql.iter().any(|m| t.to_ascii_uppercase().starts_with(m))
        && !t.starts_with("http")
        && !(t.contains(';') && t.contains('{') && t.contains(':'))
        && !["default-src", "'use strict'", "--", "; ", "class=", "attachment;", "inline;"].iter().any(|m| t.starts_with(m))
        && !t.contains("filename=")
        && !(t.contains(", ") && t.bytes().all(|b| b.is_ascii_lowercase() || b"_, ".contains(&b)))
}

fn chaine_js_a_traduire(s: &str) -> bool {
    s.chars().any(char::is_alphabetic) && (s.contains(' ') || !s.is_ascii()) && !s.contains("://") && !s.starts_with('/') && !s.contains("${") && !s.contains(" => ") && !s.contains("function")
}

/// Toutes les phrases du code du portail.
fn phrases() -> BTreeSet<String> {
    let dossier = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut cles = BTreeSet::new();
    let mut fichiers: Vec<_> = std::fs::read_dir(&dossier).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    fichiers.sort();
    for f in fichiers {
        let nom = f.file_name().unwrap().to_string_lossy().to_string();
        if SANS_TEXTE.contains(&nom.as_str()) || !nom.ends_with(".rs") {
            continue;
        }
        let code = std::fs::read_to_string(&f).unwrap();
        for lit in litteraux(&code) {
            // Feuille de style ou script seuls : pas de texte.
            if !lit.contains('<') && (lit.contains(";\n") || lit.contains("{\n")) {
                continue;
            }
            let html = valeurs_numerotees(&lit);
            let mut trouvees: Vec<String> = Vec::new();
            let mut attributs: Vec<String> = Vec::new();
            let js: std::cell::RefCell<Vec<String>> = Default::default();
            langues::parcourir(
                &html,
                |p| {
                    trouvees.push(p.cle.clone());
                    None
                },
                |b| {
                    for nom in ["placeholder", "title", "aria-label", "alt"] {
                        if let Some(v) = langues::attribut(b, nom) {
                            attributs.push(langues::normaliser(&v));
                        }
                    }
                    for (nom, v) in ["onsubmit", "onclick", "onchange"].iter().filter_map(|n| langues::attribut(b, n).map(|v| (*n, v))) {
                        let _ = nom;
                        let v = v.replace("&#39;", "'");
                        for r in langues::chaines_js(&v) {
                            js.borrow_mut().push(v[r].to_string());
                        }
                    }
                    b.to_string()
                },
                |t, nom| {
                    if nom == "script" {
                        for r in langues::chaines_js(t) {
                            js.borrow_mut().push(t[r].to_string());
                        }
                    }
                    t.to_string()
                },
            );
            // Version texte des e-mails : chaque ligne compte aussi.
            let lignes: Vec<String> = if html.contains('\n') && !html.contains('<') { html.lines().map(langues::normaliser).collect() } else { Vec::new() };
            for c in trouvees.into_iter().chain(attributs).chain(lignes) {
                let c = renumeroter(&c);
                if a_traduire(&c) {
                    cles.insert(c);
                }
            }
            for s in js.into_inner() {
                let s = langues::normaliser(&s.replace("\\'", "'").replace("\\\"", "\""));
                if chaine_js_a_traduire(&s) && !s.contains("{#}") {
                    cles.insert(s);
                }
            }
        }
    }
    cles
}

/// Repères `<0>`, `</0>`, `<0/>` et valeurs `{0}` d'une phrase, triés.
fn reperes(s: &str) -> Vec<String> {
    let mut v = Vec::new();
    let o = s.as_bytes();
    for (i, &c) in o.iter().enumerate() {
        let (ouvre, ferme) = match c {
            b'<' => ('<', '>'),
            b'{' => ('{', '}'),
            _ => continue,
        };
        if let Some(f) = s[i + 1..].find(ferme) {
            let dedans = &s[i + 1..i + 1 + f];
            let chiffres = dedans.trim_start_matches('/').trim_end_matches('/');
            if !chiffres.is_empty() && chiffres.len() <= 2 && chiffres.bytes().all(|b| b.is_ascii_digit()) {
                v.push(format!("{ouvre}{dedans}{ferme}"));
            }
        }
    }
    v.sort();
    v
}

fn chemin(l: Langue) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("langues").join(format!("{}.json", l.code()))
}

#[test]
fn catalogues_complets() {
    let cles = phrases();
    assert!(cles.len() > 200, "relevé des phrases suspect : {}", cles.len());
    let ecrire = std::env::var("EKO_LANGUES_ECRIRE").is_ok();
    let mut erreurs = Vec::new();
    for l in langues::TOUTES.into_iter().filter(|l| *l != Langue::Fr) {
        let existant: BTreeMap<String, String> = serde_json::from_str(&std::fs::read_to_string(chemin(l)).unwrap()).unwrap();
        if ecrire {
            let nouveau: BTreeMap<String, String> = cles.iter().map(|c| (c.clone(), existant.get(c).cloned().unwrap_or_default())).collect();
            std::fs::write(chemin(l), serde_json::to_string_pretty(&nouveau).unwrap() + "\n").unwrap();
            continue;
        }
        for c in &cles {
            match existant.get(c) {
                None => erreurs.push(format!("{} : phrase absente : {c}", l.code())),
                Some(t) if t.trim().is_empty() => erreurs.push(format!("{} : phrase non traduite : {c}", l.code())),
                Some(t) if reperes(t) != reperes(c) => erreurs.push(format!("{} : repères différents : {c} → {t}", l.code())),
                _ => {}
            }
        }
        for c in existant.keys().filter(|c| !cles.contains(*c)) {
            erreurs.push(format!("{} : phrase qui n'est plus dans le code : {c}", l.code()));
        }
    }
    let total = erreurs.len();
    erreurs.truncate(40);
    assert!(total == 0, "{total} écarts dans les catalogues (EKO_LANGUES_ECRIRE=1 pour les mettre à jour) :\n{}", erreurs.join("\n"));
}

// ————— Pages et e-mails rendus : chaque phrase est reconnue —————

/// Phrases d'une page rendue qui n'ont pas de traduction en `l` (hors noms propres et données).
fn inconnues(l: Langue, html: &str) -> Vec<String> {
    let mut v = Vec::new();
    langues::parcourir(
        html,
        |p| {
            if !langues::connue(l, &p.cle) && langues::localiser_dates(&p.cle, Langue::En) == p.cle && a_traduire(&p.cle) && !DONNEES.iter().any(|d| p.cle.contains(d)) {
                v.push(p.cle.clone());
            }
            None
        },
        |b| b.to_string(),
        |t, _| t.to_string(),
    );
    v
}

/// Textes des pages d'essai qui ne sont pas des phrases de l'interface.
const DONNEES: [&str; 6] = ["E-KO RIP Cloud", "Atelier Dupont", "chef@atelier.fr", "GOLDEN PRINT", "plateforme.test", "Atelier Essai"];

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(std::time::Duration::from_secs(20))).build().into()
}

#[tokio::test(flavor = "multi_thread")]
async fn pages_et_courriels_traduits() {
    const URL: &str = "http://plateforme.test";
    let donnees = std::env::temp_dir().join(format!("eko-cloud-langues-{}", eko_rip_cloud::aleatoire_hex(6)));
    let r = eko_rip_cloud::relais::nouveau(eko_rip_cloud::relais::Config { donnees: donnees.clone(), url_publique: URL.into(), max_octets: 1 << 20, jeton_stats: String::new() });
    let _boite = r.utiliser_memoire();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(ecoute, eko_rip_cloud::relais::routes(r)).await.unwrap() });

    let manques = tokio::task::spawn_blocking(move || {
        // Compte super-administrateur, inscrit en anglais.
        let mut rep = agent()
            .post(format!("{base}/inscription"))
            .header("Origin", URL)
            .header("Accept-Language", "en-GB,en;q=0.9")
            .send_form([("email", "chef@atelier.fr"), ("nom", "Atelier Essai"), ("mot_de_passe", "phrase-longue-1"), ("cgu", "1")])
            .unwrap();
        assert_eq!(rep.status().as_u16(), 303);
        let session = rep.headers().get_all("set-cookie").iter().map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string()).find(|c| c.starts_with("plateforme_session=")).unwrap();
        let _ = rep.body_mut().read_to_string();
        {
            let c = eko_rip_cloud::base::ouvrir(&donnees).unwrap();
            assert!(eko_rip_cloud::base::nommer_super_admin(&c, "chef@atelier.fr").unwrap());
        }
        let publiques = ["/", "/inscription", "/connexion", "/mot-de-passe/oublie", "/abonnement/offres", "/cgu", "/mentions-legales", "/email/confirmer?jeton=x", "/mot-de-passe/nouveau?jeton=x"];
        let privees = ["/sites", "/sites?ok=rattache", "/compte", "/compte?ok=preferences", "/abonnement", "/abonnement/souscrire", "/support", "/support/nouvelle", "/admin", "/admin?ok=courriels"];
        let mut manques: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut pages = Vec::new();
        for (chemin, connecte) in publiques.iter().map(|c| (c, false)).chain(privees.iter().map(|c| (c, true))) {
            // Page en français (langue source) : chaque phrase doit avoir sa traduction.
            let cookie = if connecte { format!("{session}; langue=fr") } else { "langue=fr".to_string() };
            let mut rep = agent().get(format!("{base}{chemin}")).header("Cookie", &cookie).call().unwrap();
            pages.push((chemin.to_string(), rep.body_mut().read_to_string().unwrap()));
            // Et dans chaque langue : en-têtes et `<html lang>`.
            for l in langues::TOUTES {
                let cookie = if connecte { format!("{session}; langue={}", l.code()) } else { format!("langue={}", l.code()) };
                let mut rep = agent().get(format!("{base}{chemin}")).header("Cookie", &cookie).call().unwrap();
                let statut = rep.status().as_u16();
                let langue = rep.headers().get("content-language").map(|v| v.to_str().unwrap().to_string());
                let corps = rep.body_mut().read_to_string().unwrap();
                if statut == 200 {
                    assert_eq!(langue.as_deref(), Some(l.code()), "{chemin}");
                    assert!(corps.contains(&format!(r#"<html lang="{}""#, l.code())), "{chemin} : <html lang>");
                    assert!(corps.contains(&format!(r#"hreflang="{}" aria-current="true""#, l.code())), "{chemin} : langue marquée");
                }
            }
        }
        for (nom, c) in eko_rip_cloud::courriels::apercus(URL) {
            pages.push((format!("e-mail {nom}"), c.html.clone()));
            let lignes: Vec<String> = c.texte.lines().chain([c.sujet.as_str()]).map(langues::normaliser).collect();
            for l in langues::TOUTES.into_iter().filter(|l| *l != Langue::Fr) {
                for t in lignes.iter().filter(|t| !langues::connue(l, t) && langues::lignes(l, t) == **t && a_traduire(t) && !DONNEES.iter().any(|d| t.contains(d)) && !t.starts_with("http") && t.as_str() != "--") {
                    manques.entry(format!("{} e-mail (texte) {nom}", l.code())).or_default().insert(t.clone());
                }
            }
        }
        for (ou, html) in &pages {
            for l in langues::TOUTES.into_iter().filter(|l| *l != Langue::Fr) {
                for p in inconnues(l, html) {
                    manques.entry(format!("{} {ou}", l.code())).or_default().insert(p);
                }
            }
        }
        manques
    })
    .await
    .unwrap();
    let liste: Vec<String> = manques.iter().flat_map(|(ou, ps)| ps.iter().map(move |p| format!("{ou} : {p}"))).collect();
    let total = liste.len();
    if let Ok(f) = std::env::var("EKO_LANGUES_RAPPORT") {
        std::fs::write(f, liste.join("\n")).unwrap();
    }
    assert!(total == 0, "{total} phrases affichées sans traduction :\n{}", liste.into_iter().take(80).collect::<Vec<_>>().join("\n"));
}
