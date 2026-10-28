//! The share effects: publishing a canvas, a live room connection per
//! shared canvas, the updates handed to it, and what the rooms sent, told to
//! the app.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use specular_interact::{CanvasId, Event, ShareEvent, ShareLinks};

use super::runtime::{Runtime, ShellWindow};
use crate::share::publish::{self, Publish};
use crate::share::{Incoming, LiveLink, ShareLink};

/// The live room connection of each shared canvas, and the publishes in
/// flight.
#[derive(Debug, Default)]
pub(crate) struct Shares {
    live: HashMap<CanvasId, LiveLink>,
    publishing: Vec<(CanvasId, Receiver<Result<ShareLinks, String>>)>,
}

impl<W: ShellWindow> Runtime<W> {
    pub(super) fn share_publish(&mut self, canvas: CanvasId, title: String) {
        self.write_canvas(&canvas);
        let file = (self.files.as_ref()).and_then(|files| files.path_of(&self.app, &canvas));
        let answer = if let Some(file) = file {
            publish::start(Publish {
                file,
                title,
                server: publish::server(),
            })
        } else {
            let (answer, answered) = mpsc::channel();
            let _ = answer.send(Err("the canvas has no file to publish".to_owned()));
            answered
        };
        self.shares.publishing.push((canvas, answer));
    }

    pub(super) fn share_connect(&mut self, canvas: CanvasId, link: &str) {
        let started = ShareLink::parse(link).and_then(|link| {
            LiveLink::start(link).map_err(|error| format!("cannot start the share thread: {error}"))
        });
        match started {
            // A canvas connected again replaces its old connection.
            Ok(live) => drop(self.shares.live.insert(canvas, live)),
            Err(error) => tracing::warn!("the canvas is not shared: {error}"),
        }
    }

    pub(super) fn share_send(&mut self, canvas: &CanvasId, update: Vec<u8>) {
        if let Some(live) = self.shares.live.get(canvas) {
            live.send(update);
        } else {
            tracing::debug!("share update for {canvas}: {} bytes, no room", update.len());
        }
    }

    pub(super) fn share_disconnect(&mut self, canvas: &CanvasId) {
        self.shares.live.remove(canvas);
    }

    /// Whether a room or a publish may answer with no event to wake the
    /// loop.
    pub(super) fn sharing(&self) -> bool {
        !self.shares.live.is_empty() || !self.shares.publishing.is_empty()
    }

    /// Tells the app what every publish and room answered since the last
    /// turn.
    pub(super) fn take_share_notices(&mut self) {
        let mut events = Vec::new();
        self.shares.publishing.retain(|(canvas, answer)| {
            let canvas = canvas.clone();
            match answer.try_recv() {
                Ok(Ok(links)) => events.push(ShareEvent::Published { canvas, links }),
                Ok(Err(reason)) => {
                    tracing::warn!("the canvas was not published: {reason}");
                    events.push(ShareEvent::PublishFailed { canvas, reason });
                }
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => {
                    let reason = "publishing stopped".to_owned();
                    events.push(ShareEvent::PublishFailed { canvas, reason });
                }
            }
            false
        });
        let mut refused = Vec::new();
        for (canvas, live) in &self.shares.live {
            while let Some(incoming) = live.take() {
                let canvas = canvas.clone();
                events.push(match incoming {
                    Incoming::State(state) => ShareEvent::Connected { canvas, state },
                    Incoming::Update(update) => ShareEvent::Receive { canvas, update },
                    Incoming::Refused(reason) => {
                        refused.push(canvas.clone());
                        ShareEvent::Ended { canvas, reason }
                    }
                });
            }
        }
        for canvas in refused {
            self.shares.live.remove(&canvas);
        }
        for event in events {
            self.dispatch(Event::Share(event));
        }
    }
}
