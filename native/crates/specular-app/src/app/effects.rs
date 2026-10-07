//! The effect runner: sends an [`Event`] through `update` and does the I/O
//! that comes back.

use std::time::Instant;

use anyhow::Context as _;
use specular_core::{InputEvent, PageSpec, PointerEvent, PointerEventKind};
use specular_doc::EntityId;
use specular_interact::{Effect, Event, update};
use winit::dpi::{LogicalPosition, LogicalSize};

use super::{GpuWindow, PageHost, Shell};
use crate::paint_lod::PageLod;
use crate::translate;

impl Shell {
    /// Applies `event` to the app and runs its effects in order. A page that
    /// cannot be hosted ends the run.
    pub(super) fn dispatch(&mut self, event: Event) {
        for effect in update(&mut self.app, event) {
            if let Err(error) = self.run(effect) {
                self.fail(error);
                return;
            }
        }
    }

    fn run(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => {
                let mut spec = PageSpec::new(&url, viewport);
                spec.texture_scale = self.gpu.as_ref().map_or(1.0, GpuWindow::scale_factor);
                let host = self
                    .source
                    .create_page(&spec)
                    .with_context(|| format!("creating page for {url}"))?;
                let lod = PageLod::default();
                self.hosts.insert(page, PageHost { page: host, lod });
            }
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    warn_on_error(self.source.close_page(host.page));
                    if let Some(gpu) = self.gpu.as_mut() {
                        gpu.compositor.remove_page(host.page);
                    }
                }
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
                    gpu.window.set_ime_allowed(allowed);
                }
            }
            Effect::SetImeCursorArea { origin, size } => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.window.set_ime_cursor_area(
                        LogicalPosition::new(origin.x, origin.y),
                        LogicalSize::new(size.x, size.y),
                    );
                }
            }
            Effect::SetCursor(cursor) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    gpu.window.set_cursor(translate::cursor_icon(cursor));
                }
            }
            Effect::Save | Effect::WriteClipboard(_) => {
                tracing::debug!(?effect, "effect has no runner yet");
            }
        }
        Ok(())
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
