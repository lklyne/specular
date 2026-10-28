//! One connection to a live room: redeem the link, open the websocket,
//! then pass frames both ways until it drops or the link is dropped.

use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use super::{Incoming, ShareLink};

/// The room's frame tags (`server/src/live-room.ts`).
const STATE: u8 = 0;
const UPDATE: u8 = 1;

/// How long a read waits before the thread looks for updates to send.
const READ_WAIT: Duration = Duration::from_millis(30);
/// How often an idle socket is pinged, under the proxies' idle timeouts.
const PING_EVERY: Duration = Duration::from_secs(30);

/// How a connection ended.
pub(super) enum Ended {
    /// The link was dropped.
    Stopped,
    /// The socket went away; try again.
    Dropped,
    /// The room will not have this link, and trying again will not help.
    Refused(String),
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

pub(super) fn session(
    link: &ShareLink,
    to_send: &Receiver<Vec<u8>>,
    sent: &Sender<Incoming>,
    stop: &AtomicBool,
) -> anyhow::Result<Ended> {
    let token = match redeem(link) {
        Ok(token) => token,
        Err(Redeem::Refused(reason)) => return Ok(Ended::Refused(reason)),
        Err(Redeem::Failed(error)) => return Err(error),
    };
    let (mut socket, _) = tungstenite::connect(link.live_url(&token)).context("open the room")?;
    set_read_wait(&mut socket)?;
    tracing::info!(doc = link.doc, "connected to the live room");
    let mut pinged = Instant::now();
    loop {
        if stop.load(Ordering::Relaxed) {
            let _ = socket.close(None);
            let _ = socket.flush();
            return Ok(Ended::Stopped);
        }
        loop {
            match to_send.try_recv() {
                Ok(update) => socket.send(Message::binary(framed(&update)))?,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(Ended::Stopped),
            }
        }
        if pinged.elapsed() >= PING_EVERY {
            socket.send(Message::Ping(Vec::new().into()))?;
            pinged = Instant::now();
        }
        match socket.read() {
            Ok(Message::Binary(bytes)) => {
                let incoming = match bytes.split_first() {
                    Some((&STATE, state)) => Incoming::State(state.to_vec()),
                    Some((&UPDATE, update)) => Incoming::Update(update.to_vec()),
                    _ => continue,
                };
                if sent.send(incoming).is_err() {
                    return Ok(Ended::Stopped);
                }
            }
            Ok(Message::Close(frame)) => {
                let refused = frame
                    .as_ref()
                    .filter(|frame| frame.code == CloseCode::Library(4003));
                if let Some(frame) = refused {
                    return Ok(Ended::Refused(frame.reason.to_string()));
                }
                return Ok(Ended::Dropped);
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(Ended::Dropped);
            }
            Err(error) => {
                tracing::info!("the room's socket failed: {error}");
                return Ok(Ended::Dropped);
            }
        }
    }
}

fn framed(update: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(update.len() + 1);
    frame.push(UPDATE);
    frame.extend_from_slice(update);
    frame
}

/// Reads give up after [`READ_WAIT`], so one thread can both read and send.
fn set_read_wait(socket: &mut Socket) -> anyhow::Result<()> {
    let stream = match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => stream.get_mut(),
        _ => return Err(anyhow!("an unknown kind of socket")),
    };
    stream.set_read_timeout(Some(READ_WAIT))?;
    Ok(())
}

enum Redeem {
    Refused(String),
    Failed(anyhow::Error),
}

/// Trades the link token for a connection token, which lasts an hour; a
/// connection made after that redeems again.
fn redeem(link: &ShareLink) -> Result<String, Redeem> {
    let body = serde_json::json!({ "token": link.token }).to_string();
    let answer = ureq::post(format!("{}/redeem", link.server))
        .content_type("application/json")
        .send(body.as_str());
    let mut answer = match answer {
        Ok(answer) => answer,
        Err(ureq::Error::StatusCode(401 | 403 | 404)) => {
            return Err(Redeem::Refused("the link is revoked or wrong".to_owned()));
        }
        Err(error) => return Err(Redeem::Failed(anyhow!("redeem the link: {error}"))),
    };
    let text = (answer.body_mut().read_to_string())
        .map_err(|error| Redeem::Failed(anyhow!("read the redeemed token: {error}")))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| Redeem::Failed(error.into()))?;
    (value.get("token").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .ok_or_else(|| Redeem::Failed(anyhow!("the server gave no token")))
}
