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
}
