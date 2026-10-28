//! A CEF-free [`PageSource`] that paints animated CPU frames.
//!
//! It lets the app, compositor and bench run on any machine (including CI
//! without a GPU or CEF). Its frames are [`PageFrame::Cpu`] and therefore
//! non-representative: use it to exercise the pipeline, never to compare
//! against Electron.

mod cdp;
mod grid;
mod page;

use self::grid::{element_scrolled, inspected_scrolled, synthetic_capture, synthetic_elements_in};
use self::page::{SyntheticPage, paint, title_of};

use std::collections::BTreeMap;
use web_time::{Duration, Instant};

use glam::Vec2;

use crate::frame::{FrameEvent, FrameLayer, PageFrame};
use crate::geometry::{CssRect, CssSize};
use crate::input::InputEvent;
use crate::locator::LocatorBundle;
use crate::page::{PageId, PageSpec, validate_texture_scale, validate_viewport};
use crate::source::{DevtoolsSink, PageEvent, PageNav, PageSource, PageSourceError, PointKind};

/// Synthetic page backend; see the module docs.
#[derive(Default)]
pub struct SyntheticPageSource {
    pages: BTreeMap<PageId, SyntheticPage>,
    next_id: u64,
    focused: Option<PageId>,
    pending: Vec<PageEvent>,
    /// Devtools messages on their way to the sink, sent on the next pump as
    /// a browser's arrive: after the call that caused them returned.
    devtools_out: Vec<(PageId, String)>,
    devtools_sink: Option<DevtoolsSink>,
}

impl std::fmt::Debug for SyntheticPageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyntheticPageSource")
            .field("pages", &self.pages)
            .field("focused", &self.focused)
            .finish_non_exhaustive()
    }
}

impl SyntheticPageSource {
    /// Creates an empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// [`pump`](PageSource::pump) with an explicit clock, for deterministic tests.
    pub(crate) fn pump_at(&mut self, now: Instant) {
        self.flush_devtools();
        for (&id, page) in &mut self.pages {
            if !page.painting || page.next_paint.is_some_and(|due| now < due) {
                continue;
            }
            let interval = Duration::from_secs(1) / page.spec.frame_rate.max(1);
            page.next_paint = Some(now + interval);
            page.frames_painted += 1;
            let frame = paint(&page.spec, page.frames_painted);
            self.pending.push(PageEvent::Frame(FrameEvent {
                page: id,
                layer: FrameLayer::View,
                viewport: page.spec.viewport,
                frame: PageFrame::Cpu(frame),
                produced_at: now,
            }));
        }
    }

    /// Queues what a page reports when it shows the entry `at` of its
    /// history: the load starting, the address, the title, the scroll back
    /// at the top, and the load ending.
    fn show(&mut self, id: PageId) -> Result<(), PageSourceError> {
        let page = self.page_mut(id)?;
        page.scroll = Vec2::ZERO;
        page.next_paint = None;
        let url = page.history[page.at].clone();
        let (can_go_back, can_go_forward) = (page.at > 0, page.at + 1 < page.history.len());
        // Another document: what was tracked is found again in it.
        let places = page.places(id);
        if page.page_events {
            let navigated = cdp::navigated_events(&url);
            (self.devtools_out).extend(navigated.into_iter().map(|event| (id, event)));
        }
        let loading = |loading| PageEvent::Loading {
            page: id,
            loading,
            can_go_back,
            can_go_forward,
        };
        self.pending.extend([
            loading(true),
            PageEvent::Title {
                page: id,
                title: title_of(&url),
            },
            PageEvent::Url { page: id, url },
            PageEvent::Scrolled {
                page: id,
                offset: Vec2::ZERO,
            },
            PageEvent::Loaded {
                page: id,
                http_status: 200,
            },
            loading(false),
        ]);
        self.pending.extend(places);
        Ok(())
    }

    fn page(&self, page: PageId) -> Result<&SyntheticPage, PageSourceError> {
        self.pages
            .get(&page)
            .ok_or(PageSourceError::UnknownPage(page))
    }

    fn page_mut(&mut self, page: PageId) -> Result<&mut SyntheticPage, PageSourceError> {
        self.pages
            .get_mut(&page)
            .ok_or(PageSourceError::UnknownPage(page))
    }
}

