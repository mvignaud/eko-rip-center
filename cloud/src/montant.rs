//! Sens montant du tunnel par requêtes HTTP (sous-protocole `eko-tunnel/2`).
//!
//! Derrière le frontal d'un hébergement mutualisé (o2switch : PowerBoost puis Passenger),
//! le WebSocket du tunnel ne marche que dans un sens : ce que la plate-forme envoie arrive
//! au serveur RIP, mais ce que le serveur RIP envoie reste dans un tampon du frontal, qui le
//! prend pour un corps de requête jamais terminé. En `eko-tunnel/2`, le WebSocket ne porte
//! donc que le sens descendant ; le sens montant part en `POST /tunnel/<session>/montant`
//! successifs, numérotés, chacun complet (le frontal les transmet donc aussitôt).
//!
//! - La plate-forme choisit `eko-tunnel/2` quand le serveur RIP le propose et qu'elle est
//!   réglée pour (`EKO_CLOUD_MONTANT_POST=1`, posé par le lanceur o2switch) ; sinon
//!   `eko-tunnel/1`, où le WebSocket porte les deux sens.
//! - Premier message du WebSocket (texte) : `session <id>`, l'identifiant de session (128 bits
//!   aléatoires). Il ne circule que dans la connexion authentifiée par la clé du site et sert
//!   de droit d'écriture sur ce tunnel.
//! - `POST /tunnel/<id>/montant`, en-tête `X-EKO-Seq: <n>` (0, 1, 2…), corps = octets du
//!   tunnel. Réponse 204. Un numéro déjà reçu est acquitté sans effet (renvoi après une
//!   coupure) ; un numéro en avance ou une session inconnue : 409, le tunnel est perdu.

use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::{Notify, mpsc};

pub const PROTOCOLE_2: &str = "eko-tunnel/2";
/// Taille maximale du corps d'un envoi montant.
pub const MAX_CORPS: usize = 1 << 20;
pub const ENTETE_SEQ: &str = "x-eko-seq";

/// Chemin d'envoi montant d'une session.
pub fn chemin(session: &str) -> String {
    format!("{}/{session}/montant", crate::CHEMIN_TUNNEL)
}

/// Identifiant de session : 32 caractères hexadécimaux.
pub fn session_valide(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|o| o.is_ascii_hexdigit())
}

// --- Côté plate-forme ---

struct Session {
    attendu: tokio::sync::Mutex<u64>,
    envoi: mpsc::Sender<Bytes>,
}

/// Sessions montantes ouvertes, par identifiant.
#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Arc<Session>>>);

/// Issue d'un envoi montant.
#[derive(Debug, PartialEq)]
pub enum Reception {
    /// Reçu (ou déjà reçu).
    Ok,
    /// Session inconnue ou close, numéro en avance : le tunnel est perdu.
    Perdu,
}

impl Sessions {
    /// Ouvre une session ; rend son identifiant et le lecteur des octets montants.
    pub fn ouvrir(&self) -> (String, Lecteur) {
        let (envoi, recu) = mpsc::channel(64);
        let id = crate::aleatoire_hex(16);
        self.0.lock().unwrap().insert(id.clone(), Arc::new(Session { attendu: tokio::sync::Mutex::new(0), envoi }));
        (id, Lecteur { recu, lu: Bytes::new() })
    }

    pub fn fermer(&self, id: &str) {
        self.0.lock().unwrap().remove(id);
    }

    /// Un envoi montant : appliqué dans l'ordre des numéros, une seule fois.
    pub async fn recevoir(&self, id: &str, seq: u64, corps: Bytes) -> Reception {
        let Some(s) = self.0.lock().unwrap().get(id).cloned() else { return Reception::Perdu };
        // Le verrou tient pendant la remise : un renvoi arrivé en même temps que l'original
        // attend son issue et le voit déjà compté.
        let mut attendu = s.attendu.lock().await;
        if seq < *attendu {
            return Reception::Ok;
        }
        if seq > *attendu {
            return Reception::Perdu;
        }
        if !corps.is_empty() && s.envoi.send(corps).await.is_err() {
            return Reception::Perdu;
        }
        *attendu += 1;
        Reception::Ok
    }
}

/// Octets montants d'une session, dans l'ordre (fin : session fermée).
pub struct Lecteur {
    recu: mpsc::Receiver<Bytes>,
    lu: Bytes,
}

impl AsyncRead for Lecteur {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        if self.lu.is_empty() {
            match std::task::ready!(self.recu.poll_recv(cx)) {
                Some(b) => self.lu = b,
                None => return Poll::Ready(Ok(())),
            }
        }
        let n = self.lu.len().min(buf.remaining());
        let morceau = self.lu.split_to(n);
        buf.put_slice(&morceau);
        Poll::Ready(Ok(()))
    }
}

