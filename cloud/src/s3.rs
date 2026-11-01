//! Client S3 minimal (OVH Object Storage ou tout service compatible) : PUT, GET et DELETE d'un
//! objet, signés en AWS Signature Version 4, adresses « path-style » (`<endpoint>/<bucket>/<clé>`).
//! L'index des objets est tenu dans SQLite (pas de ListObjects).
//!
//! Réglages saisis dans Administration (`s3_endpoint`, `s3_region`, `s3_bucket`, `s3_acces`,
//! `s3_secret`, `s3_sse`) ; le secret n'est jamais réaffiché. Appels bloquants (ureq, rustls) :
//! depuis `spawn_blocking`.

use rusqlite::Connection;
use std::io::Read;
use std::time::Duration;

/// Empreinte SHA-256 d'un corps vide.
pub const VIDE: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Clone)]
pub struct S3 {
    /// `https://s3.gra.io.cloud.ovh.net`, sans barre finale.
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub acces: String,
    secret: String,
    /// En-tête `x-amz-server-side-encryption: AES256` sur les envois.
    pub sse: bool,
}

impl std::fmt::Debug for S3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3").field("endpoint", &self.endpoint).field("region", &self.region).field("bucket", &self.bucket).field("acces", &self.acces).field("secret", &"…").field("sse", &self.sse).finish()
    }
}

