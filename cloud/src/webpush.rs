//! Web Push (RFC 8030) : chiffrement des messages en `aes128gcm` (RFC 8291) et identification
//! de la plate-forme par VAPID (RFC 8292, jeton JWT signé en ES256). Tout en Rust (RustCrypto :
//! p256, hkdf, aes-gcm, sha2), sans OpenSSL : le binaire se construit en musl.
//!
//! Ce module ne fait aucun accès réseau : `chiffrer` et `autorisation_vapid` sont pures, l'envoi
//! HTTP est dans `notifications`.

use crate::{base64url, base64url_decoder};
use aes_gcm::aead::{Aead, KeyInit};
use hkdf::Hkdf;
use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

/// Taille de l'enregistrement chiffré annoncée dans l'en-tête du message.
const TAILLE_ENREGISTREMENT: u32 = 4096;
/// Contenu maximal d'un message : un seul enregistrement (RFC 8291 : 4096 - 86 d'en-tête - 17).
pub const CHARGE_MAX: usize = 3993;
/// Durée de validité du jeton VAPID (le maximum permis est 24 h).
pub const VALIDITE_JWT: u64 = 12 * 3600;
pub const SUJET_VAPID: &str = "mailto:contact@e-ko.rip";

/// Clé privée P-256 tirée du générateur du système.
pub fn nouvelle_cle() -> SecretKey {
    loop {
        let mut b = [0u8; 32];
        getrandom::fill(&mut b).expect("générateur aléatoire");
        if let Ok(k) = SecretKey::from_slice(&b) {
            return k;
        }
    }
}

/// Point public non compressé (65 octets).
pub fn public_sec1(k: &SecretKey) -> Vec<u8> {
    k.public_key().to_sec1_bytes().to_vec()
}

/// Décode du base64 « URL » avec ou sans remplissage (les navigateurs n'en mettent pas).
pub fn decoder(texte: &str) -> Option<Vec<u8>> {
    base64url_decoder(texte.trim_end_matches('='))
}

/// Chiffre `charge` pour le navigateur (clé publique `ua_public`, secret d'authentification
/// `auth`) avec la clé éphémère `as_cle` et le sel `sel`. Rend le corps de la requête :
/// sel (16) + taille d'enregistrement (4) + longueur de l'identifiant (1) + clé publique
/// de la plate-forme (65) + contenu chiffré.
pub fn chiffrer_avec(ua_public: &[u8], auth: &[u8], charge: &[u8], as_cle: &SecretKey, sel: [u8; 16]) -> Result<Vec<u8>, String> {
    if charge.len() > CHARGE_MAX {
        return Err("message trop long".into());
    }
    let ua = PublicKey::from_sec1_bytes(ua_public).map_err(|_| "clé publique du navigateur invalide")?;
    let as_public = public_sec1(as_cle);
    let secret = as_cle.diffie_hellman(&ua);
    // PRK_key = HKDF-Extract(auth, ecdh) ; IKM = HKDF-Expand(PRK_key, "WebPush: info\0" || ua || as, 32)
    let mut info = b"WebPush: info\0".to_vec();
    info.extend_from_slice(&ua.to_sec1_bytes());
    info.extend_from_slice(&as_public);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), secret.raw_secret_bytes().as_slice()).expand(&info, &mut ikm).map_err(|_| "dérivation de clé")?;
    let prk = Hkdf::<Sha256>::new(Some(&sel), &ikm);
    let mut cek = [0u8; 16];
    let mut nonce = [0u8; 12];
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek).map_err(|_| "dérivation de clé")?;
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce).map_err(|_| "dérivation de clé")?;
    // Dernier (seul) enregistrement : contenu puis délimiteur 0x02.
    let mut clair = charge.to_vec();
    clair.push(2);
    let chiffre = aes_gcm::Aes128Gcm::new_from_slice(&cek).map_err(|_| "clé AES")?.encrypt(&nonce.into(), clair.as_slice()).map_err(|_| "chiffrement")?;
    let mut corps = Vec::with_capacity(21 + as_public.len() + chiffre.len());
    corps.extend_from_slice(&sel);
    corps.extend_from_slice(&TAILLE_ENREGISTREMENT.to_be_bytes());
    corps.push(as_public.len() as u8);
    corps.extend_from_slice(&as_public);
    corps.extend_from_slice(&chiffre);
    Ok(corps)
}

/// Comme `chiffrer_avec`, avec une clé éphémère et un sel neufs.
pub fn chiffrer(ua_public: &[u8], auth: &[u8], charge: &[u8]) -> Result<Vec<u8>, String> {
    let mut sel = [0u8; 16];
    getrandom::fill(&mut sel).expect("générateur aléatoire");
    chiffrer_avec(ua_public, auth, charge, &nouvelle_cle(), sel)
}

/// Origine (`https://hote[:port]`) d'une adresse de service de notification : le `aud` du JWT.
pub fn origine(endpoint: &str) -> Option<String> {
    let (schema, reste) = endpoint.split_once("://")?;
    let hote = reste.split(['/', '?', '#']).next().filter(|h| !h.is_empty())?;
    Some(format!("{schema}://{hote}"))
}

