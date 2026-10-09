//! Running what the app asks for in a headless run: pages on the synthetic
//! source, images and Documents from the shell's loader threads, and the
//! clipboard and new Documents in memory.

use std::time::{Duration, Instant};

use anyhow::Context as _;
use glam::Vec2;
use specular_core::{PageEvent, PageId, PageSource, PageSourceError, PageSpec, PixelSize};
use specular_doc::{ColorScheme, EntityId, Rect};
use specular_interact::{
    ClipboardContent, Effect, Event, ImageKey, ImageNotice, NoteNotice, PageRegion,
};

use super::Headless;
use crate::app::{page_color_scheme, page_own_scheme};
use crate::images::LoadFailure;
use crate::notes::ReadFailure;
use crate::page_notice::notice_of;
use crate::page_queries::css_rect;

/// How long a snapshot waits for images, Documents and real pages before
/// drawing without them.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

/// How long real pages are left to paint once they have all loaded.
const PAGE_SETTLE: Duration = Duration::from_millis(300);

impl Headless {
    /// Tells a page the scheme it reports: its own, or the app's.
    fn set_color_scheme(
        &mut self,
        page: &EntityId,
        own: Option<ColorScheme>,
    ) -> anyhow::Result<()> {
        if let Some(&host) = self.hosts.get(page) {
            let scheme = page_color_scheme(own, self.app.app().appearance());
            self.source.set_color_scheme(host, scheme)?;
        }
        Ok(())
    }

