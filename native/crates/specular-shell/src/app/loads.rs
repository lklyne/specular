//! What the runtime has asked for and not yet got: images being decoded,
//! Documents being read, and pages loading or not yet painted since they
//! loaded. A window draws whatever is in; a headless run waits on these
//! before it checks or draws.

use std::collections::HashSet;

use specular_core::{PageEvent, PageId};
use specular_interact::ImageKey;

/// The loads in flight.
#[derive(Debug, Default)]
pub(crate) struct Loads {
    pub(crate) images: HashSet<ImageKey>,
    pub(crate) notes: HashSet<String>,
    /// The pages whose current document has not finished loading.
    pub(crate) pages: HashSet<PageId>,
    /// The pages that have not painted since their document loaded.
    pub(crate) unpainted: HashSet<PageId>,
}

impl Loads {
    /// Follows a page through its load and its first paint after.
    pub(crate) fn page_event(&mut self, event: &PageEvent) {
        match event {
            PageEvent::Loading {
                page,
                loading: true,
                ..
            } => {
                self.pages.insert(*page);
                self.unpainted.insert(*page);
            }
            PageEvent::Loaded { page, .. } | PageEvent::Crashed { page, .. } => {
                self.pages.remove(page);
            }
            PageEvent::Frame(frame) if !self.pages.contains(&frame.page) => {
                self.unpainted.remove(&frame.page);
            }
            _ => {}
        }
    }

    /// Forgets a page that is no longer hosted.
    pub(crate) fn page_closed(&mut self, page: PageId) {
        self.pages.remove(&page);
        self.unpainted.remove(&page);
    }

    /// Whether the pages have loaded and painted.
    pub(crate) fn pages_ready(&self) -> bool {
        self.pages.is_empty() && self.unpainted.is_empty()
    }

    /// Whether the images and Documents asked for are in.
    pub(crate) fn files_ready(&self) -> bool {
        self.images.is_empty() && self.notes.is_empty()
    }
}