/// Jeton JWT ES256 de VAPID, valable jusqu'à `exp`.
pub fn jwt_vapid(cle: &SecretKey, aud: &str, exp: u64) -> Result<String, String> {
    let entete = base64url(br#"{"typ":"JWT","alg":"ES256"}"#);
    let revendications = base64url(serde_json::json!({ "aud": aud, "exp": exp, "sub": SUJET_VAPID }).to_string().as_bytes());
    let message = format!("{entete}.{revendications}");
    let signataire = SigningKey::from_slice(&cle.to_bytes()).map_err(|_| "clé VAPID invalide")?;
    let signature: Signature = signataire.try_sign(message.as_bytes()).map_err(|_| "signature VAPID")?;
    Ok(format!("{message}.{}", base64url(&signature.to_bytes())))
}

/// Vérifie la signature d'un JWT ES256 avec la clé publique (points SEC1) ; rend les revendications.
pub fn verifier_jwt(jwt: &str, public: &[u8]) -> Option<serde_json::Value> {
    let (message, signature) = jwt.rsplit_once('.')?;
    let signature = Signature::from_slice(&base64url_decoder(signature)?).ok()?;
    VerifyingKey::from_sec1_bytes(public).ok()?.verify(message.as_bytes(), &signature).ok()?;
    let revendications = message.split('.').nth(1)?;
    serde_json::from_slice(&base64url_decoder(revendications)?).ok()
}

/// Valeur de l'en-tête `Authorization` : `vapid t=<jwt>, k=<clé publique>`.
pub fn autorisation_vapid(cle: &SecretKey, endpoint: &str, maintenant: u64) -> Result<String, String> {
    let aud = origine(endpoint).ok_or("adresse d'abonnement invalide")?;
    Ok(format!("vapid t={}, k={}", jwt_vapid(cle, &aud, maintenant + VALIDITE_JWT)?, base64url(&public_sec1(cle))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Vec<u8> {
        base64url_decoder(s).unwrap()
    }

    /// Annexe A de la RFC 8291 : message « When I grow up, I want to be a watermelon ».
    #[test]
    fn vecteur_rfc_8291() {
        let as_cle = SecretKey::from_slice(&d("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
        assert_eq!(base64url(&public_sec1(&as_cle)), "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8");
        let ua_cle = SecretKey::from_slice(&d("q1dXpw3UpT5VOmu_cf_v6ih07Aems3njxI-JWgLcM94")).unwrap();
        let ua_public = d("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4");
        assert_eq!(public_sec1(&ua_cle), ua_public);
        let sel: [u8; 16] = d("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
        let corps = chiffrer_avec(&ua_public, &d("BTBZMqHH6r4Tts7J_aSIgg"), b"When I grow up, I want to be a watermelon", &as_cle, sel).unwrap();
        assert_eq!(
            base64url(&corps),
            "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
        );
    }

    #[test]
    fn chiffrement_aleatoire_et_limite() {
        let ua = nouvelle_cle();
        let corps = chiffrer(&public_sec1(&ua), &[7; 16], b"bonjour").unwrap();
        assert_eq!(corps[16..20], [0, 0, 16, 0]);
        assert_eq!(corps[20], 65);
        assert_ne!(corps, chiffrer(&public_sec1(&ua), &[7; 16], b"bonjour").unwrap(), "sel et clé éphémère neufs");
        assert!(chiffrer(&public_sec1(&ua), &[7; 16], &vec![b'x'; CHARGE_MAX + 1]).is_err());
        assert!(chiffrer(&[1, 2, 3], &[7; 16], b"x").is_err());
    }

    #[test]
    fn jwt_vapid_signe_et_verifiable() {
        let cle = nouvelle_cle();
        let public = public_sec1(&cle);
        let h = autorisation_vapid(&cle, "https://fcm.googleapis.com/fcm/send/abc?x=1", 1_000).unwrap();
        let (t, k) = h.strip_prefix("vapid t=").unwrap().split_once(", k=").unwrap();
        assert_eq!(k, base64url(&public));
        let rev = verifier_jwt(t, &public).expect("signature valide");
        assert_eq!(rev["aud"], "https://fcm.googleapis.com");
        assert_eq!(rev["sub"], "mailto:contact@e-ko.rip");
        assert_eq!(rev["exp"], 1_000 + 12 * 3600);
        assert!(verifier_jwt(t, &public_sec1(&nouvelle_cle())).is_none(), "autre clé");
        let (m, s) = t.rsplit_once('.').unwrap();
        assert!(verifier_jwt(&format!("{}A{}.{s}", &m[..m.len() - 1], &m[m.len() - 1..]), &public).is_none(), "message modifié");
        assert!(verifier_jwt(&format!("{m}.{s}"), &public).is_some());
        assert_eq!(origine("https://updates.push.services.mozilla.com:443/wpush/v2/x").unwrap(), "https://updates.push.services.mozilla.com:443");
    }
}
