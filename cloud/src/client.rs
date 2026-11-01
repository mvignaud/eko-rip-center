//! Côté serveur RIP : ouvrir le tunnel vers la plate-forme et y servir son API.

use crate::montant;
use crate::ws::{self, FluxWs};
use crate::{CHEMIN_TUNNEL, PROTOCOLE};
use axum::Router;
use axum::http::{Request, StatusCode, Uri, header};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

const DELAI_CONNEXION: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq)]
pub enum Erreur {
    /// Plate-forme injoignable, coupure, certificat refusé…
    Reseau(String),
    /// La plate-forme a répondu non : code de sa réponse (`cle_invalide`…).
    Refus { statut: u16, code: String },
}

impl std::fmt::Display for Erreur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Erreur::Reseau(e) => write!(f, "réseau : {e}"),
            Erreur::Refus { statut, code } => write!(f, "refus {statut} : {code}"),
        }
    }
}

fn reseau(e: impl std::fmt::Display) -> Erreur {
    Erreur::Reseau(e.to_string())
}

/// Connecteur TLS : racines publiques (webpki), sans dépendre des certificats du système.
fn connecteur() -> tokio_rustls::TlsConnector {
    let mut racines = rustls::RootCertStore::empty();
    racines.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("versions TLS")
        .with_root_certificates(racines)
        .with_no_client_auth();
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

/// Ouvre le tunnel vers `base` (`https://platform.e-ko.rip`) au nom du site, appelle
/// `connecte` une fois la plate-forme d'accord, puis sert `service` dans le tunnel jusqu'à
/// sa fermeture. Rend Ok(()) quand le tunnel se ferme normalement.
pub async fn servir(base: &str, site: &str, cle: &str, agent: &str, service: Router, connecte: impl FnOnce()) -> Result<(), Erreur> {
    let uri: Uri = base.parse().map_err(|_| Erreur::Reseau(format!("adresse invalide : {base}")))?;
    let https = match uri.scheme_str() {
        Some("https") => true,
        Some("http") => false,
        _ => return Err(Erreur::Reseau(format!("adresse invalide : {base}"))),
    };
    let hote = uri.host().ok_or_else(|| Erreur::Reseau(format!("adresse invalide : {base}")))?.to_string();
    let port = uri.port_u16().unwrap_or(if https { 443 } else { 80 });
    let entete_hote = uri.authority().map(|a| a.to_string()).unwrap_or_else(|| hote.clone());
    let cible = Cible { hote, port, https, entete_hote };
    let io = connecter(&cible).await?;
    let autorisation = format!("Bearer {site}.{cle}");
    tunnel(io, &cible, &autorisation, agent, service, connecte).await
}

/// Où joindre la plate-forme.
#[derive(Clone)]
struct Cible {
    hote: String,
    port: u16,
    https: bool,
    /// Valeur de l'en-tête `Host`.
    entete_hote: String,
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// Connexion TCP (et TLS en HTTPS) à la plate-forme.
async fn connecter(c: &Cible) -> Result<Box<dyn Io>, Erreur> {
    let tcp = tokio::time::timeout(DELAI_CONNEXION, tokio::net::TcpStream::connect((c.hote.as_str(), c.port))).await.map_err(|_| reseau("délai dépassé"))?.map_err(reseau)?;
    let _ = tcp.set_nodelay(true);
    if !c.https {
        return Ok(Box::new(tcp));
    }
    let nom = rustls::pki_types::ServerName::try_from(c.hote.clone()).map_err(reseau)?;
    let tls = tokio::time::timeout(DELAI_CONNEXION, connecteur().connect(nom, tcp)).await.map_err(|_| reseau("délai TLS dépassé"))?.map_err(reseau)?;
    Ok(Box::new(tls))
}

async fn tunnel(io: Box<dyn Io>, cible: &Cible, autorisation: &str, agent: &str, service: Router, connecte: impl FnOnce()) -> Result<(), Erreur> {
    let hote = cible.entete_hote.as_str();
    let (mut envoi, connexion) = hyper::client::conn::http1::handshake(TokioIo::new(io)).await.map_err(reseau)?;
    tokio::spawn(async move {
        let _ = connexion.with_upgrades().await;
    });
    let cle_ws = ws::cle_websocket();
    let requete = Request::get(CHEMIN_TUNNEL)
        .header(header::HOST, hote)
        .header(header::AUTHORIZATION, autorisation)
        .header(header::CONNECTION, "upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_VERSION, "13")
        .header(header::SEC_WEBSOCKET_KEY, &cle_ws)
        // `eko-tunnel/2` d'abord : sens montant en requêtes HTTP si la plate-forme le choisit
        // (frontal qui retient le sens montant des WebSocket), sinon WebSocket dans les deux sens.
        .header(header::SEC_WEBSOCKET_PROTOCOL, format!("{}, {PROTOCOLE}", montant::PROTOCOLE_2))
        .header(header::USER_AGENT, agent)
        .body(Empty::<Bytes>::new())
        .map_err(reseau)?;
    let reponse = tokio::time::timeout(DELAI_CONNEXION, envoi.send_request(requete)).await.map_err(|_| reseau("délai dépassé"))?.map_err(reseau)?;
    if reponse.status() != StatusCode::SWITCHING_PROTOCOLS {
        let statut = reponse.status().as_u16();
        let corps = tokio::time::timeout(DELAI_CONNEXION, http_body_util::Limited::new(reponse.into_body(), 4096).collect()).await.ok().and_then(Result::ok).map(|c| c.to_bytes()).unwrap_or_default();
        let code = serde_json::from_slice::<serde_json::Value>(&corps).ok().and_then(|v| v["code"].as_str().map(str::to_string)).unwrap_or_else(|| format!("http_{statut}"));
        return Err(Erreur::Refus { statut, code });
    }
    let attendu = ws::acceptation(cle_ws.as_bytes());
    if reponse.headers().get(header::SEC_WEBSOCKET_ACCEPT).and_then(|v| v.to_str().ok()) != Some(attendu.as_str()) {
        return Err(reseau("poignée de main WebSocket invalide"));
    }
    let v2 = reponse.headers().get(header::SEC_WEBSOCKET_PROTOCOL).and_then(|v| v.to_str().ok()).is_some_and(|v| v.trim().eq_ignore_ascii_case(montant::PROTOCOLE_2));
    let flux = hyper::upgrade::on(reponse).await.map_err(reseau)?;
    let mut flux = FluxWs::nouveau(ws::SansBlocs::nouveau(TokioIo::new(flux)), tokio_tungstenite::tungstenite::protocol::Role::Client).await;
    if !v2 {
        connecte();
        return servir_h2(flux, service).await;
    }
    // `eko-tunnel/2` : la plate-forme annonce la session, puis tout ce que ce serveur envoie
    // part en requêtes HTTP.
    let annonce = tokio::time::timeout(DELAI_CONNEXION, flux.lire_texte()).await.map_err(|_| reseau("annonce de session : délai dépassé"))?.map_err(reseau)?;
    let session = annonce.strip_prefix("session ").filter(|s| montant::session_valide(s)).ok_or_else(|| reseau("annonce de session invalide"))?.to_string();
    let (lecture, _ecriture_ws) = tokio::io::split(flux);
    let (ecrivain, depart) = montant::ecrivain();
    let envois = tokio::spawn(envoyer_montant(cible.clone(), session, agent.to_string(), depart));
    connecte();
    let r = servir_h2(tokio::io::join(lecture, ecrivain), service).await;
    envois.abort();
    r
}

/// Sert l'API du serveur RIP en HTTP/2 dans le tunnel.
async fn servir_h2<I: AsyncRead + AsyncWrite + Unpin + Send + 'static>(io: I, service: Router) -> Result<(), Erreur> {
    hyper::server::conn::http2::Builder::new(TokioExecutor::new())
        .timer(TokioTimer::new())
        .keep_alive_interval(Duration::from_secs(25))
        .keep_alive_timeout(Duration::from_secs(20))
        .adaptive_window(true)
        .max_concurrent_streams(64)
        .serve_connection(TokioIo::new(io), TowerToHyperService::new(service))
        .await
        .map_err(|e| Erreur::Reseau(chaine(&e)))
}

/// Envoie le sens montant d'un tunnel `eko-tunnel/2` : un `POST` par lot, dans l'ordre, sur
/// une connexion gardée ouverte ; un envoi qui échoue est refait (même numéro) sur une
/// nouvelle connexion, jusqu'à 6 fois.
async fn envoyer_montant(cible: Cible, session: String, agent: String, depart: montant::Depart) {
    let chemin = montant::chemin(&session);
    let mut conn: Option<hyper::client::conn::http1::SendRequest<Full<Bytes>>> = None;
    let mut seq = 0u64;
    while let Some(lot) = depart.prochain().await {
        let mut essai = 0u32;
        loop {
            match poster(&cible, &mut conn, &chemin, &agent, seq, lot.clone()).await {
                Ok(()) => break,
                Err(Envoi::Perdu(r)) => return depart.echec(r),
                Err(Envoi::Reessayer(r)) => {
                    conn = None;
                    essai += 1;
                    if essai >= 6 {
                        return depart.echec(r);
                    }
                    tokio::time::sleep(Duration::from_millis(250 << essai.min(4))).await;
                }
            }
        }
        seq += 1;
    }
}

enum Envoi {
    Perdu(String),
    Reessayer(String),
}

async fn poster(cible: &Cible, conn: &mut Option<hyper::client::conn::http1::SendRequest<Full<Bytes>>>, chemin: &str, agent: &str, seq: u64, lot: Bytes) -> Result<(), Envoi> {
    let reessayer = |e: String| Envoi::Reessayer(e);
    if conn.as_ref().is_none_or(|c| c.is_closed()) {
        let io = connecter(cible).await.map_err(|e| reessayer(e.to_string()))?;
        let (envoi, connexion) = hyper::client::conn::http1::handshake(TokioIo::new(io)).await.map_err(|e| reessayer(e.to_string()))?;
        tokio::spawn(async move {
            let _ = connexion.await;
        });
        *conn = Some(envoi);
    }
    let c = conn.as_mut().expect("connexion ouverte");
    tokio::time::timeout(DELAI_CONNEXION, c.ready()).await.map_err(|_| reessayer("délai dépassé".into()))?.map_err(|e| reessayer(e.to_string()))?;
    let requete = Request::post(chemin)
        .header(header::HOST, &cible.entete_hote)
        .header(header::USER_AGENT, agent)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, lot.len())
        .header(montant::ENTETE_SEQ, seq)
        .body(Full::new(lot))
        .map_err(|e| Envoi::Perdu(e.to_string()))?;
    let reponse = tokio::time::timeout(Duration::from_secs(60), c.send_request(requete)).await.map_err(|_| reessayer("délai dépassé".into()))?.map_err(|e| reessayer(e.to_string()))?;
    let statut = reponse.status();
    // Corps lu pour garder la connexion réutilisable.
    let _ = tokio::time::timeout(Duration::from_secs(10), http_body_util::Limited::new(reponse.into_body(), 4096).collect()).await;
    match statut.as_u16() {
        200..=299 => Ok(()),
        404 | 409 | 410 => Err(Envoi::Perdu(format!("tunnel perdu ({statut})"))),
        _ => Err(reessayer(format!("http {statut}"))),
    }
}

/// Erreur et ses causes successives (« connection error : … : … »), pour le journal.
pub fn chaine(e: &(dyn std::error::Error + 'static)) -> String {
    let mut texte = e.to_string();
    let mut cause = e.source();
    while let Some(c) = cause {
        texte.push_str(" : ");
        texte.push_str(&c.to_string());
        cause = c.source();
    }
    texte
}
