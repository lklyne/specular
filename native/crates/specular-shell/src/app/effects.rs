//! The effect runner: sends an [`Event`] through `update` and does the I/O
//! that comes back.

mod asking;
mod hosting;

use std::time::Instant;

use specular_bench::PaintPolicy;
use specular_core::PageNav;
use specular_doc::EntityId;
use specular_interact::{Effect, Event, update};

use super::FolderAsk;
use super::runtime::{Runtime, ShellWindow};
use crate::paint_lod::PageLod;

impl<W: ShellWindow> Runtime<W> {
    /// Applies `event` to the app and runs its effects in order. A page that
    /// cannot be hosted ends the run.
    pub fn dispatch(&mut self, event: Event) {
        // Any event may change what a frame shows. The clock is the one
        // sent with nothing to say, and it goes through `demand::tick`.
        self.demand.changed();
        // `RUST_LOG=specular_shell::app::effects=trace` says why frames are
        // being drawn on a canvas that should be idle.
        if tracing::enabled!(tracing::Level::TRACE) {
            let event = format!("{event:?}");
            let name = event.split([' ', '(', '{']).next().unwrap_or_default();
            tracing::trace!(event = name, "frame owed");
        }
        let effects = update(&mut self.app, event);
        if let Some(sent) = self.sent.as_mut() {
            sent.extend(effects.iter().cloned());
        }
        let resized =
            (effects.iter()).any(|effect| matches!(effect, Effect::SetPageViewport { .. }));
        self.run_all(effects);
        if resized {
            // A resized page is told now, not at the next frame.
            self.settle_pages(Instant::now());
        }
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
            .map_or_else(PageLod::default, |(_, placement)| {
                PageLod::starting_at(placement.display_scale(&camera))
            })
    }

    fn run(&mut self, effect: Effect) -> anyhow::Result<()> {
        if !self.options.writes && leaves_memory(&effect) {
            return Ok(());
        }
        // Where no agent runs, a script plays its side: the run stays in
        // flight until the script says how it went.
        if !self.options.agents && matches!(effect, Effect::RunAgent(_) | Effect::CancelAgent(_)) {
            return Ok(());
        }
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => self.create_page(&page, &url, viewport)?,
            Effect::ClosePage(page) => self.close_page(&page),
            Effect::SetPageViewport { page, viewport } => self.set_page_viewport(&page, viewport),
            Effect::FocusPage(page) => self.focus_page(page.as_ref()),
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
            Effect::EditField(field) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.edit_field(&field);
                }
            }
            Effect::Save => self.request_save(),
            Effect::WriteCanvas(canvas) => self.write_canvas(&canvas),
            Effect::RenameCanvasFile { canvas, from, to } => {
                self.rename_canvas_file(&canvas, &from, &to);
            }
            Effect::TrashCanvasFile { canvas, file } => self.trash_canvas_file(&canvas, &file),
            Effect::SaveSpaceMeta => self.save_space_meta(),
            // No transport carries shared canvases yet (ADR 0048).
            Effect::ShareSend { canvas, update } => share_send(&canvas, &update),
            Effect::LoadImage { image, file } => self.load_image(image, &file),
            Effect::RasterImage { image, file, size } => self.raster_image(image, &file, size),
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
            Effect::SaveTheme(theme) => self.save_theme(theme),
            Effect::SaveAgentModel(model) => self.save_agent_model(&model),
            Effect::SetPageColorScheme { page, scheme } => {
                self.set_page_color_scheme(&page, scheme);
            }
            Effect::SetPageDevice { page, device } => self.set_page_device(&page, device),
            Effect::LoadThreads => self.load_threads(),
            Effect::WriteThread(thread) => self.write_thread(&thread),
            Effect::WriteThreadIndex => self.write_thread_index(),
            Effect::RunAgent(request) => self.run_agent(&request),
            Effect::CancelAgent(thread) => self.cancel_agent(&thread),
            Effect::ApiReply { outcome, .. } => self.api_outcome = Some(outcome),
            Effect::AgentGo { ticket } => self.agent_go(ticket),
            effect @ (Effect::CapturePage(_)
            | Effect::AskCandidates { .. }
            | Effect::ReplayPointer { .. }
            | Effect::AskScrollProgress(_)
            | Effect::AskScrollRoom { .. }
            | Effect::ScrollPage { .. }
            | Effect::CaptureElement { .. }
            | Effect::TrackElements { .. }) => self.run_sync(effect),
            Effect::Navigate { page, nav } => {
                if let PageNav::To(url) = &nav {
                    self.page_files.hosted(&page, url);
                }
                self.on_host(&page, |source, host| source.navigate(host, &nav));
            }
            Effect::QueryElement { page, point } => self.query_element(&page, point),
            Effect::InspectAt { page, point, pick } => self.inspect_at(&page, point, pick),
            Effect::SaveRepos => self.save_repos(),
            Effect::ChooseSpace { create } => self.ask_for_folder(FolderAsk::Space { create }),
            Effect::OpenSpace(folder) => self.choose_space(std::path::Path::new(&folder)),
            Effect::RevealSpace => self.reveal_space(),
            Effect::Quit => self.exit(),
            Effect::SaveSettings(settings) => self.save_settings(settings),
            Effect::PickRepoFolder { origin } => self.ask_for_folder(FolderAsk::Repo { origin }),
            Effect::QueryRegionGrab { region, pages } => self.query_region_grab(region, &pages),
            Effect::PreviewRegion { pages } => self.preview_region(&pages),
        }
        Ok(())
    }
}

/// Whether `effect` writes to disk past the canvas and the Documents, or
/// asks the user something: what a run that does not write leaves undone.
const fn leaves_memory(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::WriteAsset { .. }
            | Effect::CopyAsset { .. }
            | Effect::LoadThreads
            | Effect::WriteThread(_)
            | Effect::WriteThreadIndex
            | Effect::RunAgent(_)
            | Effect::CancelAgent(_)
            | Effect::ChooseSpace { .. }
            | Effect::OpenSpace(_)
            | Effect::RevealSpace
            | Effect::PickRepoFolder { .. }
            | Effect::SaveRepos
    )
}

/// No transport carries a shared canvas's updates yet (ADR 0048).
fn share_send(canvas: &specular_interact::CanvasId, update: &[u8]) {
    tracing::debug!(
        "share update for {canvas}: {} bytes, not sent",
        update.len()
    );
}

fn warn_on_error<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(error) = result {
        tracing::warn!("{error}");
    }
}
