//! What a sync set asks of a hosted page (ADR 0027, ADR 0030): its scroll
//! as a fraction, the capture of the entered page's hovers and clicks, a
//! peer's candidate elements, and the replay of input on a peer. All of it
//! goes over the page's own devtools channel, with the messages built and
//! read in [`dom_query`] and [`sync_query`].

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_core::{LocatorBundle, PageId, PageSourceError, PointKind};

use crate::devtools::Asked;
use crate::source::{CefPageSource, refused};
use crate::{dom_query, sync_query};

/// How long a capture poll may go unanswered before another is sent. The
/// page answers within two seconds when it has nothing to tell.
const POLL_LOST: Duration = Duration::from_secs(4);
/// The least time between capture polls, so a page that cannot answer (it
/// is between documents) is not asked on every turn.
const POLL_SPACING: Duration = Duration::from_millis(100);

/// The page being captured and when it was last asked.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Capture {
    page: PageId,
    polled: Option<Instant>,
}

impl CefPageSource {
    pub(crate) fn ask_scroll_progress(&self, page: PageId) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        (entry.devtools)
            .send(
                &entry.host,
                Asked::ScrollProgress,
                dom_query::scroll_progress_message,
            )
            .then_some(())
            .ok_or_else(|| refused("scroll progress"))
    }

    pub(crate) fn scroll_to_progress(
        &self,
        page: PageId,
        progress: Vec2,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        (entry.devtools)
            .send(&entry.host, Asked::Done, |id| {
                dom_query::scroll_to_message(id, progress.x, progress.y)
            })
            .then_some(())
            .ok_or_else(|| refused("scroll to"))
    }

    pub(crate) fn capture(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        if let Some(page) = page {
            self.entry(page)?;
        }
        self.capture = page.map(|page| Capture { page, polled: None });
        self.poll_capture();
        Ok(())
    }

    /// Asks the captured page what it was pointed at, unless it is still
    /// thinking about the last time it was asked. Called on every pump.
    pub(crate) fn poll_capture(&mut self) {
        let Some(capture) = self.capture else {
            return;
        };
        let Ok(entry) = self.entry(capture.page) else {
            self.capture = None;
            return;
        };
        let waited = capture.polled.map(|at| at.elapsed());
        let asking = entry.devtools.is_asking(Asked::Pointed);
        let due = match waited {
            None => true,
            Some(waited) if asking => waited > POLL_LOST,
            Some(waited) => waited > POLL_SPACING,
        };
        if due && (entry.devtools).send(&entry.host, Asked::Pointed, sync_query::capture_message) {
            self.capture = Some(Capture {
                polled: Some(Instant::now()),
                ..capture
            });
        }
    }

    pub(crate) fn ask_candidates(
        &self,
        page: PageId,
        bundle: &LocatorBundle,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        (entry.devtools)
            .send(&entry.host, Asked::Candidates(request), |id| {
                sync_query::candidates_message(id, bundle)
            })
            .then_some(())
            .ok_or_else(|| refused("candidates"))
    }

    pub(crate) fn replay(
        &self,
        page: PageId,
        kind: PointKind,
        point: Vec2,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        for mut message in sync_query::replay_messages(kind, point.x, point.y) {
            let sent = entry.devtools.send(&entry.host, Asked::Done, |id| {
                message["id"] = id.into();
                message.to_string().into_bytes()
            });
            if !sent {
                return Err(refused("replayed input"));
            }
        }
        Ok(())
    }
}
