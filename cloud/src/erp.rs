//! Client de l'API v1 de l'ERP « E-KO » (tenant GOLDEN PRINT) qui facture les offres du
//! cloud : tiers, abonnements, codes promo et partenaires, mandats SEPA, cycles de facturation.
//!
//! Adresse et jeton saisis dans Administration (`erp_url`, `erp_jeton`), jamais dans le dépôt.
//! Appels bloquants (ureq, rustls) : toujours depuis `spawn_blocking`. L'ERP n'envoie aucun
//! e-mail aux clients du cloud (`customer_notifications: false`) : la plate-forme s'en charge.
//!
//! Les webhooks de l'ERP (`POST /api/erp/webhooks`) sont signés :
//! `X-EKO-Signature: t=<unix>,v1=<hex>`, v1 = HMAC-SHA256(secret, "{t}.{corps brut}"), 300 s de tolérance.

use rusqlite::Connection;
use serde_json::Value;
use std::time::Duration;

/// Chemin du PDF d'une facture dans l'API de l'ERP (un seul endroit à changer).
pub const CHEMIN_PDF_FACTURE: &str = "/invoices/{code}/pdf";
/// Tolérance de l'horodatage des webhooks.
pub const TOLERANCE_WEBHOOK: u64 = 300;
/// Source annoncée à l'ERP (libellé des webhooks créés par l'API).
pub const SOURCE: &str = "E-KO RIP Cloud";
/// Événements de l'ERP écoutés par la plate-forme (un seul point d'arrivée, un seul secret).
pub const EVENEMENTS_WEBHOOK: [&str; 11] = [
    "subscription.created",
    "subscription.paused",
    "subscription.resumed",
    "subscription.cancel_scheduled",
    "subscription.cancelled",
    "cycle.billed",
    "cycle.paid",
    "cycle.failed",
    "dunning.reminder_sent",
    "dunning.exhausted",
    "sepa_mandate.signed",
];

#[derive(Clone)]
pub struct Erp {
    pub base: String,
    jeton: String,
}

impl std::fmt::Debug for Erp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Erp").field("base", &self.base).field("jeton", &"…").finish()
    }
}

/// Réponse de l'ERP : statut HTTP et corps JSON (Null s'il n'en est pas).
#[derive(Debug, Clone)]
pub struct Reponse {
    pub statut: u16,
    pub corps: Value,
}

impl Reponse {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.statut)
    }
    pub fn donnees(&self) -> &Value {
        &self.corps["data"]
    }
    /// Message lisible d'un refus (422 : `errors` ou `error`).
    pub fn erreur(&self) -> String {
        if let Some(o) = self.corps.get("errors").and_then(Value::as_object) {
            let mut v: Vec<String> = Vec::new();
            for (champ, msgs) in o {
                let m = match msgs {
                    Value::Array(a) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
                    Value::String(s) => s.clone(),
                    autre => autre.to_string(),
                };
                v.push(format!("{champ} : {m}"));
            }
            if !v.is_empty() {
                return v.join(" ; ");
            }
        }
        if let Some(e) = self.corps.get("error").and_then(Value::as_str) {
            return e.to_string();
        }
        if let Some(e) = self.corps.get("message").and_then(Value::as_str) {
            return e.to_string();
        }
        format!("réponse HTTP {}", self.statut)
    }
}

