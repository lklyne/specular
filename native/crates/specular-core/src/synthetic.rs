//! A CEF-free [`PageSource`] that paints animated CPU frames.
//!
//! It lets the app, compositor and bench run on any machine (including CI
//! without a GPU or CEF). Its frames are [`PageFrame::Cpu`] and therefore
//! non-representative: use it to exercise the pipeline, never to compare
//! against Electron.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use glam::Vec2;

use crate::frame::{CpuFrame, FrameEvent, FrameLayer, PageFrame};
use crate::geometry::{CssRect, CssSize, PixelRect};
use crate::input::InputEvent;
use crate::locator::LocatorBundle;
use crate::page::{PageId, PageSpec, validate_texture_scale, validate_viewport};
use crate::source::{
    DevtoolsSink, InspectedNode, PageElement, PageEvent, PageNav, PageSource, PageSourceError,
    PointKind,
};

mod cdp;

#[derive(Debug)]
struct SyntheticPage {
    spec: PageSpec,
    painting: bool,
    frames_painted: u64,
    next_paint: Option<Instant>,
    /// Every address the page has shown, oldest first.
    history: Vec<String>,
    /// Which entry of `history` it shows now.
    at: usize,
    /// How far its document is scrolled, in CSS pixels.
    scroll: Vec2,
    /// Whether a devtools client enabled the `Page` domain, so navigations
    /// are reported to it.
    page_events: bool,
}

impl SyntheticPage {
    /// The furthest the document scrolls: it is three viewports tall and
    /// one wide.
    fn max_scroll(&self) -> Vec2 {
        Vec2::new(0.0, 2.0 * self.spec.viewport.height as f32)
    }
}

