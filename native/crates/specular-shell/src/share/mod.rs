//! A canvas's connection to the live room of its share link (ADR 0050):
//! one thread that redeems the link, holds the room's websocket, and comes
//! back after a drop.
//!
//! The main thread hands updates in with [`LiveLink::send`] and takes what
//! the room sent with [`LiveLink::take`]. Updates handed in while the socket
//! is down wait and go out once it is back.

mod account;
mod link;
pub(crate) mod publish;
mod socket;
#[cfg(test)]
mod through_a_room;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

pub(crate) use self::link::ShareLink;
use self::socket::Ended;

/// What the room sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Incoming {
    /// The room's whole state, the first frame of each connection.
    State(Vec<u8>),
    /// One peer's update.
    Update(Vec<u8>),
    /// The room refused this link for good: revoked, or it cannot edit.
    Refused(String),
}

/// The longest wait between two tries to come back.
const MOST_BACKOFF: Duration = Duration::from_secs(30);

/// A live room connection, running until it is dropped.
#[derive(Debug)]
pub(crate) struct LiveLink {
    outgoing: Sender<Vec<u8>>,
    incoming: Receiver<Incoming>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LiveLink {
    /// Starts connecting to `link`'s room.
    pub(crate) fn start(link: ShareLink) -> std::io::Result<Self> {
        let (outgoing, to_send) = mpsc::channel();
        let (sent, incoming) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("share".to_owned())
            .spawn(move || run(&link, &to_send, &sent, &stopped))?;
        Ok(Self {
            outgoing,
            incoming,
            stop,
            thread: Some(thread),
        })
    }

    /// Sends `update` to the room, as soon as it is connected.
    pub(crate) fn send(&self, update: Vec<u8>) {
        if self.outgoing.send(update).is_err() {
            tracing::warn!("the share thread has stopped; an edit is not shared");
        }
    }

    /// One thing the room sent, if there is one.
    pub(crate) fn take(&self) -> Option<Incoming> {
        self.incoming.try_recv().ok()
    }
}

impl Drop for LiveLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::warn!("the share thread ended badly");
        }
    }
}

fn run(link: &ShareLink, to_send: &Receiver<Vec<u8>>, sent: &Sender<Incoming>, stop: &AtomicBool) {
    let mut backoff = Duration::from_secs(1);
    while !stop.load(Ordering::Relaxed) {
        match socket::session(link, to_send, sent, stop) {
            Ok(Ended::Stopped) => return,
            Ok(Ended::Refused(reason)) => {
                tracing::warn!("the share was refused: {reason}");
                let _ = sent.send(Incoming::Refused(reason));
                return;
            }
            Ok(Ended::Dropped) => {
                tracing::info!("the share's connection dropped; coming back");
                backoff = Duration::from_secs(1);
            }
            Err(error) => tracing::warn!("the share could not connect: {error:#}"),
        }
        sleep_unless_stopped(backoff, stop);
        backoff = (backoff * 2).min(MOST_BACKOFF);
    }
}

fn sleep_unless_stopped(time: Duration, stop: &AtomicBool) {
    let step = Duration::from_millis(100);
    let mut slept = Duration::ZERO;
    while slept < time && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(step);
        slept += step;
    }
}