impl Erp {
    /// Client réglé dans Administration, sinon `None`.
    pub fn depuis(c: &Connection) -> Option<Erp> {
        let base = crate::base::reglage(c, "erp_url").ok().flatten()?;
        let jeton = crate::base::reglage(c, "erp_jeton").ok().flatten()?;
        Some(Erp { base: base.trim_end_matches('/').to_string(), jeton })
    }

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_global(Some(Duration::from_secs(20))).build().into()
    }

    fn url(&self, chemin: &str) -> String {
        format!("{}{chemin}", self.base)
    }

    fn lire(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Reponse, String> {
        let mut r = r.map_err(|e| format!("ERP injoignable : {e}"))?;
        let statut = r.status().as_u16();
        let texte = r.body_mut().with_config().limit(4 * 1024 * 1024).read_to_string().unwrap_or_default();
        Ok(Reponse { statut, corps: serde_json::from_str(&texte).unwrap_or(Value::Null) })
    }

    pub fn get(&self, chemin: &str) -> Result<Reponse, String> {
        Self::lire(Self::agent().get(self.url(chemin)).header("Authorization", &format!("Bearer {}", self.jeton)).header("Accept", "application/json").header("X-Integration-Source", SOURCE).call())
    }

    pub fn supprimer(&self, chemin: &str) -> Result<Reponse, String> {
        Self::lire(Self::agent().delete(self.url(chemin)).header("Authorization", &format!("Bearer {}", self.jeton)).header("Accept", "application/json").header("X-Integration-Source", SOURCE).call())
    }

    /// Requête avec corps JSON. `idempotence` : en-tête `Idempotency-Key` (créations).
    pub fn envoyer(&self, methode: &str, chemin: &str, corps: &Value, idempotence: Option<&str>) -> Result<Reponse, String> {
        let a = Self::agent();
        let url = self.url(chemin);
        let auth = format!("Bearer {}", self.jeton);
        macro_rules! avec_corps {
            ($q:expr) => {{
                let mut q = $q.header("Authorization", &auth).header("Accept", "application/json").header("X-Integration-Source", SOURCE);
                if let Some(k) = idempotence {
                    q = q.header("Idempotency-Key", k);
                }
                Self::lire(q.send_json(corps))
            }};
        }
        match methode {
            "POST" => avec_corps!(a.post(&url)),
            "PUT" => avec_corps!(a.put(&url)),
            "PATCH" => avec_corps!(a.patch(&url)),
            _ => Err(format!("méthode {methode} non prise en charge")),
        }
    }

    pub fn post(&self, chemin: &str, corps: &Value, idempotence: Option<&str>) -> Result<Reponse, String> {
        self.envoyer("POST", chemin, corps, idempotence)
    }

    /// Document binaire (PDF) : (statut, type, octets).
    pub fn octets(&self, chemin: &str) -> Result<(u16, String, Vec<u8>), String> {
        let mut r = Self::agent().get(self.url(chemin)).header("Authorization", &format!("Bearer {}", self.jeton)).header("Accept", "application/pdf").call().map_err(|e| format!("ERP injoignable : {e}"))?;
        let statut = r.status().as_u16();
        let genre = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("application/octet-stream").to_string();
        let o = r.body_mut().with_config().limit(30 * 1024 * 1024).read_to_vec().map_err(|e| format!("document illisible : {e}"))?;
        Ok((statut, genre, o))
    }
}

/// Chemin d'une ressource identifiée par un code (encodé).
pub fn chemin(modele: &str, code: &str) -> String {
    modele.replace("{code}", &crate::portail::encoder(code))
}

/// Vérifie l'en-tête `X-EKO-Signature` d'un webhook.
pub fn verifier_signature(secret: &str, corps: &[u8], entete: &str, maintenant: u64) -> Result<(), &'static str> {
    let mut t = None;
    let mut v1 = None;
    for morceau in entete.split(',') {
        if let Some((k, v)) = morceau.split_once('=') {
            match k.trim() {
                "t" => t = v.trim().parse::<u64>().ok(),
                "v1" => v1 = Some(v.trim().to_ascii_lowercase()),
                _ => {}
            }
        }
    }
    let (Some(t), Some(v1)) = (t, v1) else { return Err("signature_malformee") };
    if t.abs_diff(maintenant) > TOLERANCE_WEBHOOK {
        return Err("signature_expiree");
    }
    let mut message = format!("{t}.").into_bytes();
    message.extend_from_slice(corps);
    let attendu = crate::hex(&crate::hmac_sha256(secret.as_bytes(), &message));
    if crate::egal_constant(&attendu, &v1) { Ok(()) } else { Err("signature_invalide") }
}

/// Signe un corps comme le fait l'ERP (tests, outils).
pub fn signer(secret: &str, corps: &[u8], t: u64) -> String {
    let mut message = format!("{t}.").into_bytes();
    message.extend_from_slice(corps);
    format!("t={t},v1={}", crate::hex(&crate::hmac_sha256(secret.as_bytes(), &message)))
}

/// SIREN (9 chiffres) ou SIRET (14 chiffres) nettoyé des espaces ; rend le SIREN et le SIRET s'il y en a un.
pub fn siren_siret(saisie: &str) -> Option<(String, Option<String>)> {
    let c: String = saisie.chars().filter(|c| !c.is_whitespace() && *c != '.').collect();
    if !c.chars().all(|x| x.is_ascii_digit()) {
        return None;
    }
    match c.len() {
        9 => Some((c, None)),
        14 => Some((c[..9].to_string(), Some(c))),
        _ => None,
    }
}

