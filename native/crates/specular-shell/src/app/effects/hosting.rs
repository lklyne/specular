//! Hosting pages: creating, closing, focusing, resizing and reloading them, and their input.

use std::time::Instant;

use anyhow::Context as _;
use specular_core::{CssSize, InputEvent, PageNav, PageSpec, PointerEvent, PointerEventKind};
use specular_doc::EntityId;

use super::super::runtime::{PageHost, Runtime, ShellWindow};
use super::super::settings::page_device;
use super::warn_on_error;

impl<W: ShellWindow> Runtime<W> {
    pub(super) fn focus_page(&mut self, page: Option<&EntityId>) {
        let host = page.and_then(|page| self.hosts.get(page));
        warn_on_error(self.source.set_focus(host.map(|host| host.page)));
    }

    pub(super) fn set_page_viewport(&mut self, page: &EntityId, viewport: specular_core::CssSize) {
        if let Some(host) = self.hosts.get(page) {
            warn_on_error(self.source.set_viewport(host.page, viewport));
        }
    }

    /// Lets the agent whose cursor has arrived carry out its act.
    pub(super) fn agent_go(&self, ticket: u64) {
        tracing::debug!(ticket, "the agent's cursor is there");
        if let Some(cdp) = &self.cdp {
            cdp.go(ticket);
        }
    }

    /// Hosts `page` at `url`, watching the file it loads if it is one.
    pub(super) fn create_page(
        &mut self,
        page: &EntityId,
        url: &str,
        viewport: CssSize,
    ) -> anyhow::Result<()> {
        let lod = self.starting_lod(page);
        let scale = self.gpu.as_ref().map_or(1.0, W::scale_factor);
        let mut spec = PageSpec::new(url, viewport);
        spec.texture_scale = scale * lod.texture().factor();
        spec.device = page_device(&self.app, page, scale);
        let host = self
            .source
            .create_page(&spec)
            .with_context(|| format!("creating page for {url}"))?;
        self.hosts
            .insert(page.clone(), PageHost { page: host, lod });
        self.loads.pages.insert(host);
        self.loads.unpainted.insert(host);
        self.page_files.hosted(page, url);
        self.give_color_scheme(page);
        Ok(())
    }

    /// Stops hosting `page` and answers what was asked of it.
    pub(super) fn close_page(&mut self, page: &EntityId) {
        self.page_files.closed(page);
        if let Some(host) = self.hosts.remove(page) {
            if let Some(cdp) = &self.cdp {
                cdp.page_closed(page);
            }
            self.loads.page_closed(host.page);
            warn_on_error(self.source.close_page(host.page));
            if let Some(gpu) = self.gpu.as_mut() {
                gpu.compositor_mut().remove_page(host.page);
            }
        }
        self.give_up_on_page(page);
    }

    /// Loads again each page whose HTML file changed on disk, unless the
    /// page has since gone somewhere else.
    pub(crate) fn reload_changed_pages(&mut self, now: Instant) {
        for page in self.page_files.changed(now) {
            let live = (self.app.page_state(&page)).and_then(|state| state.url.as_deref());
            if live.is_some_and(|url| !self.page_files.watches(&page, url)) {
                continue;
            }
            self.on_host(&page, |source, host| {
                source.navigate(host, &PageNav::Reload)
            });
        }
    }

    pub(super) fn send_to_page(&mut self, page: &EntityId, event: &InputEvent) {
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
