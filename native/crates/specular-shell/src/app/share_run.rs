//! The share effects: a live room connection per shared canvas, the
//! updates handed to it, and what the rooms sent, told to the app.

use std::collections::HashMap;

use specular_interact::{CanvasId, Event, ShareEvent};

use super::runtime::{Runtime, ShellWindow};
use crate::share::{Incoming, LiveLink, ShareLink};

/// The live room connection of each shared canvas.
#[derive(Debug, Default)]
pub(crate) struct Shares(HashMap<CanvasId, LiveLink>);

impl<W: ShellWindow> Runtime<W> {
    pub(super) fn share_connect(&mut self, canvas: CanvasId, link: &str) {
        let started = ShareLink::parse(link).and_then(|link| {
            LiveLink::start(link).map_err(|error| format!("cannot start the share thread: {error}"))
        });
        match started {
            // A canvas connected again replaces its old connection.
            Ok(live) => drop(self.shares.0.insert(canvas, live)),
            Err(error) => tracing::warn!("the canvas is not shared: {error}"),
        }
    }

    pub(super) fn share_send(&mut self, canvas: &CanvasId, update: Vec<u8>) {
        if let Some(live) = self.shares.0.get(canvas) {
            live.send(update);
        } else {
            tracing::debug!("share update for {canvas}: {} bytes, no room", update.len());
        }
    }

    pub(super) fn share_disconnect(&mut self, canvas: &CanvasId) {
        self.shares.0.remove(canvas);
    }

    /// Whether a room may send something with no event to wake the loop.
    pub(super) fn sharing(&self) -> bool {
        !self.shares.0.is_empty()
    }

    /// Tells the app what every room sent since the last turn.
    pub(super) fn take_share_notices(&mut self) {
        let mut events = Vec::new();
        let mut refused = Vec::new();
        for (canvas, live) in &self.shares.0 {
            while let Some(incoming) = live.take() {
                let canvas = canvas.clone();
                events.push(match incoming {
                    Incoming::State(state) => ShareEvent::Connected { canvas, state },
                    Incoming::Update(update) => ShareEvent::Receive { canvas, update },
                    Incoming::Refused(_) => {
                        refused.push(canvas);
                        continue;
                    }
                });
            }
        }
        for canvas in refused {
            self.shares.0.remove(&canvas);
        }
        for event in events {
            self.dispatch(Event::Share(event));
        }
    }
}