/// Numéro de téléphone au format E.164 (`+33612345678`) depuis une saisie française ou internationale.
pub fn telephone_e164(saisie: &str) -> Option<String> {
    let c: String = saisie.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    let n = if let Some(r) = c.strip_prefix('+') {
        format!("+{r}")
    } else if let Some(r) = c.strip_prefix("00") {
        format!("+{r}")
    } else if c.len() == 10 && c.starts_with('0') {
        format!("+33{}", &c[1..])
    } else {
        return None;
    };
    (n.len() >= 9 && n.len() <= 16 && n[1..].chars().all(|x| x.is_ascii_digit())).then_some(n)
}

/// Pays proposés devant le numéro de téléphone : (code ISO, nom, indicatif).
pub const INDICATIFS: &[(&str, &str, &str)] = &[
    ("FR", "France", "33"),
    ("BE", "Belgique", "32"),
    ("CH", "Suisse", "41"),
    ("LU", "Luxembourg", "352"),
    ("MC", "Monaco", "377"),
    ("DE", "Allemagne", "49"),
    ("AT", "Autriche", "43"),
    ("ES", "Espagne", "34"),
    ("AD", "Andorre", "376"),
    ("PT", "Portugal", "351"),
    ("IT", "Italie", "39"),
    ("NL", "Pays-Bas", "31"),
    ("GB", "Royaume-Uni", "44"),
    ("IE", "Irlande", "353"),
    ("DK", "Danemark", "45"),
    ("SE", "Suède", "46"),
    ("NO", "Norvège", "47"),
    ("FI", "Finlande", "358"),
    ("PL", "Pologne", "48"),
    ("CZ", "Tchéquie", "420"),
    ("GR", "Grèce", "30"),
    ("RO", "Roumanie", "40"),
    ("MA", "Maroc", "212"),
    ("DZ", "Algérie", "213"),
    ("TN", "Tunisie", "216"),
    ("SN", "Sénégal", "221"),
    ("CI", "Côte d'Ivoire", "225"),
    ("US", "États-Unis", "1"),
    ("CA", "Canada", "1"),
    ("RE", "La Réunion", "262"),
    ("GP", "Guadeloupe", "590"),
    ("MQ", "Martinique", "596"),
    ("GF", "Guyane", "594"),
    ("NC", "Nouvelle-Calédonie", "687"),
    ("PF", "Polynésie française", "689"),
];

/// Drapeau emoji d'un code pays ISO (deux lettres).
pub fn drapeau(iso: &str) -> String {
    iso.chars()
        .filter_map(|c| {
            char::from_u32(0x1F1E6 + (c.to_ascii_uppercase() as u32).checked_sub('A' as u32)?)
        })
        .collect()
}

/// Numéro au format international à partir du pays choisi et de la saisie :
/// « 06 12 34 56 78 » avec FR donne +33612345678 ; une saisie qui commence par + ou 00 garde son indicatif.
pub fn telephone_avec_pays(pays: &str, saisie: &str) -> Option<String> {
    let t = saisie.trim();
    if t.starts_with('+') || t.starts_with("00") {
        return telephone_e164(t);
    }
    let indicatif = INDICATIFS
        .iter()
        .find(|(iso, _, _)| iso.eq_ignore_ascii_case(pays))
        .map_or("33", |(_, _, i)| *i);
    let chiffres: String = t.chars().filter(char::is_ascii_digit).collect();
    // Le 0 de tête du numéro national tombe (sauf en Italie, où il fait partie du numéro).
    let national = if indicatif == "39" {
        chiffres.as_str()
    } else {
        chiffres.strip_prefix('0').unwrap_or(&chiffres)
    };
    if national.is_empty()
        || t.chars()
            .any(|c| !(c.is_ascii_digit() || " .-()/".contains(c)))
    {
        return None;
    }
    telephone_e164(&format!("+{indicatif}{national}"))
}