/// The title a synthetic page gives the document at `url`: the address
/// without its scheme.
fn title_of(url: &str) -> String {
    let address = url.split_once("://").map_or(url, |(_, rest)| rest);
    format!("Synthetic {}", address.trim_end_matches('/'))
}

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
    pub fn pump_at(&mut self, now: Instant) {
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

/// The size of one cell of the grid a synthetic page answers
/// [`query_element`](PageSource::query_element) from, in CSS pixels.
const CELL: (u32, u32) = (160, 48);

/// The element a synthetic page laid out at `viewport` has under `point`:
/// the cell of a 160x48 grid holding it, cut off at the viewport's edge.
/// `None` outside the viewport.
///
/// The frames show no such grid. It stands in for a DOM so that what is
/// built on an element answer runs with no browser, and gives the same
/// answer every time.
pub fn synthetic_element_at(viewport: CssSize, point: Vec2) -> Option<PageElement> {
    let inside = point.x >= 0.0
        && point.y >= 0.0
        && point.x < viewport.width as f32
        && point.y < viewport.height as f32;
    if !inside {
        return None;
    }
    let (column, row) = (point.x as u32 / CELL.0, point.y as u32 / CELL.1);
    let (x, y) = (column * CELL.0, row * CELL.1);
    Some(PageElement {
        selector: format!("div.cell[data-col=\"{column}\"][data-row=\"{row}\"]"),
        element_path: Some("body > div.cell".to_owned()),
        bounding_box: PixelRect::new(
            x as i32,
            y as i32,
            CELL.0.min(viewport.width - x),
            CELL.1.min(viewport.height - y),
        ),
    })
}

/// The element a synthetic page scrolled by `scroll` has under the viewport
/// point `point`. The grid is laid out in the document, so its cells move up
/// as the page scrolls down, and the box is where the cell sits in the
/// viewport now.
fn element_scrolled(viewport: CssSize, scroll: Vec2, point: Vec2) -> Option<PageElement> {
    synthetic_element_at(viewport, point)?;
    let at = point + scroll;
    let (column, row) = (at.x as u32 / CELL.0, at.y as u32 / CELL.1);
    let (x, y) = ((column * CELL.0) as f32, (row * CELL.1) as f32);
    Some(PageElement {
        selector: format!("div.cell[data-col=\"{column}\"][data-row=\"{row}\"]"),
        element_path: Some("body > div.cell".to_owned()),
        bounding_box: PixelRect::new(
            (x - scroll.x).round() as i32,
            (y - scroll.y).round() as i32,
            CELL.0.min(viewport.width.saturating_sub(x as u32)),
            CELL.1,
        ),
    })
}

/// [`inspected_scrolled`] for a page that has not scrolled: what a synthetic
/// page answers an inspect at `point` with.
pub fn synthetic_inspected_at(viewport: CssSize, point: Vec2) -> Option<InspectedNode> {
    inspected_scrolled(viewport, Vec2::ZERO, point)
}

/// The cell of a synthetic page's grid under the viewport point `point`, as
/// the inspect tool reads it: a `div.cell` named for its column and row, in
/// the styles every cell has.
fn inspected_scrolled(viewport: CssSize, scroll: Vec2, point: Vec2) -> Option<InspectedNode> {
    let element = element_scrolled(viewport, scroll, point)?;
    let at = point + scroll;
    let (column, row) = (at.x as u32 / CELL.0, at.y as u32 / CELL.1);
    let styles = [
        ("display", "block"),
        ("position", "static"),
        ("font-family", "Inter, sans-serif"),
        ("font-size", "14px"),
        ("font-weight", "400"),
        ("color", "rgb(17, 24, 39)"),
        ("background", "rgb(255, 255, 255)"),
        ("padding", "8px"),
        ("margin", "0px"),
    ];
    Some(InspectedNode {
        node_id: format!("cell-{column}-{row}"),
        tag_name: "div".to_owned(),
        name: format!("div \"Cell {column},{row}\""),
        selector: element.selector,
        id_attribute: Some(format!("cell-{column}-{row}")),
        classes: vec!["cell".to_owned()],
        styles: styles
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
        bounding_box: element.bounding_box,
    })
}

/// How many cells of a synthetic page's grid lie wholly inside `rect`, in
/// the viewport CSS pixels of a page laid out at `viewport` and scrolled by
/// `scroll`. A cell the rect only clips is not grabbed.
pub fn synthetic_elements_in(viewport: CssSize, scroll: Vec2, rect: CssRect) -> usize {
    let (left, top) = (rect.x + scroll.x, rect.y + scroll.y);
    let (right, bottom) = (left + rect.width, top + rect.height);
    let cells = |from: f32, to: f32, cell: u32, limit: f32| {
        let first = (from.max(0.0) / cell as f32).ceil();
        let last = (to.min(limit) / cell as f32).floor();
        (last - first).max(0.0) as usize
    };
    let document_height = 3.0 * viewport.height as f32;
    cells(left, right, CELL.0, viewport.width as f32) * cells(top, bottom, CELL.1, document_height)
}

/// Paints a flat colour that cycles with `frame_index` plus a sweeping bar,
/// so every frame differs and a stalled page is visible at a glance.
fn paint(spec: &PageSpec, frame_index: u64) -> CpuFrame {
    let size = spec.viewport.to_pixels(spec.texture_scale);
    let stride = size.width * 4;
    let shade = (frame_index % 256) as u8;
    let bar_x = (frame_index * 8) % u64::from(size.width.max(1));
    let mut bgra = vec![0_u8; stride as usize * size.height as usize];
    for row in bgra.chunks_exact_mut(stride as usize) {
        let (texels, _) = row.as_chunks_mut::<4>();
        for (x, texel) in texels.iter_mut().enumerate() {
            let in_bar = (x as u64).abs_diff(bar_x) < 16;
            let value = if in_bar { 255 } else { shade };
            *texel = [value, 96, 255 - value, 255];
        }
    }
    CpuFrame {
        size,
        stride,
        bgra,
        dirty: Vec::new(),
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

    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.page(page)?;
        let count = synthetic_elements_in(entry.spec.viewport, entry.scroll, rect);
        self.pending.push(PageEvent::ElementsInRect {
            page,
            request,
            count,
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

#[cfg(test)]
mod tests;
