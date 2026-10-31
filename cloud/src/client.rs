//! Côté serveur RIP : ouvrir le tunnel vers la plate-forme et y servir son API.

use crate::ws::{self, FluxWs};
use crate::{CHEMIN_TUNNEL, PROTOCOLE};
use axum::Router;
use axum::http::{Request, StatusCode, Uri, header};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
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
    let tcp = tokio::time::timeout(DELAI_CONNEXION, tokio::net::TcpStream::connect((hote.as_str(), port))).await.map_err(|_| reseau("délai dépassé"))?.map_err(reseau)?;
    let _ = tcp.set_nodelay(true);
    let autorisation = format!("Bearer {site}.{cle}");
    if https {
        let nom = rustls::pki_types::ServerName::try_from(hote.clone()).map_err(reseau)?;
        let tls = tokio::time::timeout(DELAI_CONNEXION, connecteur().connect(nom, tcp)).await.map_err(|_| reseau("délai TLS dépassé"))?.map_err(reseau)?;
        tunnel(tls, &entete_hote, &autorisation, agent, service, connecte).await
    } else {
        tunnel(tcp, &entete_hote, &autorisation, agent, service, connecte).await
    }
}

async fn tunnel<T>(io: T, hote: &str, autorisation: &str, agent: &str, service: Router, connecte: impl FnOnce()) -> Result<(), Erreur>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
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
        .header(header::SEC_WEBSOCKET_PROTOCOL, PROTOCOLE)
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
    let flux = hyper::upgrade::on(reponse).await.map_err(reseau)?;
    let flux = TokioIo::new(FluxWs::nouveau(TokioIo::new(flux), tokio_tungstenite::tungstenite::protocol::Role::Client).await);
    connecte();
    hyper::server::conn::http2::Builder::new(TokioExecutor::new())
        .timer(TokioTimer::new())
        .keep_alive_interval(Duration::from_secs(25))
        .keep_alive_timeout(Duration::from_secs(20))
        .adaptive_window(true)
        .max_concurrent_streams(64)
        .serve_connection(flux, TowerToHyperService::new(service))
        .await
        .map_err(reseau)
}