/// Découpe un numéro international en (pays, numéro national) pour réafficher le formulaire.
pub fn pays_du_telephone(numero: &str) -> (&'static str, String) {
    if let Some(r) = numero.strip_prefix('+') {
        // L'indicatif le plus long d'abord (+352 avant +35…) ; +1 donne le Canada et les États-Unis : on garde le premier.
        let mut liste: Vec<_> = INDICATIFS.iter().collect();
        liste.sort_by_key(|(_, _, i)| std::cmp::Reverse(i.len()));
        if let Some((iso, _, i)) = liste.into_iter().find(|(_, _, i)| r.starts_with(*i)) {
            let reste = &r[i.len()..];
            return (
                iso,
                if *i == "39" {
                    reste.to_string()
                } else {
                    format!("0{reste}")
                },
            );
        }
    }
    ("FR", numero.to_string())
}

/// IBAN plausible (clé modulo 97), sans espaces, en majuscules.
pub fn iban_valide(saisie: &str) -> Option<String> {
    let i: String = saisie.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    if !(15..=34).contains(&i.len()) || !i.chars().all(|c| c.is_ascii_alphanumeric()) || !i[..2].chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let tourne = format!("{}{}", &i[4..], &i[..4]);
    let mut reste: u32 = 0;
    for c in tourne.chars() {
        let v = c.to_digit(36)?;
        for chiffre in v.to_string().chars() {
            reste = (reste * 10 + chiffre.to_digit(10)?) % 97;
        }
    }
    (reste == 1).then_some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_des_webhooks() {
        let corps = br#"{"event":"cycle.paid"}"#;
        let e = signer("secret", corps, 1000);
        assert!(verifier_signature("secret", corps, &e, 1100).is_ok());
        assert_eq!(verifier_signature("autre", corps, &e, 1100), Err("signature_invalide"));
        assert_eq!(verifier_signature("secret", b"{}", &e, 1100), Err("signature_invalide"));
        assert_eq!(verifier_signature("secret", corps, &e, 1000 + TOLERANCE_WEBHOOK + 1), Err("signature_expiree"));
        assert_eq!(verifier_signature("secret", corps, "v1=abc", 1000), Err("signature_malformee"));
        // Vecteur connu de HMAC-SHA256.
        assert_eq!(crate::hex(&crate::hmac_sha256(b"key", b"The quick brown fox jumps over the lazy dog")), "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8");
    }

    #[test]
    fn telephone_et_pays() {
        assert_eq!(
            telephone_avec_pays("FR", "06 12 34 56 78").as_deref(),
            Some("+33612345678")
        );
        assert_eq!(
            telephone_avec_pays("BE", "0470 12 34 56").as_deref(),
            Some("+32470123456")
        );
        assert_eq!(
            telephone_avec_pays("IT", "06 1234 5678").as_deref(),
            Some("+390612345678")
        );
        assert_eq!(
            telephone_avec_pays("CH", "+33 6 12 34 56 78").as_deref(),
            Some("+33612345678")
        );
        assert_eq!(telephone_avec_pays("FR", "06 12 abc"), None);
        assert_eq!(telephone_avec_pays("FR", ""), None);
        assert_eq!(
            pays_du_telephone("+33612345678"),
            ("FR", "0612345678".to_string())
        );
        assert_eq!(
            pays_du_telephone("+352621123456"),
            ("LU", "0621123456".to_string())
        );
        assert_eq!(
            pays_du_telephone("+390612345678"),
            ("IT", "0612345678".to_string())
        );
        assert_eq!(drapeau("FR"), "\u{1F1EB}\u{1F1F7}");
    }

    #[test]
    fn saisies() {
        assert_eq!(siren_siret("939 018 024"), Some(("939018024".into(), None)));
        assert_eq!(siren_siret("93901802400017"), Some(("939018024".into(), Some("93901802400017".into()))));
        assert_eq!(siren_siret("12345"), None);
        assert_eq!(telephone_e164("06 12 34 56 78").as_deref(), Some("+33612345678"));
        assert_eq!(telephone_e164("+32 470 12 34 56").as_deref(), Some("+32470123456"));
        assert_eq!(telephone_e164("12"), None);
        assert_eq!(iban_valide("FR76 3000 6000 0112 3456 7890 189").as_deref(), Some("FR7630006000011234567890189"));
        assert_eq!(iban_valide("FR76 3000 6000 0112 3456 7890 188"), None);
        assert_eq!(chemin(CHEMIN_PDF_FACTURE, "FA 1"), "/invoices/FA%201/pdf");
    }
}
