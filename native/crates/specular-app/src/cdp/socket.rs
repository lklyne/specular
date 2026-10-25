//! The server: an accept thread, and per connection a reader thread and a
//! writer thread.

use std::collections::HashMap;
use std::io::{BufReader, Write as _};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::frame::{Decoder, Incoming, Opcode, Role, encode};
use super::handshake;

/// How long a client has to finish its upgrade request.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// What the server does with its connections. Called on connection threads.
pub(crate) trait WsHandler: Send + Sync + 'static {
    /// A client asked to upgrade at `path` (the request target, e.g. `/cdp/page/abc`).
    /// `Some(id)` accepts it under that connection id; `None` refuses with a 404.
    /// `out` sends to this client and may be kept for as long as the connection lives.
    fn open(&self, path: &str, out: WsSender) -> Option<u64>;
    /// One whole text (or binary, lossily decoded as UTF-8) message from the client.
    fn message(&self, connection: u64, text: String);
    /// The connection ended, for any reason. Called exactly once per accepted connection.
    fn closed(&self, connection: u64);
}

#[derive(Debug)]
enum Outgoing {
    Text(String),
    Pong(Vec<u8>),
    Close,
}

/// Sends to one client. Cheap to clone; sending never blocks on the socket.
#[derive(Debug, Clone)]
pub(crate) struct WsSender {
    queue: Sender<Outgoing>,
}

impl WsSender {
    /// Queues one text message. `false` once the connection has gone.
    pub(crate) fn send(&self, text: String) -> bool {
        self.queue.send(Outgoing::Text(text)).is_ok()
    }

    /// Sends a close frame and ends the connection.
    pub(crate) fn close(&self) {
        // A connection that is already gone has nothing left to close.
        let _ = self.queue.send(Outgoing::Close);
    }
}

/// Sockets of live connections by an internal number, so `stop` can shut
/// them down.
type Live = Arc<Mutex<HashMap<u64, TcpStream>>>;

fn lock(live: &Live) -> std::sync::MutexGuard<'_, HashMap<u64, TcpStream>> {
    live.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The listening server.
#[derive(Debug)]
pub(crate) struct WsServer {
    port: u16,
    stopping: Arc<AtomicBool>,
    live: Live,
    accept: Option<JoinHandle<()>>,
}

impl WsServer {
    /// Listens on 127.0.0.1 on a port the system picks.
    pub(crate) fn spawn(handler: Arc<dyn WsHandler>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let stopping = Arc::new(AtomicBool::new(false));
        let live = Live::default();
        let accept = thread::Builder::new().name("specular-cdp".into()).spawn({
            let (stopping, live) = (stopping.clone(), live.clone());
            move || accept_loop(&listener, &handler, &stopping, &live)
        })?;
        Ok(Self {
            port,
            stopping,
            live,
            accept: Some(accept),
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    /// Stops accepting, closes every connection, joins the accept thread. Idempotent. Also run on Drop.
    pub(crate) fn stop(&mut self) {
        let Some(accept) = self.accept.take() else {
            return;
        };
        self.stopping.store(true, Ordering::SeqCst);
        // A throwaway connection wakes the blocked `accept`.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        let _ = accept.join();
        for stream in lock(&self.live).values() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for WsServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn accept_loop(
    listener: &TcpListener,
    handler: &Arc<dyn WsHandler>,
    stopping: &Arc<AtomicBool>,
    live: &Live,
) {
    let next = AtomicU64::new(0);
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let slot = next.fetch_add(1, Ordering::Relaxed);
        let Ok(handle) = stream.try_clone() else {
            continue;
        };
        lock(live).insert(slot, handle);
        let (handler, live_for_thread) = (handler.clone(), live.clone());
        let spawned = thread::Builder::new()
            .name("specular-cdp-conn".into())
            .spawn(move || {
                serve(stream, &handler);
                lock(&live_for_thread).remove(&slot);
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "could not start a CDP connection thread");
            lock(live).remove(&slot);
        }
    }
}

/// One connection, from its upgrade request to its end.
fn serve(mut stream: TcpStream, handler: &Arc<dyn WsHandler>) {
    // Without this a request sent in small packets waits on Nagle's timer.
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
    let Ok(head) = handshake::read_head(&mut stream) else {
        return;
    };
    let upgrade = match handshake::parse_request(&head) {
        Ok(upgrade) => upgrade,
        Err(status) => {
            let _ = stream.write_all(handshake::refusal(status).as_bytes());
            return;
        }
    };
    let _ = stream.set_read_timeout(None);
    let Ok(write_half) = stream.try_clone() else {
        return;
    };
    let (queue, outgoing) = mpsc::channel();
    let Some(id) = handler.open(
        &upgrade.path,
        WsSender {
            queue: queue.clone(),
        },
    ) else {
        let _ = stream.write_all(handshake::refusal(404).as_bytes());
        return;
    };
    let writer = if stream
        .write_all(handshake::switching_protocols(&upgrade.key).as_bytes())
        .is_ok()
    {
        thread::Builder::new()
            .name("specular-cdp-write".into())
            .spawn(move || write_loop(write_half, &outgoing))
            .ok()
    } else {
        None
    };
    if writer.is_some() {
        read_loop(&stream, handler.as_ref(), id, &queue);
    }
    // The writer sends the close frame, then shuts the socket down.
    let _ = queue.send(Outgoing::Close);
    if let Some(writer) = writer {
        let _ = writer.join();
    } else {
        let _ = stream.shutdown(Shutdown::Both);
    }
    handler.closed(id);
}

fn read_loop(stream: &TcpStream, handler: &dyn WsHandler, id: u64, queue: &Sender<Outgoing>) {
    let mut from = BufReader::new(stream);
    let mut decoder = Decoder::new(Role::Server);
    loop {
        match decoder.read(&mut from) {
            Ok(Incoming::Data(bytes)) => {
                handler.message(id, String::from_utf8_lossy(&bytes).into_owned());
            }
            Ok(Incoming::Ping(payload)) => {
                let _ = queue.send(Outgoing::Pong(payload));
            }
            Ok(Incoming::Pong(_)) => {}
            Ok(Incoming::Close) | Err(_) => return,
        }
    }
}

/// Owns the socket's write side, so frames never interleave. Shuts the socket
/// down on the way out, which also ends the reader.
fn write_loop(mut stream: TcpStream, outgoing: &Receiver<Outgoing>) {
    while let Ok(item) = outgoing.recv() {
        let (frame, last) = match item {
            Outgoing::Text(text) => (encode(Opcode::Text, text.as_bytes(), None), false),
            Outgoing::Pong(payload) => (encode(Opcode::Pong, &payload, None), false),
            Outgoing::Close => (encode(Opcode::Close, &[], None), true),
        };
        if stream.write_all(&frame).is_err() || last {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}
