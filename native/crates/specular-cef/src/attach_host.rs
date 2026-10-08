//! What element attachment (ADR 0032) asks of a hosted page: the element an
//! item should follow, and where the elements items follow have moved to.
//! Both go over the page's own devtools channel, with the messages built
//! and read in [`attach_query`].

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_core::{PageId, PageSourceError};

use crate::attach_query;
use crate::devtools::Asked;
use crate::source::{CefPageSource, refused};

/// How long a tracking poll may go unanswered before another is sent. The
/// page answers within two seconds when nothing has moved.
const POLL_LOST: Duration = Duration::from_secs(4);
/// The least time between tracking polls, so a page that cannot answer (it
/// is between documents) is not asked on every turn.
const POLL_SPACING: Duration = Duration::from_millis(100);

/// The elements one page reports the place of, and when it was last asked.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tracking {
    selectors: Vec<String>,
    polled: Option<Instant>,
}

impl CefPageSource {
    pub(crate) fn ask_capture(
        &self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        (entry.devtools)
            .send(&entry.host, Asked::Captured(request), |id| {
                attach_query::capture_message(id, point.x, point.y)
            })
            .then_some(())
            .ok_or_else(|| refused("element capture"))
    }

    pub(crate) fn track(
        &mut self,
        page: PageId,
        selectors: &[String],
    ) -> Result<(), PageSourceError> {
        let entry = self.entry_mut(page)?;
        // A poll in flight answers for the old set; the next one, sent as
        // soon as it is due, carries the new.
        entry.tracking = Tracking {
            selectors: selectors.to_vec(),
            polled: None,
        };
        self.poll_tracking();
        Ok(())
    }

    /// Asks every page with tracked elements where they are, unless it is
    /// still thinking about the last time it was asked. Called on every
    /// pump.
    pub(crate) fn poll_tracking(&mut self) {
        for entry in self.pages.values_mut() {
            if entry.tracking.selectors.is_empty() {
                continue;
            }
            let waited = entry.tracking.polled.map(|at| at.elapsed());
            let asking = entry.devtools.is_asking(Asked::Places);
            let due = match waited {
                None => !asking,
                Some(waited) if asking => waited > POLL_LOST,
                Some(waited) => waited > POLL_SPACING,
            };
            let selectors = &entry.tracking.selectors;
            if due
                && (entry.devtools).send(&entry.host, Asked::Places, |id| {
                    attach_query::track_message(id, selectors)
                })
            {
                entry.tracking.polled = Some(Instant::now());
            }
        }
    }
}