impl PageSource for SyntheticPageSource {
    fn name(&self) -> &'static str {
        "synthetic"
    }

    fn create_page(&mut self, spec: &PageSpec) -> Result<PageId, PageSourceError> {
        spec.validate()?;
        self.next_id += 1;
        let id = PageId(self.next_id);
        self.pages.insert(
            id,
            SyntheticPage {
                spec: spec.clone(),
                painting: true,
                frames_painted: 0,
                next_paint: None,
                history: vec![spec.url.clone()],
                at: 0,
                scroll: Vec2::ZERO,
                page_events: false,
                tracked: Vec::new(),
            },
        );
        self.show(id)?;
        Ok(id)
    }

    fn set_viewport(&mut self, page: PageId, viewport: CssSize) -> Result<(), PageSourceError> {
        let entry = self.page_mut(page)?;
        validate_viewport(viewport)?;
        entry.spec.viewport = viewport;
        entry.next_paint = None;
        Ok(())
    }

    fn set_texture_scale(&mut self, page: PageId, scale: f32) -> Result<(), PageSourceError> {
        let entry = self.page_mut(page)?;
        validate_texture_scale(scale)?;
        entry.spec.texture_scale = scale;
        entry.next_paint = None;
        Ok(())
    }

    fn set_frame_rate(&mut self, page: PageId, fps: u32) -> Result<(), PageSourceError> {
        self.page_mut(page)?.spec.frame_rate = fps;
        Ok(())
    }

    fn set_painting(&mut self, page: PageId, painting: bool) -> Result<(), PageSourceError> {
        self.page_mut(page)?.painting = painting;
        Ok(())
    }

    fn close_page(&mut self, page: PageId) -> Result<(), PageSourceError> {
        self.pages
            .remove(&page)
            .map(|_| ())
            .ok_or(PageSourceError::UnknownPage(page))?;
        if self.focused == Some(page) {
            self.focused = None;
        }
        Ok(())
    }

    fn set_focus(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        if let Some(id) = page {
            self.page_mut(id)?;
        }
        self.focused = page;
        Ok(())
    }

    fn send_input(&mut self, page: PageId, event: &InputEvent) -> Result<(), PageSourceError> {
        // Input forces the next pump to repaint, standing in for the DOM
        // reacting so input-to-paint latency is measurable end to end.
        let entry = self.page_mut(page)?;
        entry.next_paint = None;
        // A wheel scrolls the document, as a browser's does: a positive
        // delta is content moving down, so the offset shrinks.
        if let InputEvent::Wheel(wheel) = event {
            let next = (entry.scroll - wheel.delta).clamp(Vec2::ZERO, entry.max_scroll());
            if next != entry.scroll {
                entry.scroll = next;
                self.pending
                    .push(PageEvent::Scrolled { page, offset: next });
            }
        }
        Ok(())
    }

    fn set_capture(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        // A synthetic page has no elements to point at, so there is
        // nothing to report.
        page.map_or(Ok(()), |page| self.page(page).map(|_| ()))
    }

    fn query_candidates(
        &mut self,
        page: PageId,
        _bundle: &LocatorBundle,
        request: u64,
    ) -> Result<(), PageSourceError> {
        self.page(page)?;
        self.pending.push(PageEvent::Candidates {
            page,
            request,
            candidates: Vec::new(),
        });
        Ok(())
    }

    fn replay_pointer(
        &mut self,
        page: PageId,
        _kind: PointKind,
        _point: Vec2,
    ) -> Result<(), PageSourceError> {
        self.page_mut(page)?.next_paint = None;
        Ok(())
    }

    fn scroll_progress(&mut self, page: PageId) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let max = entry.max_scroll();
        let along = |at: f32, max: f32| if max > 0.0 { at / max } else { 0.0 };
        let progress = Vec2::new(along(entry.scroll.x, max.x), along(entry.scroll.y, max.y));
        self.pending
            .push(PageEvent::ScrollProgress { page, progress });
        Ok(())
    }

    fn scroll_to(&mut self, page: PageId, progress: Vec2) -> Result<(), PageSourceError> {
        let entry = self.page_mut(page)?;
        let next = entry.max_scroll() * progress.clamp(Vec2::ZERO, Vec2::ONE);
        if next != entry.scroll {
            entry.scroll = next;
            entry.next_paint = None;
            self.pending
                .push(PageEvent::Scrolled { page, offset: next });
        }
        Ok(())
    }

    fn navigate(&mut self, page: PageId, nav: &PageNav) -> Result<(), PageSourceError> {
        let entry = self.page_mut(page)?;
        match nav {
            PageNav::To(url) => {
                entry.history.truncate(entry.at + 1);
                entry.history.push(url.clone());
                entry.at += 1;
            }
            PageNav::Back if entry.at > 0 => entry.at -= 1,
            PageNav::Forward if entry.at + 1 < entry.history.len() => entry.at += 1,
            PageNav::Reload => {}
            // A synthetic load ends as it starts, so there is none to stop.
            PageNav::Back | PageNav::Forward | PageNav::Stop => return Ok(()),
        }
        self.show(page)
    }

    fn query_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let element = element_scrolled(entry.spec.viewport, entry.scroll, point);
        self.pending.push(PageEvent::ElementAt {
            page,
            request,
            element,
        });
        Ok(())
    }

    fn inspect_at(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let node = inspected_scrolled(entry.spec.viewport, entry.scroll, point).map(Box::new);
        self.pending.push(PageEvent::Inspected {
            page,
            request,
            node,
        });
        Ok(())
    }

    fn capture_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let element = Some(synthetic_capture(entry.spec.viewport, point));
        self.pending.push(PageEvent::ElementCaptured {
            page,
            request,
            element,
        });
        Ok(())
    }

    fn track_elements(
        &mut self,
        page: PageId,
        selectors: &[String],
    ) -> Result<(), PageSourceError> {
        let entry = self.page_mut(page)?;
        entry.tracked = selectors.to_vec();
        let places = entry.places(page);
        self.pending.extend(places);
        Ok(())
    }

    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let boxes = synthetic_elements_in(entry.spec.viewport, entry.scroll, rect);
        self.pending.push(PageEvent::ElementsInRect {
            page,
            request,
            count: boxes.len(),
            boxes,
        });
        Ok(())
    }

    fn pump(&mut self) {
        self.pump_at(Instant::now());
    }

    fn paints_on_pump(&self) -> bool {
        self.pages.values().any(|page| page.painting)
    }

    fn drain_events(&mut self, out: &mut Vec<PageEvent>) {
        out.append(&mut self.pending);
    }

    fn devtools_port(&self) -> Option<u16> {
        None
    }

    fn devtools_send(&mut self, page: PageId, message: &str) -> Result<(), PageSourceError> {
        self.answer_devtools(page, message)
    }

    fn set_devtools_sink(&mut self, sink: Option<DevtoolsSink>) {
        self.devtools_sink = sink;
    }

    fn shutdown(&mut self) {
        self.pages.clear();
        self.focused = None;
        self.pending.clear();
        self.devtools_out.clear();
    }
}