    pub(super) fn run(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => {
                let mut spec = PageSpec::new(&url, viewport);
                spec.texture_scale = self.scale;
                let host = self
                    .source
                    .create_page(&spec)
                    .with_context(|| format!("creating page for {url}"))?;
                self.hosts.insert(page.clone(), host);
                let own = page_own_scheme(self.app.app().document(), &page);
                self.set_color_scheme(&page, own)?;
                self.loading_pages.insert(host);
                self.unpainted_pages.insert(host);
            }
            Effect::ClosePage(page) => self.close_page(&page)?,
            Effect::SetPageViewport { page, viewport } => {
                if let Some(&host) = self.hosts.get(&page) {
                    self.source.set_viewport(host, viewport)?;
                }
            }
            Effect::SetPageColorScheme { page, scheme } => self.set_color_scheme(&page, scheme)?,
            Effect::LoadImage { image, file } => {
                self.loading_images.insert(image);
                self.images
                    .request(image, &file, self.compositor.image_spec());
            }
            Effect::RasterImage { image, file, size } => self.raster_image(image, &file, size),
            Effect::DropImage(image) => self.drop_image(image),
            Effect::LoadNote { file } => {
                if let Some(text) = self.stand_ins.note(&file) {
                    let notice = NoteNotice::Text(text.to_owned());
                    self.drive(|app| app.send(Event::Note { file, notice }))?;
                } else {
                    self.notes.watch(&file);
                    self.loading_notes.insert(file);
                }
            }
            Effect::DropNote { file } => {
                self.notes.unwatch(&file);
                self.loading_notes.remove(&file);
            }
            effect @ (Effect::WriteClipboard(_)
            | Effect::ReadClipboard
            | Effect::WriteNote { .. }
            | Effect::CreateNote { .. }) => self.run_stand_in(effect)?,
            effect @ (Effect::Navigate { .. }
            | Effect::CapturePage(_)
            | Effect::AskCandidates { .. }
            | Effect::ReplayPointer { .. }
            | Effect::AskScrollProgress(_)
            | Effect::ScrollPage { .. }
            | Effect::CaptureElement { .. }
            | Effect::TrackElements { .. }) => self.run_sync(effect)?,
            Effect::FocusPage(page) => {
                let host = page.and_then(|page| self.hosts.get(&page).copied());
                self.source.set_focus(host)?;
            }
            Effect::ForwardInput { page, event } => {
                if let Some(&host) = self.hosts.get(&page) {
                    self.source.send_input(host, &event)?;
                }
            }
            Effect::QueryElement { page, point } => self.query_element(&page, point)?,
            Effect::InspectAt { page, point, pick } => self.inspect_at(&page, point, pick)?,
            Effect::QueryRegionGrab { region, pages } => self.query_region_grab(region, &pages)?,
            // A headless run has no cursor and no input method, it leaves
            // the disk alone, and it hosts no API.
            Effect::ApiReply { .. }
            | Effect::SetImeAllowed(_)
            | Effect::SetImeCursorArea { .. }
            | Effect::SetCursor(_)
            | Effect::EditField(_)
            | Effect::Save
            | Effect::WriteCanvas(_)
            | Effect::RenameCanvasFile { .. }
            | Effect::TrashCanvasFile { .. }
            | Effect::SaveSpaceMeta
            | Effect::WriteAsset { .. }
            | Effect::CopyAsset { .. }
            | Effect::SaveToolDefaults(_)
            | Effect::SaveTheme(_)
            | Effect::LoadThreads
            | Effect::WriteThread(_)
            | Effect::WriteThreadIndex
            | Effect::RunAgent(_)
            | Effect::CancelAgent(_)
            | Effect::SaveRepos
            | Effect::PickRepoFolder { .. }
            | Effect::ChooseSpace { .. }
            | Effect::OpenSpace(_)
            | Effect::RevealSpace
            | Effect::Quit
            | Effect::SaveSettings(_) => {}
        }
        Ok(())
    }

    /// Stops hosting a page and answers what was asked of it.
    fn close_page(&mut self, page: &EntityId) -> anyhow::Result<()> {
        if let Some(host) = self.hosts.remove(page) {
            self.source.close_page(host)?;
            self.compositor.remove_page(host);
            self.loading_pages.remove(&host);
            self.unpainted_pages.remove(&host);
        }
        for answer in self.queries.give_up_on(page) {
            self.drive(|app| app.send(answer))?;
        }
        self.answer_settled_grabs()?;
        Ok(())
    }

    /// Runs what the clipboard and a Document's file are stood in for.
    fn run_stand_in(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::WriteClipboard(text) => self.stand_ins.clipboard = Some(text),
            Effect::ReadClipboard => {
                let content = ClipboardContent {
                    text: self.stand_ins.clipboard.clone(),
                    image: None,
                };
                self.drive(|app| app.send(Event::Clipboard(content)))?;
            }
            Effect::WriteNote { file, text } => self.stand_ins.write_note(&file, text),
            Effect::CreateNote { rect } => {
                let file = self.stand_ins.create_note();
                self.drive(|app| app.send(Event::NoteCreated { file, rect }))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Runs what is asked of hosted pages past their making: a navigation,
    /// what a sync set asks (the capture of the entered page, a peer's
    /// candidates, a replayed pointer and the scroll), and what element
    /// attachment asks (the element under an item, and the ones to track).
    fn run_sync(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::CapturePage(page) => {
                let host = page.and_then(|page| self.hosts.get(&page).copied());
                self.source.set_capture(host)?;
            }
            Effect::AskCandidates {
                page,
                request,
                bundle,
            } => self.on_host(&page, |source, host| {
                source.query_candidates(host, &bundle, request)
            })?,
            Effect::ReplayPointer { page, kind, point } => {
                self.on_host(&page, |source, host| {
                    source.replay_pointer(host, kind, point)
                })?;
            }
            Effect::AskScrollProgress(page) => {
                self.on_host(&page, |source, host| source.scroll_progress(host))?;
            }
            Effect::ScrollPage { page, progress } => {
                self.on_host(&page, |source, host| source.scroll_to(host, progress))?;
            }
            Effect::CaptureElement {
                page,
                request,
                point,
            } => self.on_host(&page, |source, host| {
                source.capture_element(host, point, request)
            })?,
            Effect::TrackElements { page, selectors } => {
                self.on_host(&page, |source, host| {
                    source.track_elements(host, &selectors)
                })?;
            }
            Effect::Navigate { page, nav } => {
                tracing::debug!(%page, ?nav, "navigate");
                self.on_host(&page, |source, host| source.navigate(host, &nav))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Asks something of the backend page that hosts `page`, if one does.
    fn on_host(
        &mut self,
        page: &EntityId,
        ask: impl FnOnce(&mut dyn PageSource, PageId) -> Result<(), PageSourceError>,
    ) -> anyhow::Result<()> {
        if let Some(&host) = self.hosts.get(page) {
            ask(self.source.as_mut(), host)?;
        }
        Ok(())
    }

    /// Asks the page for the element at `point`; a page that cannot be
    /// asked has none.
    fn query_element(&mut self, page: &EntityId, point: Vec2) -> anyhow::Result<()> {
        let request = self.queries.ask_element(page.clone(), point);
        let asked = (self.hosts.get(page))
            .is_some_and(|&host| self.source.query_element(host, point, request).is_ok());
        if !asked && let Some(answer) = self.queries.element_answer(request, None) {
            self.drive(|app| app.send(answer))?;
        }
        Ok(())
    }

    /// Asks the page for the node at `point` as the inspect tool reads it;
    /// a page that cannot be asked has none.
    fn inspect_at(&mut self, page: &EntityId, point: Vec2, pick: bool) -> anyhow::Result<()> {
        let request = self.queries.ask_inspect(page.clone(), point, pick);
        let asked = (self.hosts.get(page))
            .is_some_and(|&host| self.source.inspect_at(host, point, request).is_ok());
        if !asked && let Some(answer) = self.queries.inspect_answer(request, None) {
            self.drive(|app| app.send(answer))?;
        }
        Ok(())
    }

    /// Asks each page what the region grabbed in it; a page that cannot be
    /// asked grabbed nothing.
    fn query_region_grab(&mut self, region: Rect, pages: &[PageRegion]) -> anyhow::Result<()> {
        let ids = pages.iter().map(|covered| covered.page.clone()).collect();
        let requests = self.queries.ask_grab(region, ids);
        for (covered, request) in pages.iter().zip(requests) {
            let rect = css_rect(covered.rect);
            let asked = self.hosts.get(&covered.page).is_some_and(|&host| {
                (self.source.query_elements_in_rect(host, rect, request)).is_ok()
            });
            if !asked {
                self.queries.grab_answer(request, 0);
            }
        }
        self.answer_settled_grabs()
    }

    /// Takes a frame from every page and waits for the images and Documents
    /// asked for so far. Real pages are also waited for: each has to load,
    /// answer what it was asked and paint, and is then given a moment more,
    /// since the frame that follows a load is rarely the last.
    pub(super) fn settle(&mut self) -> anyhow::Result<()> {
        let deadline = Instant::now() + LOAD_TIMEOUT;
        let mut ready_since = None;
        loop {
            self.take_page_events()?;
            self.take_images()?;
            self.take_notes()?;
            let pages_ready = !self.live
                || (self.loading_pages.is_empty()
                    && self.unpainted_pages.is_empty()
                    && self.queries.is_idle());
            if self.loading_images.is_empty() && self.loading_notes.is_empty() && pages_ready {
                if !self.live {
                    return Ok(());
                }
                let now = Instant::now();
                if now.duration_since(*ready_since.get_or_insert(now)) >= PAGE_SETTLE {
                    return Ok(());
                }
            } else {
                ready_since = None;
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    images = self.loading_images.len(),
                    documents = self.loading_notes.len(),
                    pages_loading = self.loading_pages.len(),
                    pages_unpainted = self.unpainted_pages.len(),
                    "drawing without loads that never finished"
                );
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Lets real pages run for `time`: a script's `wait` is their only
    /// chance to animate, scroll or follow a link. Synthetic pages have
    /// nothing to wait for.
    pub(super) fn run_pages_for(&mut self, time: Duration) -> anyhow::Result<()> {
        if !self.live {
            return Ok(());
        }
        let until = Instant::now() + time.min(LOAD_TIMEOUT);
        while Instant::now() < until {
            self.take_page_events()?;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    /// Pumps the source and passes on what its pages reported.
    pub(super) fn take_page_events(&mut self) -> anyhow::Result<()> {
        let mut events = Vec::new();
        self.source.pump();
        self.source.drain_events(&mut events);
        for event in events {
            match &event {
                PageEvent::Loading {
                    page,
                    loading: true,
                    ..
                } => {
                    self.loading_pages.insert(*page);
                    self.unpainted_pages.insert(*page);
                }
                PageEvent::Loaded { page, .. } | PageEvent::Crashed { page, .. } => {
                    self.loading_pages.remove(page);
                }
                PageEvent::Frame(frame) if !self.loading_pages.contains(&frame.page) => {
                    self.unpainted_pages.remove(&frame.page);
                }
                PageEvent::ElementAt {
                    request, element, ..
                } => {
                    if let Some(answer) = self.queries.element_answer(*request, element.clone()) {
                        self.drive(|app| app.send(answer))?;
                    }
                }
                PageEvent::Inspected { request, node, .. } => {
                    if let Some(answer) = self.queries.inspect_answer(*request, node.clone()) {
                        self.drive(|app| app.send(answer))?;
                    }
                }
                PageEvent::ElementsInRect { request, count, .. } => {
                    self.queries.grab_answer(*request, *count);
                    self.answer_settled_grabs()?;
                }
                _ => {}
            }
            if let Some((host, notice)) = notice_of(&event, self.source.devtools_port())
                && let Some(page) = self.entity_of(host)
            {
                tracing::debug!(%page, ?notice, "page notice");
                self.drive(|app| app.send(Event::Page { page, notice }))?;
            }
            self.compositor.handle_page_event(event)?;
        }
        Ok(())
    }

    fn answer_settled_grabs(&mut self) -> anyhow::Result<()> {
        for answer in self.queries.settled() {
            self.drive(|app| app.send(answer))?;
        }
        Ok(())
    }

    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|&(_, &host)| host == page)?;
        Some(entity.clone())
    }

    fn raster_image(&mut self, image: ImageKey, file: &str, size: PixelSize) {
        self.loading_images.insert(image);
        let device = |logical: u32| ((logical as f32 * self.scale).ceil() as u32).max(1);
        let want = PixelSize::new(device(size.width), device(size.height));
        self.images
            .redraw(image, file, self.compositor.image_spec(), want);
    }

    fn drop_image(&mut self, image: ImageKey) {
        self.loading_images.remove(&image);
        self.images.forget(image);
        self.uploaded.remove(&mut self.compositor, image);
    }

    fn take_images(&mut self) -> anyhow::Result<()> {
        while let Some(image) = self.images.take_changed() {
            if self.loading_images.insert(image) {
                self.drive(|app| {
                    app.send(Event::Image {
                        image,
                        notice: ImageNotice::Changed,
                    })
                })?;
            }
        }
        while let Some(loaded) = self.images.take() {
            if !self.loading_images.remove(&loaded.key) {
                continue;
            }
            let notice = match loaded.result {
                Ok(content) => {
                    (self.uploaded).install(&mut self.compositor, loaded.key, &content)?
                }
                Err(LoadFailure::Missing) => ImageNotice::Missing,
                Err(LoadFailure::Failed) => ImageNotice::Failed,
            };
            let image = loaded.key;
            self.drive(|app| app.send(Event::Image { image, notice }))?;
        }
        Ok(())
    }

    fn take_notes(&mut self) -> anyhow::Result<()> {
        while let Some(read) = self.notes.take() {
            self.loading_notes.remove(&read.file);
            let notice = match read.result {
                Ok(text) => NoteNotice::Text(text),
                Err(ReadFailure::Missing) => NoteNotice::Missing,
                Err(ReadFailure::Failed) => NoteNotice::Failed,
            };
            let file = read.file;
            self.drive(|app| app.send(Event::Note { file, notice }))?;
        }
        Ok(())
    }
}
