//! Le tunnel en WebSocket : les hébergements mutualisés (LiteSpeed, Passenger…) ne laissent
//! passer que `Upgrade: websocket`. Le WebSocket ne sert que d'enveloppe : la connexion
//! HTTP/2 du tunnel voyage telle quelle dans des messages binaires.

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};

pub use tokio_tungstenite::tungstenite::handshake::client::generate_key as cle_websocket;
pub use tokio_tungstenite::tungstenite::handshake::derive_accept_key as acceptation;

/// Flux d'octets porté par un WebSocket (un message binaire par écriture).
pub struct FluxWs<S> {
    ws: WebSocketStream<S>,
    lu: Bytes,
}

impl<S: AsyncRead + AsyncWrite + Unpin> FluxWs<S> {
    /// `io` : la connexion juste après la poignée de main HTTP (réponse 101).
    pub async fn nouveau(io: S, role: Role) -> Self {
        // Les messages font la taille d'une écriture d'HTTP/2 (quelques Kio à 16 Mio).
        let config = WebSocketConfig::default().max_message_size(Some(64 << 20)).max_frame_size(Some(64 << 20));
        FluxWs { ws: WebSocketStream::from_raw_socket(io, role, Some(config)).await, lu: Bytes::new() }
    }

    /// Envoie un message texte (avant le début du flux d'octets : annonce de session).
    pub async fn envoyer_texte(&mut self, texte: &str) -> io::Result<()> {
        self.ws.send(Message::text(texte)).await.map_err(erreur)
    }

    /// Lit le premier message, qui doit être du texte (avant le début du flux d'octets).
    pub async fn lire_texte(&mut self) -> io::Result<String> {
        loop {
            match self.ws.next().await {
                Some(Ok(Message::Text(t))) => return Ok(t.to_string()),
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(_)) | None => return Err(io::Error::new(io::ErrorKind::InvalidData, "annonce de session attendue")),
                Some(Err(e)) => return Err(erreur(e)),
            }
        }
    }
}

fn erreur(e: tokio_tungstenite::tungstenite::Error) -> io::Error {
    use tokio_tungstenite::tungstenite::Error as E;
    match e {
        E::Io(e) => e,
        E::ConnectionClosed | E::AlreadyClosed => io::Error::from(io::ErrorKind::BrokenPipe),
        e => io::Error::other(e),
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for FluxWs<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        loop {
            if !self.lu.is_empty() {
                let n = self.lu.len().min(buf.remaining());
                buf.put_slice(&self.lu.split_to(n));
                return Poll::Ready(Ok(()));
            }
            match ready!(self.ws.poll_next_unpin(cx)) {
                Some(Ok(Message::Binary(b))) => self.lu = b,
                // Ping / pong : tungstenite répond tout seul.
                Some(Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_))) => {}
                Some(Ok(Message::Text(_))) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidData, "message texte dans le tunnel"))),
                // Fin du flux.
                Some(Ok(Message::Close(_))) | None => return Poll::Ready(Ok(())),
                Some(Err(e)) => {
                    let e = erreur(e);
                    return Poll::Ready(if e.kind() == io::ErrorKind::BrokenPipe { Ok(()) } else { Err(e) });
                }
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for FluxWs<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        ready!(self.ws.poll_ready_unpin(cx)).map_err(erreur)?;
        self.ws.start_send_unpin(Message::Binary(Bytes::copy_from_slice(buf))).map_err(erreur)?;
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.ws.poll_flush_unpin(cx).map_err(erreur)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match ready!(self.ws.poll_close_unpin(cx)) {
            Err(e) if !matches!(e, tokio_tungstenite::tungstenite::Error::ConnectionClosed | tokio_tungstenite::tungstenite::Error::AlreadyClosed) => Poll::Ready(Err(erreur(e))),
            _ => Poll::Ready(Ok(())),
        }
    }
}

/// Sens descendant d'un tunnel passé par un frontal qui ré-émet la suite de la réponse 101
/// en blocs (`Transfer-Encoding: chunked` sans le dire : constaté derrière Passenger chez
/// o2switch, « 3d\r\n » devant les trames). Au premier octet reçu : un chiffre hexadécimal
/// ASCII ne peut pas ouvrir une trame WebSocket venue du serveur (0x81, 0x82, 0x88… ou
/// 0x00-0x0f), il annonce donc un découpage en blocs, qui est alors retiré ; sinon le flux
/// passe tel quel. L'écriture n'est jamais touchée.
pub struct SansBlocs<S> {
    io: S,
    etat: Blocs,
    entree: Vec<u8>,
    pos: usize,
}

