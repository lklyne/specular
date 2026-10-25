//! What a scripted run needs of the runtime that a person at the window
//! does not: a clipboard that is not the system's, the text of the canvas
//! without waiting for an autosave, the effects of the last step, and a
//! wait for what is still loading.

use std::time::{Duration, Instant};

use anyhow::Context as _;
use specular_interact::{App, Effect, Event, EventSink};

use super::runtime::{Runtime, ShellWindow};
use crate::persist::canvas_text;

/// How long a headless run waits for images, Documents and real pages
/// before going on without them.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

/// How long real pages are left to paint once they have all loaded.
const PAGE_SETTLE: Duration = Duration::from_millis(300);

/// The clipboard of a scripted run: the text last copied, if any was.
#[derive(Debug, Clone, Default)]
pub(crate) struct HeldClipboard(pub(crate) Option<String>);

impl<W: ShellWindow> Runtime<W> {
    /// Keeps the clipboard in memory from now on, holding `text`. A copy or
    /// a cut no longer writes the system clipboard and a paste no longer
    /// reads it, so a script cannot paste what the person at the machine
    /// last copied, or replace it.
    pub fn script_clipboard(&mut self, text: Option<String>) {
        self.scripted_clipboard = Some(HeldClipboard(text));
    }

    /// The `.canvas` text an autosave of the canvas showing would write now.
    pub fn canvas_text(&self) -> anyhow::Result<String> {
        let app = self.app();
        let canvas = (app.canvas_to_save(&app.space().active().id))
            .context("the canvas showing is not in the space")?;
        canvas_text(&canvas).context("writing the canvas as text")
    }
}

/// A script's hands send their events through the runtime, which runs every
/// effect as it does for a window.
impl<W: ShellWindow> EventSink for Runtime<W> {
    fn app(&self) -> &App {
        &self.app
    }

    fn send(&mut self, event: Event) {
        self.dispatch(event);
    }
}

impl<W: ShellWindow> Runtime<W> {
    /// Starts keeping the effects `update` returns, from an empty list.
    pub(crate) fn clear_sent(&mut self) {
        self.sent = Some(Vec::new());
    }

    /// The effects kept since [`clear_sent`](Self::clear_sent).
    pub(crate) fn sent(&self) -> &[Effect] {
        self.sent.as_deref().unwrap_or_default()
    }

    /// The error that ended the run, once.
    pub(crate) fn take_error(&mut self) -> Option<anyhow::Error> {
        self.error.take()
    }

    /// Takes in what the pages, the decode thread and the note thread have
    /// finished, until each image and Document asked for is in. With `live`
    /// pages, which load and paint in their own time, each page must also
    /// have loaded, answered what it was asked and painted, and is then
    /// given a moment more: the frame after a load is rarely the last.
    pub(crate) fn settle(&mut self, live: bool) {
        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut ready_since = None;
        loop {
            self.take_page_events();
            while self.take_loaded_image() {}
            self.take_read_notes();
            let pages_ready = !live || (self.loads.pages_ready() && self.queries.is_idle());
            if self.loads.files_ready() && pages_ready {
                if !live {
                    return;
                }
                let now = Instant::now();
                if now.duration_since(*ready_since.get_or_insert(now)) >= PAGE_SETTLE {
                    return;
                }
            } else {
                ready_since = None;
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    images = self.loads.images.len(),
                    documents = self.loads.notes.len(),
                    pages_loading = self.loads.pages.len(),
                    pages_unpainted = self.loads.unpainted.len(),
                    "going on without loads that never finished"
                );
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Lets real pages run for `time`, passing on what they report: a
    /// script's `wait` is their only chance to animate, scroll or follow a
    /// link.
    pub(crate) fn run_pages_for(&mut self, time: Duration) {
        let until = Instant::now() + time.min(LOAD_TIMEOUT);
        while Instant::now() < until {
            self.take_page_events();
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Stops the page backend at the end of a run with no loop to turn.
    pub(crate) fn shut_down(&mut self) {
        self.exit();
        self.source.shutdown();
    }
}
