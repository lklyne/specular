//! The effect runner: sends an [`Event`] through `update` and does the I/O
//! that comes back.

use std::time::Instant;

use anyhow::Context as _;
use glam::Vec2;
use specular_bench::PaintPolicy;
use specular_core::{InputEvent, PageSpec, PointerEvent, PointerEventKind};
use specular_doc::{EntityId, Rect};
use specular_interact::{Effect, Event, PageRegion, update};

use super::runtime::{PageHost, Runtime, ShellWindow};
use crate::page_queries::css_rect;
use crate::paint_lod::PageLod;

impl<W: ShellWindow> Runtime<W> {
    /// Applies `event` to the app and runs its effects in order. A page that
    /// cannot be hosted ends the run.
    pub fn dispatch(&mut self, event: Event) {
        // Any event may change what a frame shows. The clock is the one
        // sent with nothing to say, and it goes through `demand::tick`.
        self.demand.changed();
        // `RUST_LOG=specular_app::app::effects=trace` says why frames are
        // being drawn on a canvas that should be idle.
        if tracing::enabled!(tracing::Level::TRACE) {
            let event = format!("{event:?}");
            let name = event.split([' ', '(', '{']).next().unwrap_or_default();
            tracing::trace!(event = name, "frame owed");
        }
        let effects = update(&mut self.app, event);
        self.run_all(effects);
    }

    /// Runs `effects` in order. A page that cannot be hosted ends the run.
    pub(crate) fn run_all(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            if let Err(error) = self.run(effect) {
                self.fail(error);
                return;
            }
        }
    }

    /// The paint LOD a page starts with: what it is owed where it now
    /// shows, or full when pages are not graded at all.
    fn starting_lod(&self, page: &EntityId) -> PageLod {
        if self.options.paint_policy != PaintPolicy::ElectronLod {
            return PageLod::default();
        }
        let camera = self.app.session().camera;
        (self.app.pages())
            .find(|(id, ..)| *id == page)
            .map_or_else(PageLod::default, |(_, _, placement)| {
                PageLod::starting_at(placement.display_scale(&camera))
            })
    }

    fn run(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => {
                let lod = self.starting_lod(&page);
                let mut spec = PageSpec::new(&url, viewport);
                spec.texture_scale =
                    self.gpu.as_ref().map_or(1.0, W::scale_factor) * lod.texture().factor();
                let host = self
                    .source
                    .create_page(&spec)
                    .with_context(|| format!("creating page for {url}"))?;
                self.hosts.insert(page, PageHost { page: host, lod });
            }
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    if let Some(cdp) = &self.cdp {
                        cdp.page_closed(&page);
                    }
                    warn_on_error(self.source.close_page(host.page));
                    if let Some(gpu) = self.gpu.as_mut() {
                        gpu.compositor_mut().remove_page(host.page);
                    }
                }
                self.give_up_on_page(&page);
            }
            Effect::SetPageViewport { page, viewport } => {
                if let Some(host) = self.hosts.get(&page) {
                    warn_on_error(self.source.set_viewport(host.page, viewport));
                }
            }
            Effect::FocusPage(page) => {
                let host = page.and_then(|page| self.hosts.get(&page));
                warn_on_error(self.source.set_focus(host.map(|host| host.page)));
            }
            Effect::ForwardInput { page, event } => self.send_to_page(&page, &event),
            Effect::SetImeAllowed(allowed) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.set_ime_allowed(allowed);
                }
            }
            Effect::SetImeCursorArea { origin, size } => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.set_ime_cursor_area(origin, size);
                }
            }
            Effect::SetCursor(cursor) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.set_cursor(cursor);
                }
            }
            Effect::Save => self.request_save(),
            Effect::WriteCanvas(canvas) => self.write_canvas(&canvas),
            Effect::RenameCanvasFile { canvas, from, to } => {
                self.rename_canvas_file(&canvas, &from, &to);
            }
            Effect::TrashCanvasFile { canvas, file } => self.trash_canvas_file(&canvas, &file),
            Effect::SaveSpaceMeta => self.save_space_meta(),
            Effect::LoadImage { image, file } => self.load_image(image, &file),
            Effect::DropImage(image) => self.drop_image(image),
            Effect::LoadNote { file } => self.load_note(&file),
            Effect::DropNote { file } => self.drop_note(&file),
            Effect::WriteNote { file, text } => self.write_note(file, text),
            Effect::CreateNote { rect } => self.create_note(rect),
            Effect::WriteClipboard(text) => self.write_clipboard(text),
            Effect::ReadClipboard => self.read_clipboard(),
            Effect::WriteAsset { file, bytes } => self.write_asset(&file, bytes.as_slice()),
            Effect::CopyAsset { from, file } => self.copy_asset(&from, &file),
            Effect::SaveToolDefaults(defaults) => self.save_tool_defaults(&defaults),
            Effect::LoadThreads => self.load_threads(),
            Effect::WriteThread(thread) => self.write_thread(&thread),
            Effect::WriteThreadIndex => self.write_thread_index(),
            Effect::RunAgent(request) => self.run_agent(&request),
            Effect::CancelAgent(thread) => self.cancel_agent(&thread),
            Effect::ApiReply { outcome, .. } => self.api_outcome = Some(outcome),
            Effect::Navigate { page, nav } => {
                if let Some(host) = self.hosts.get(&page) {
                    warn_on_error(self.source.navigate(host.page, &nav));
                }
            }
            Effect::QueryElement { page, point } => self.query_element(&page, point),
            Effect::QueryRegionGrab { region, pages } => self.query_region_grab(region, &pages),
        }
        Ok(())
    }

    /// Asks the page for the element at `point`. The answer comes back
    /// through the page's events; a page that cannot be asked has none.
    fn query_element(&mut self, page: &EntityId, point: Vec2) {
        let request = self.queries.ask_element(page.clone(), point);
        let asked = (self.hosts.get(page))
            .is_some_and(|host| self.source.query_element(host.page, point, request).is_ok());
        if !asked && let Some(answer) = self.queries.element_answer(request, None) {
            self.dispatch(answer);
        }
    }

    /// Asks each page what the region grabbed in it. The answer goes to the
    /// app once every page has said; a page that cannot be asked grabbed
    /// nothing.
    fn query_region_grab(&mut self, region: Rect, pages: &[PageRegion]) {
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

    /// Tells the app about the regions every page has now answered for.
    pub(super) fn answer_settled_grabs(&mut self) {
        for answer in self.queries.settled() {
            self.dispatch(answer);
        }
    }

    /// Answers what was still being asked of a page that will not answer.
    pub(super) fn give_up_on_page(&mut self, page: &EntityId) {
        for answer in self.queries.give_up_on(page) {
            self.dispatch(answer);
        }
        self.answer_settled_grabs();
    }

    fn send_to_page(&mut self, page: &EntityId, event: &InputEvent) {
        let Some(host) = self.hosts.get(page) else {
            return;
        };
        if let Err(error) = self.source.send_input(host.page, event) {
            tracing::warn!("{error}");
            return;
        }
        let is_hover = matches!(
            event,
            InputEvent::Pointer(PointerEvent {
                kind: PointerEventKind::Move | PointerEventKind::Leave,
                ..
            })
        );
        if !is_hover {
            self.latency.input_sent(host.page, Instant::now());
        }
    }
}

fn warn_on_error<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        tracing::warn!("{error}");
    }
}