#[derive(Debug, PartialEq)]
enum Blocs {
    Detecter,
    Brut,
    /// Ligne de taille en cours de lecture.
    Taille(Vec<u8>),
    /// Octets de données restant dans le bloc.
    Donnees(u64),
    /// Octets de fin de bloc (CR LF) restant à sauter.
    FinBloc(u8),
    /// Bloc de taille 0 : fin du flux.
    Fini,
}

impl<S> SansBlocs<S> {
    pub fn nouveau(io: S) -> Self {
        SansBlocs { io, etat: Blocs::Detecter, entree: Vec::new(), pos: 0 }
    }

    /// Traite les octets en attente ; rend le nombre d'octets utiles copiés dans `sortie`.
    fn traiter(&mut self, sortie: &mut ReadBuf<'_>) -> io::Result<usize> {
        let mut copie = 0;
        while self.pos < self.entree.len() && sortie.remaining() > 0 {
            match &mut self.etat {
                Blocs::Detecter => {
                    self.etat = if self.entree[self.pos].is_ascii_hexdigit() { Blocs::Taille(Vec::new()) } else { Blocs::Brut };
                }
                Blocs::Brut | Blocs::Donnees(_) => {
                    let dispo = self.entree.len() - self.pos;
                    let max = match self.etat {
                        Blocs::Donnees(reste) => reste.min(dispo as u64) as usize,
                        _ => dispo,
                    };
                    let n = max.min(sortie.remaining());
                    sortie.put_slice(&self.entree[self.pos..self.pos + n]);
                    self.pos += n;
                    copie += n;
                    if let Blocs::Donnees(reste) = &mut self.etat {
                        *reste -= n as u64;
                        if *reste == 0 {
                            self.etat = Blocs::FinBloc(2);
                        }
                    }
                }
                Blocs::Taille(ligne) => {
                    let o = self.entree[self.pos];
                    self.pos += 1;
                    if o != b'\n' {
                        if ligne.len() > 1024 {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, "ligne de bloc trop longue"));
                        }
                        ligne.push(o);
                        continue;
                    }
                    let texte = String::from_utf8_lossy(ligne);
                    let hex = texte.trim_end_matches('\r').split(';').next().unwrap_or("").trim();
                    let taille = u64::from_str_radix(hex, 16).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "taille de bloc invalide"))?;
                    self.etat = if taille == 0 { Blocs::Fini } else { Blocs::Donnees(taille) };
                }
                Blocs::FinBloc(reste) => {
                    let o = self.entree[self.pos];
                    self.pos += 1;
                    // Tolère un LF seul.
                    if o == b'\n' || *reste == 1 {
                        self.etat = Blocs::Taille(Vec::new());
                    } else {
                        *reste -= 1;
                    }
                }
                Blocs::Fini => {
                    self.pos = self.entree.len();
                }
            }
        }
        if self.pos >= self.entree.len() {
            self.entree.clear();
            self.pos = 0;
        }
        Ok(copie)
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for SansBlocs<S> {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let moi = &mut *self;
        loop {
            if moi.etat == Blocs::Brut && moi.pos >= moi.entree.len() {
                return Pin::new(&mut moi.io).poll_read(cx, buf);
            }
            if moi.etat == Blocs::Fini {
                return Poll::Ready(Ok(()));
            }
            if moi.pos < moi.entree.len() && moi.traiter(buf)? > 0 {
                return Poll::Ready(Ok(()));
            }
            if moi.pos < moi.entree.len() {
                // Des octets restent (le tampon de sortie est plein ou l'état a changé).
                if buf.remaining() == 0 {
                    return Poll::Ready(Ok(()));
                }
                continue;
            }
            let mut tampon = [0u8; 16 * 1024];
            let mut lu = ReadBuf::new(&mut tampon);
            ready!(Pin::new(&mut moi.io).poll_read(cx, &mut lu))?;
            if lu.filled().is_empty() {
                return Poll::Ready(Ok(()));
            }
            moi.entree.extend_from_slice(lu.filled());
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for SansBlocs<S> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn les_octets_traversent_dans_les_deux_sens() {
        let (a, b) = tokio::io::duplex(1024);
        let (mut client, mut serveur) = tokio::join!(FluxWs::nouveau(a, Role::Client), FluxWs::nouveau(b, Role::Server));
        let gros = vec![7u8; 200_000];
        let envoi = {
            let gros = gros.clone();
            tokio::spawn(async move {
                client.write_all(b"bonjour").await.unwrap();
                client.write_all(&gros).await.unwrap();
                client.flush().await.unwrap();
                let mut r = [0u8; 2];
                client.read_exact(&mut r).await.unwrap();
                assert_eq!(&r, b"ok");
                client.shutdown().await.unwrap();
            })
        };
        let mut recu = vec![0u8; 7 + gros.len()];
        serveur.read_exact(&mut recu).await.unwrap();
        assert_eq!(&recu[..7], b"bonjour");
        assert_eq!(&recu[7..], &gros[..]);
        serveur.write_all(b"ok").await.unwrap();
        serveur.flush().await.unwrap();
        let mut reste = Vec::new();
        serveur.read_to_end(&mut reste).await.unwrap();
        assert!(reste.is_empty());
        envoi.await.unwrap();
    }

    #[test]
    fn acceptation_conforme_a_la_rfc() {
        // Exemple de la RFC 6455, § 1.3.
        assert_eq!(acceptation(b"dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    async fn tout_lire(entree: Vec<Vec<u8>>) -> Vec<u8> {
        use tokio::io::AsyncReadExt;
        let mut b = tokio_test_flux(entree);
        let mut s = SansBlocs::nouveau(&mut b);
        let mut v = Vec::new();
        s.read_to_end(&mut v).await.unwrap();
        v
    }

    // Lecteur qui rend les morceaux un par un (découpages arbitraires).
    fn tokio_test_flux(morceaux: Vec<Vec<u8>>) -> impl AsyncRead + Unpin {
        let (mut a, b) = tokio::io::duplex(1 << 20);
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            for m in morceaux {
                a.write_all(&m).await.unwrap();
                a.flush().await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        b
    }

    #[tokio::test]
    async fn blocs_retires_echantillon_o2switch() {
        // Octets réellement reçus derrière le frontal d'o2switch : « 3d\r\n » puis deux trames.
        let mut trames = vec![0x82, 0x18];
        trames.extend_from_slice(b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
        trames.extend_from_slice(&[0x82, 0x21]);
        trames.extend_from_slice(&[0u8; 33]);
        assert_eq!(trames.len(), 0x3d);
        let mut flux = b"3d\r\n".to_vec();
        flux.extend_from_slice(&trames);
        flux.extend_from_slice(b"\r\n4;ext=1\r\nABCD\r\n0\r\n\r\n");
        let mut attendu = trames.clone();
        attendu.extend_from_slice(b"ABCD");
        assert_eq!(tout_lire(vec![flux.clone()]).await, attendu);
        // Même flux livré octet par octet.
        assert_eq!(tout_lire(flux.iter().map(|o| vec![*o]).collect()).await, attendu);
    }

    #[tokio::test]
    async fn flux_websocket_ordinaire_intact() {
        let mut flux = vec![0x82, 0x03];
        flux.extend_from_slice(b"abc3d\r\n");
        assert_eq!(tout_lire(vec![flux.clone()]).await, flux);
    }

    #[tokio::test]
    async fn tunnel_a_travers_blocs() {
        // Serveur WebSocket dont la sortie est découpée en blocs, client derrière SansBlocs.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (serveur, relais_cote_serveur) = tokio::io::duplex(1 << 20);
        let (client, mut relais_cote_client) = tokio::io::duplex(1 << 20);
        let (mut rs_lit, mut rs_ecrit) = tokio::io::split(relais_cote_serveur);
        tokio::spawn(async move {
            // serveur → client : chaque lecture devient un bloc.
            let mut t = [0u8; 4096];
            loop {
                let n = rs_lit.read(&mut t).await.unwrap_or(0);
                if n == 0 {
                    break;
                }
                let mut bloc = format!("{n:x}\r\n").into_bytes();
                bloc.extend_from_slice(&t[..n]);
                bloc.extend_from_slice(b"\r\n");
                if relais_cote_client.write_all(&bloc).await.is_err() {
                    break;
                }
                // client → serveur, intact (lecture non bloquante improbable ici : sens unique dans ce test).
                let _ = &mut rs_ecrit;
            }
        });
        let mut s = FluxWs::nouveau(serveur, Role::Server).await;
        let mut c = FluxWs::nouveau(SansBlocs::nouveau(client), Role::Client).await;
        s.write_all(b"bonjour tunnel").await.unwrap();
        s.flush().await.unwrap();
        let mut v = [0u8; 14];
        c.read_exact(&mut v).await.unwrap();
        assert_eq!(&v, b"bonjour tunnel");
    }
}