/// Date AWS (`20130524T000000Z`) d'un instant UTC.
pub fn date_amz(t: u64) -> String {
    let (a, m, j) = crate::portail::civil((t / 86_400) as i64);
    let s = t % 86_400;
    format!("{a:04}{m:02}{j:02}T{:02}{:02}{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

/// Chemin canonique : chaque segment encodé (RFC 3986), les `/` gardés.
pub fn chemin_canonique(chemin: &str) -> String {
    chemin.split('/').map(crate::portail::encoder).collect::<Vec<_>>().join("/")
}

/// En-tête `Authorization` SigV4. `entetes` : (nom, valeur) à signer, `host` compris.
#[allow(clippy::too_many_arguments)]
pub fn autorisation(acces: &str, secret: &str, region: &str, service: &str, methode: &str, chemin: &str, requete: &str, entetes: &[(String, String)], empreinte_corps: &str, date: &str) -> String {
    let mut h: Vec<(String, String)> = entetes.iter().map(|(n, v)| (n.to_ascii_lowercase(), v.split_whitespace().collect::<Vec<_>>().join(" "))).collect();
    h.sort();
    let canoniques: String = h.iter().map(|(n, v)| format!("{n}:{v}\n")).collect();
    let signes = h.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(";");
    let canonique = format!("{methode}\n{chemin}\n{requete}\n{canoniques}\n{signes}\n{empreinte_corps}");
    let jour = &date[..8];
    let portee = format!("{jour}/{region}/{service}/aws4_request");
    let a_signer = format!("AWS4-HMAC-SHA256\n{date}\n{portee}\n{}", crate::sha256_octets_hex(canonique.as_bytes()));
    let k = crate::hmac_sha256(format!("AWS4{secret}").as_bytes(), jour.as_bytes());
    let k = crate::hmac_sha256(&k, region.as_bytes());
    let k = crate::hmac_sha256(&k, service.as_bytes());
    let k = crate::hmac_sha256(&k, b"aws4_request");
    let signature = crate::hex(&crate::hmac_sha256(&k, a_signer.as_bytes()));
    format!("AWS4-HMAC-SHA256 Credential={acces}/{portee},SignedHeaders={signes},Signature={signature}")
}

impl S3 {
    pub fn nouveau(endpoint: &str, region: &str, bucket: &str, acces: &str, secret: &str, sse: bool) -> S3 {
        S3 { endpoint: endpoint.trim().trim_end_matches('/').to_string(), region: region.trim().to_string(), bucket: bucket.trim().to_string(), acces: acces.trim().to_string(), secret: secret.trim().to_string(), sse }
    }

    /// Stockage des sauvegardes réglé dans Administration, sinon `None`.
    pub fn depuis(c: &Connection) -> Option<S3> {
        let v = |k: &str| crate::base::reglage(c, k).ok().flatten();
        Some(S3::nouveau(&v("s3_endpoint")?, &v("s3_region")?, &v("s3_bucket")?, &v("s3_acces")?, &v("s3_secret")?, v("s3_sse").as_deref() == Some("1")))
    }

    /// Second stockage pour la copie nocturne de la base du portail (mêmes identifiants ;
    /// autre bucket, et autre région ou adresse si renseignées).
    pub fn depuis_base(c: &Connection) -> Option<S3> {
        let principal = S3::depuis(c)?;
        let v = |k: &str| crate::base::reglage(c, k).ok().flatten();
        let bucket = v("s3_base_bucket")?;
        Some(S3::nouveau(&v("s3_base_endpoint").unwrap_or(principal.endpoint.clone()), &v("s3_base_region").unwrap_or(principal.region.clone()), &bucket, &principal.acces, &principal.secret, principal.sse))
    }

    fn hote(&self) -> String {
        let sans_schema = self.endpoint.split("://").nth(1).unwrap_or(&self.endpoint);
        let hote = sans_schema.split('/').next().unwrap_or(sans_schema);
        let https = self.endpoint.starts_with("https://");
        match hote.rsplit_once(':') {
            Some((h, "443")) if https => h.to_string(),
            Some((h, "80")) if !https => h.to_string(),
            _ => hote.to_string(),
        }
    }

    fn chemin(&self, cle: &str) -> String {
        chemin_canonique(&format!("/{}/{}", self.bucket, cle.trim_start_matches('/')))
    }

    /// En-têtes signés d'une requête : (nom, valeur), `Authorization` compris.
    fn entetes(&self, methode: &str, cle: &str, empreinte: &str, envoi: bool, t: u64) -> Vec<(String, String)> {
        let date = date_amz(t);
        let mut h = vec![("host".to_string(), self.hote()), ("x-amz-content-sha256".to_string(), empreinte.to_string()), ("x-amz-date".to_string(), date.clone())];
        if envoi && self.sse {
            h.push(("x-amz-server-side-encryption".into(), "AES256".into()));
        }
        let auth = autorisation(&self.acces, &self.secret, &self.region, "s3", methode, &self.chemin(cle), "", &h, empreinte, &date);
        h.push(("authorization".into(), auth));
        h.retain(|(n, _)| n != "host");
        h
    }

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder().http_status_as_error(false).max_redirects(0).timeout_connect(Some(Duration::from_secs(15))).timeout_recv_response(Some(Duration::from_secs(120))).build().into()
    }

    fn url(&self, cle: &str) -> String {
        format!("{}{}", self.endpoint, self.chemin(cle))
    }

    fn statut_ok(statut: u16, quoi: &str, cle: &str) -> Result<(), String> {
        if (200..300).contains(&statut) { Ok(()) } else { Err(format!("S3 : {quoi} de {cle} refusé (HTTP {statut})")) }
    }

    pub fn mettre(&self, cle: &str, octets: &[u8]) -> Result<(), String> {
        let empreinte = crate::sha256_octets_hex(octets);
        let mut q = Self::agent().put(self.url(cle)).header("Content-Type", "application/octet-stream");
        for (n, v) in self.entetes("PUT", cle, &empreinte, true, crate::maintenant()) {
            q = q.header(n, v);
        }
        let r = q.send(octets).map_err(|e| format!("S3 injoignable : {e}"))?;
        Self::statut_ok(r.status().as_u16(), "envoi", cle)
    }

    /// Objet en flux : (taille annoncée, lecteur).
    pub fn lire_flux(&self, cle: &str) -> Result<(Option<u64>, Box<dyn Read + Send>), String> {
        let mut q = Self::agent().get(self.url(cle));
        for (n, v) in self.entetes("GET", cle, VIDE, false, crate::maintenant()) {
            q = q.header(n, v);
        }
        let r = q.call().map_err(|e| format!("S3 injoignable : {e}"))?;
        Self::statut_ok(r.status().as_u16(), "lecture", cle)?;
        let taille = r.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
        Ok((taille, Box::new(r.into_body().into_with_config().limit(u64::MAX).reader())))
    }

    pub fn lire(&self, cle: &str) -> Result<Vec<u8>, String> {
        let (_, mut l) = self.lire_flux(cle)?;
        let mut v = Vec::new();
        l.read_to_end(&mut v).map_err(|e| format!("S3 : lecture de {cle} interrompue : {e}"))?;
        Ok(v)
    }

    pub fn supprimer(&self, cle: &str) -> Result<(), String> {
        let mut q = Self::agent().delete(self.url(cle));
        for (n, v) in self.entetes("DELETE", cle, VIDE, false, crate::maintenant()) {
            q = q.header(n, v);
        }
        let r = q.call().map_err(|e| format!("S3 injoignable : {e}"))?;
        // Objet déjà absent : rien à faire.
        if r.status().as_u16() == 404 { Ok(()) } else { Self::statut_ok(r.status().as_u16(), "suppression", cle) }
    }

    /// Essai d'Administration : envoi, relecture et suppression d'un petit objet.
    pub fn essayer(&self) -> Result<(), String> {
        let cle = format!("essais/{}.txt", crate::aleatoire_hex(6));
        let contenu = format!("Essai E-KO RIP Cloud {}", crate::maintenant());
        self.mettre(&cle, contenu.as_bytes())?;
        let lu = self.lire(&cle)?;
        self.supprimer(&cle)?;
        if lu == contenu.as_bytes() { Ok(()) } else { Err("S3 : l'objet relu ne correspond pas à l'objet envoyé".into()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exemple « GET Object » de la documentation AWS (Signature Version 4, en-tête Authorization).
    #[test]
    fn vecteur_de_la_documentation_aws() {
        let h = vec![
            ("Host".to_string(), "examplebucket.s3.amazonaws.com".to_string()),
            ("Range".to_string(), "bytes=0-9".to_string()),
            ("x-amz-content-sha256".to_string(), VIDE.to_string()),
            ("x-amz-date".to_string(), "20130524T000000Z".to_string()),
        ];
        let a = autorisation("AKIAIOSFODNN7EXAMPLE", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", "us-east-1", "s3", "GET", "/test.txt", "", &h, VIDE, "20130524T000000Z");
        assert_eq!(a, "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request,SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41");
    }

    #[test]
    fn dates_chemins_et_hote() {
        assert_eq!(date_amz(1_369_353_600), "20130524T000000Z");
        assert_eq!(chemin_canonique("/seau/sauvegardes/1/a b.tar"), "/seau/sauvegardes/1/a%20b.tar");
        let s = S3::nouveau("https://s3.gra.io.cloud.ovh.net:443/", "gra", "seau", "a", "s", false);
        assert_eq!(s.hote(), "s3.gra.io.cloud.ovh.net");
        assert_eq!(s.url("x/y.tar"), "https://s3.gra.io.cloud.ovh.net:443/seau/x/y.tar");
        assert_eq!(S3::nouveau("http://127.0.0.1:9000", "r", "b", "a", "s", true).hote(), "127.0.0.1:9000");
        assert!(!format!("{s:?}").contains("\"s\""), "secret absent des journaux");
        let h = S3::nouveau("http://127.0.0.1:9000", "r", "b", "a", "s", true).entetes("PUT", "k", VIDE, true, 0);
        assert!(h.iter().any(|(n, v)| n == "x-amz-server-side-encryption" && v == "AES256"));
        assert!(h.iter().any(|(n, v)| n == "authorization" && v.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-server-side-encryption")));
    }
}
