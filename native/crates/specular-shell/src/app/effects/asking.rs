//! What is asked of hosted pages: sync, element attachment, inspect and region grabs.

use glam::Vec2;
use specular_core::{PageId, PageSource, PageSourceError};
use specular_doc::{EntityId, Rect};
use specular_interact::{Effect, PageRegion};

use super::super::runtime::{Runtime, ShellWindow};
use super::warn_on_error;
use crate::page_queries::css_rect;

impl<W: ShellWindow> Runtime<W> {
    /// Runs what a sync set asks of its pages (the capture of the entered
    /// page, a peer's candidates, a replayed pointer and the scroll) and
    /// what element attachment asks of them (the element under an item, and
    /// the ones to track).
    pub(super) fn run_sync(&mut self, effect: Effect) {
        match effect {
            Effect::CapturePage(page) => self.capture_page(page.as_ref()),
            Effect::AskCandidates {
                page,
                request,
                bundle,
            } => self.on_host(&page, |source, host| {
                source.query_candidates(host, &bundle, request)
            }),
            Effect::ReplayPointer { page, kind, point } => {
                self.on_host(&page, |source, host| {
                    source.replay_pointer(host, kind, point)
                });
            }
            Effect::AskScrollProgress(page) => {
                self.on_host(&page, |source, host| source.scroll_progress(host));
            }
            Effect::AskScrollRoom { page, at, toward } => {
                self.on_host(&page, |source, host| source.scroll_room(host, at, toward));
            }
            Effect::ScrollPage { page, progress } => {
                self.on_host(&page, |source, host| source.scroll_to(host, progress));
            }
            Effect::CaptureElement {
                page,
                request,
                point,
            } => self.on_host(&page, |source, host| {
                source.capture_element(host, point, request)
            }),
            Effect::TrackElements { page, selectors } => {
                self.on_host(&page, |source, host| {
                    source.track_elements(host, &selectors)
                });
            }
            _ => {}
        }
    }

    /// Asks something of the backend page that hosts `page`, if one does. A
    /// refusal is logged: the page may be closing or between documents.
    pub(super) fn on_host(
        &mut self,
        page: &EntityId,
        ask: impl FnOnce(&mut dyn PageSource, PageId) -> Result<(), PageSourceError>,
    ) {
        if let Some(host) = self.hosts.get(page) {
            warn_on_error(ask(self.source.as_mut(), host.page));
        }
    }

    /// Names the page whose hovers and clicks the backend reports.
    fn capture_page(&mut self, page: Option<&EntityId>) {
        let host = page.and_then(|page| self.hosts.get(page));
        warn_on_error(self.source.set_capture(host.map(|host| host.page)));
    }

    /// Asks the page for the element at `point`. The answer comes back
    /// through the page's events; a page that cannot be asked has none.
    pub(super) fn query_element(&mut self, page: &EntityId, point: Vec2) {
        let request = self.queries.ask_element(page.clone(), point);
        let asked = (self.hosts.get(page))
            .is_some_and(|host| self.source.query_element(host.page, point, request).is_ok());
        if !asked && let Some(answer) = self.queries.element_answer(request, None) {
            self.dispatch(answer);
        }
    }

    /// Asks the page for the node at `point` as the inspect tool reads it.
    /// A page that cannot be asked has none.
    pub(super) fn inspect_at(&mut self, page: &EntityId, point: Vec2, pick: bool) {
        let request = self.queries.ask_inspect(page.clone(), point, pick);
        let asked = (self.hosts.get(page))
            .is_some_and(|host| self.source.inspect_at(host.page, point, request).is_ok());
        if !asked && let Some(answer) = self.queries.inspect_answer(request, None) {
            self.dispatch(answer);
        }
    }

    /// Asks each page what the region grabbed in it. The answer goes to the
    /// app once every page has said; a page that cannot be asked grabbed
    /// nothing.
    pub(super) fn query_region_grab(&mut self, region: Rect, pages: &[PageRegion]) {
        let ids = pages.iter().map(|covered| covered.page.clone()).collect();
        let requests = self.queries.ask_grab(region, ids);
        for (covered, request) in pages.iter().zip(requests) {
            let rect = css_rect(covered.rect);
            let asked = self.hosts.get(&covered.page).is_some_and(|host| {
                (self.source.query_elements_in_rect(host.page, rect, request)).is_ok()
            });
            if !asked {
                self.queries.grab_answer(request, 0);
            }
        }
        self.answer_settled_grabs();
    }

    /// Asks each page what a region still being dragged has inside it. Each
    /// answers for itself; a page that cannot be asked is not answered for.
    pub(super) fn preview_region(&mut self, pages: &[PageRegion]) {
        for covered in pages {
            let Some(host) = self.hosts.get(&covered.page).map(|host| host.page) else {
                continue;
            };
            let request = self.queries.ask_preview(covered.page.clone());
            let rect = css_rect(covered.rect);
            if (self.source.query_elements_in_rect(host, rect, request)).is_err() {
                self.queries.preview_answer(request, &[]);
            }
        }
    }

    /// Tells the app about the regions every page has now answered for.
    pub(crate) fn answer_settled_grabs(&mut self) {
        for answer in self.queries.settled() {
            self.dispatch(answer);
        }
    }

    /// Answers what was still being asked of a page that will not answer.
    pub(crate) fn give_up_on_page(&mut self, page: &EntityId) {
        for answer in self.queries.give_up_on(page) {
            self.dispatch(answer);
        }
        self.answer_settled_grabs();
    }
}
