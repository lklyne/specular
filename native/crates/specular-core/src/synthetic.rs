//! A CEF-free [`PageSource`] that paints animated CPU frames.
//!
//! It lets the app, compositor and bench run on any machine (including CI
//! without a GPU or CEF). Its frames are [`PageFrame::Cpu`] and therefore
//! non-representative: use it to exercise the pipeline, never to compare
//! against Electron.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::frame::{CpuFrame, FrameEvent, FrameLayer, PageFrame};
use crate::geometry::CssSize;
use crate::input::InputEvent;
use crate::page::{PageId, PageSpec, validate_texture_scale, validate_viewport};
use crate::source::{PageEvent, PageSource, PageSourceError};

#[derive(Debug)]
struct SyntheticPage {
    spec: PageSpec,
    painting: bool,
    frames_painted: u64,
    next_paint: Option<Instant>,
}

/// Synthetic page backend; see the module docs.
#[derive(Debug, Default)]
pub struct SyntheticPageSource {
    pages: BTreeMap<PageId, SyntheticPage>,
    next_id: u64,
    focused: Option<PageId>,
    pending: Vec<PageEvent>,
}

impl SyntheticPageSource {
    /// Creates an empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// [`pump`](PageSource::pump) with an explicit clock, for deterministic tests.
    pub fn pump_at(&mut self, now: Instant) {
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

    fn page_mut(&mut self, page: PageId) -> Result<&mut SyntheticPage, PageSourceError> {
        self.pages
            .get_mut(&page)
            .ok_or(PageSourceError::UnknownPage(page))
    }
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
        for (x, texel) in row.chunks_exact_mut(4).enumerate() {
            let in_bar = (x as u64).abs_diff(bar_x) < 16;
            let value = if in_bar { 255 } else { shade };
            texel.copy_from_slice(&[value, 96, 255 - value, 255]);
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
            },
        );
        self.pending.push(PageEvent::Loaded {
            page: id,
            http_status: 200,
        });
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

    fn send_input(&mut self, page: PageId, _event: &InputEvent) -> Result<(), PageSourceError> {
        // Input forces the next pump to repaint, standing in for the DOM
        // reacting so input-to-paint latency is measurable end to end.
        self.page_mut(page)?.next_paint = None;
        Ok(())
    }

    fn pump(&mut self) {
        self.pump_at(Instant::now());
    }

    fn drain_events(&mut self, out: &mut Vec<PageEvent>) {
        out.append(&mut self.pending);
    }

    fn devtools_port(&self) -> Option<u16> {
        None
    }

    fn shutdown(&mut self) {
        self.pages.clear();
        self.focused = None;
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::PixelSize;

    fn frames(events: &[PageEvent]) -> Vec<&FrameEvent> {
        events
            .iter()
            .filter_map(|event| match event {
                PageEvent::Frame(frame) => Some(frame),
                _ => None,
            })
            .collect()
    }

    fn source_with_page(scale: f32) -> (SyntheticPageSource, PageId) {
        let mut source = SyntheticPageSource::new();
        let mut spec = PageSpec::new("https://example.com/", CssSize::new(40, 20));
        spec.texture_scale = scale;
        let id = source.create_page(&spec).unwrap();
        (source, id)
    }

    #[test]
    fn pump_paints_frame_at_texture_scaled_size() {
        let (mut source, _) = source_with_page(0.5);
        source.pump_at(Instant::now());
        let mut events = Vec::new();
        source.drain_events(&mut events);
        assert_eq!(frames(&events)[0].frame.size(), PixelSize::new(20, 10));
    }

    #[test]
    fn pump_respects_frame_rate_interval() {
        let (mut source, _) = source_with_page(1.0);
        let start = Instant::now();
        source.pump_at(start);
        source.pump_at(start + Duration::from_millis(1));
        let mut events = Vec::new();
        source.drain_events(&mut events);
        assert_eq!(frames(&events).len(), 1);
    }

    #[test]
    fn page_with_painting_off_produces_no_frames() {
        let (mut source, id) = source_with_page(1.0);
        source.set_painting(id, false).unwrap();
        source.pump_at(Instant::now());
        let mut events = Vec::new();
        source.drain_events(&mut events);
        assert!(frames(&events).is_empty());
    }

    #[test]
    fn closed_page_reports_unknown_page() {
        let (mut source, id) = source_with_page(1.0);
        source.close_page(id).unwrap();
        assert!(matches!(
            source.set_frame_rate(id, 30),
            Err(PageSourceError::UnknownPage(_))
        ));
    }

    #[test]
    fn create_page_rejects_empty_viewport() {
        let mut source = SyntheticPageSource::new();
        let spec = PageSpec::new("https://example.com/", CssSize::new(0, 10));
        assert!(matches!(
            source.create_page(&spec),
            Err(PageSourceError::InvalidSpec(_))
        ));
    }
}