// --- Côté serveur RIP ---

#[derive(Default)]
struct Etat {
    tampon: BytesMut,
    /// Écriture en attente de place dans le tampon.
    attente: Option<Waker>,
    erreur: Option<String>,
    ferme: bool,
}

/// Écriture montante : les octets s'accumulent pendant qu'un envoi est en route, puis
/// partent ensemble au suivant (au plus `MAX_CORPS` par envoi).
pub struct Ecrivain {
    etat: Arc<Mutex<Etat>>,
    reveil: Arc<Notify>,
}

/// Le côté « envoi » d'un `Ecrivain` : la tâche qui fait les requêtes.
pub struct Depart {
    etat: Arc<Mutex<Etat>>,
    reveil: Arc<Notify>,
}

/// Place au plus ce volume en attente avant de faire patienter les écritures.
const MAX_ATTENTE: usize = 4 * MAX_CORPS;

pub fn ecrivain() -> (Ecrivain, Depart) {
    let etat = Arc::new(Mutex::new(Etat::default()));
    let reveil = Arc::new(Notify::new());
    (Ecrivain { etat: etat.clone(), reveil: reveil.clone() }, Depart { etat, reveil })
}

impl Depart {
    /// Prochain lot à envoyer ; `None` quand l'écrivain est fermé et le tampon vide.
    pub async fn prochain(&self) -> Option<Bytes> {
        loop {
            {
                let mut e = self.etat.lock().unwrap();
                if !e.tampon.is_empty() {
                    let n = e.tampon.len().min(MAX_CORPS);
                    let lot = e.tampon.split_to(n).freeze();
                    if let Some(w) = e.attente.take() {
                        w.wake();
                    }
                    return Some(lot);
                }
                if e.ferme {
                    return None;
                }
            }
            self.reveil.notified().await;
        }
    }

    /// L'envoi a échoué pour de bon : les écritures suivantes échouent.
    pub fn echec(&self, raison: String) {
        let mut e = self.etat.lock().unwrap();
        e.erreur = Some(raison);
        if let Some(w) = e.attente.take() {
            w.wake();
        }
    }
}

impl AsyncWrite for Ecrivain {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut e = self.etat.lock().unwrap();
        if let Some(r) = &e.erreur {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, r.clone())));
        }
        if e.ferme {
            return Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)));
        }
        let place = MAX_ATTENTE.saturating_sub(e.tampon.len());
        if place == 0 {
            e.attente = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let n = place.min(buf.len());
        e.tampon.extend_from_slice(&buf[..n]);
        drop(e);
        self.reveil.notify_one();
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.etat.lock().unwrap().ferme = true;
        self.reveil.notify_one();
        Poll::Ready(Ok(()))
    }
}

impl Drop for Ecrivain {
    fn drop(&mut self) {
        self.etat.lock().unwrap().ferme = true;
        self.reveil.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn ordre_doublons_et_perte() {
        let s = Sessions::default();
        let (id, mut lecteur) = s.ouvrir();
        assert!(session_valide(&id));
        assert_eq!(s.recevoir(&id, 0, Bytes::from_static(b"ab")).await, Reception::Ok);
        // Renvoi du même numéro : sans effet.
        assert_eq!(s.recevoir(&id, 0, Bytes::from_static(b"ab")).await, Reception::Ok);
        assert_eq!(s.recevoir(&id, 1, Bytes::from_static(b"cd")).await, Reception::Ok);
        // Numéro en avance : un envoi manque, le tunnel est perdu.
        assert_eq!(s.recevoir(&id, 3, Bytes::from_static(b"zz")).await, Reception::Perdu);
        assert_eq!(s.recevoir("inconnue", 0, Bytes::new()).await, Reception::Perdu);
        let mut v = [0u8; 4];
        lecteur.read_exact(&mut v).await.unwrap();
        assert_eq!(&v, b"abcd");
        s.fermer(&id);
        assert_eq!(s.recevoir(&id, 2, Bytes::from_static(b"ef")).await, Reception::Perdu);
    }

    #[tokio::test]
    async fn lots_et_fin() {
        let (mut e, d) = ecrivain();
        e.write_all(b"un").await.unwrap();
        e.write_all(b"deux").await.unwrap();
        assert_eq!(d.prochain().await.unwrap(), Bytes::from_static(b"undeux"));
        let gros = vec![1u8; MAX_CORPS + 10];
        e.write_all(&gros).await.unwrap();
        assert_eq!(d.prochain().await.unwrap().len(), MAX_CORPS);
        assert_eq!(d.prochain().await.unwrap().len(), 10);
        d.echec("perdu".into());
        assert!(e.write_all(b"x").await.is_err());
        drop(e);
        assert_eq!(d.prochain().await, None);
    }
}
