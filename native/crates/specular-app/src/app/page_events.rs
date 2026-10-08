//! What the page backend reports: frames for the compositor, answers to the
//! questions the app asked, and the few things the app acts on.

use specular_core::{PageEvent, PageId};
use specular_doc::EntityId;
use specular_interact::Event;

use super::runtime::{Runtime, ShellWindow};
use crate::page_notice::notice_of;

impl<W: ShellWindow> Runtime<W> {
    /// Logs what a page reported, tells the app what it acts on, and hands
    /// the event to the compositor for its frames.
    pub(super) fn handle_page_event(&mut self, event: PageEvent) {
        self.latency.observe(&event);
        match &event {
            PageEvent::Loaded { page, http_status } => {
                tracing::info!(%page, http_status, "page loaded");
            }
            PageEvent::Crashed { page, reason } => {
                tracing::error!(%page, reason, "page host crashed");
                if let Some(entity) = self.entity_of(*page) {
                    self.give_up_on_page(&entity);
                }
            }
            PageEvent::ElementAt {
                request, element, ..
            } => {
                if let Some(answer) = self.queries.element_answer(*request, element.clone()) {
                    self.dispatch(answer);
                }
            }
            PageEvent::ElementsInRect { request, count, .. } => {
                self.queries.grab_answer(*request, *count);
                self.answer_settled_grabs();
            }
            _ => {}
        }
        if let Some((host, notice)) = notice_of(&event, self.source.devtools_port())
            && let Some(page) = self.entity_of(host)
        {
            tracing::debug!(%page, ?notice, "page notice");
            self.dispatch(Event::Page { page, notice });
        }
        if let Some(gpu) = self.gpu.as_mut()
            && let Err(error) = gpu.compositor_mut().handle_page_event(event)
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
