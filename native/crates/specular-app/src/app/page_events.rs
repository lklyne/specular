//! What the page backend reports: frames for the compositor, and the few
//! things the app acts on.

use specular_core::{PageEvent, PageId};
use specular_doc::EntityId;
use specular_interact::{Event, PageNotice};

use super::Shell;

impl Shell {
    /// Logs what a page reported, tells the app what it acts on, and hands
    /// the event to the compositor for its frames.
    pub(super) fn handle_page_event(&mut self, event: PageEvent) {
        self.latency.observe(&event);
        let notice = match &event {
            PageEvent::Loaded { page, http_status } => {
                tracing::info!(%page, http_status, "page loaded");
                Some((
                    *page,
                    PageNotice::Loaded {
                        http_status: *http_status,
                    },
                ))
            }
            PageEvent::Crashed { page, reason } => {
                tracing::error!(%page, reason, "page host crashed");
                Some((
                    *page,
                    PageNotice::Crashed {
                        reason: reason.clone(),
                    },
                ))
            }
            PageEvent::ImeCompositionBounds { page, bounds } => {
                Some((*page, PageNotice::ImeCompositionBounds(*bounds)))
            }
            PageEvent::Frame(_)
            | PageEvent::FrameDropped { .. }
            | PageEvent::PopupVisibility { .. }
            | PageEvent::PopupRect { .. } => None,
        };
        if let Some((host, notice)) = notice
            && let Some(page) = self.entity_of(host)
        {
            self.dispatch(Event::Page { page, notice });
        }
        if let Some(gpu) = self.gpu.as_mut()
            && let Err(error) = gpu.compositor.handle_page_event(event)
        {
            tracing::warn!("{error}");
        }
    }

    /// The page entity a backend page is hosting.
    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|(_, host)| host.page == page)?;
        Some(entity.clone())
    }
}
