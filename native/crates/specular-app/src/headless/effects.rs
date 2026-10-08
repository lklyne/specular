//! Running what the app asks for in a headless run: pages on the synthetic
//! source, images and Documents from the shell's loader threads, and the
//! clipboard and new Documents in memory.

use std::time::{Duration, Instant};

use anyhow::Context as _;
use specular_core::{PageEvent, PageId, PageSource as _, PageSpec};
use specular_doc::EntityId;
use specular_interact::{ClipboardContent, Effect, Event, ImageNotice, NoteNotice, PageNotice};
use specular_scene::ImageId;

use super::Headless;
use crate::images::LoadFailure;
use crate::notes::ReadFailure;

/// How long a snapshot waits for images and Documents before drawing
/// without them.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

impl Headless {
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
                self.hosts.insert(page, host);
            }
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    self.source.close_page(host)?;
                    self.compositor.remove_page(host);
                }
            }
            Effect::SetPageViewport { page, viewport } => {
                if let Some(&host) = self.hosts.get(&page) {
                    self.source.set_viewport(host, viewport)?;
                }
            }
            Effect::LoadImage { image, file } => {
                self.loading_images.insert(image);
                self.images
                    .request(image, &file, self.compositor.image_spec());
            }
            Effect::DropImage(image) => {
                self.loading_images.remove(&image);
                self.compositor.remove_image(ImageId(image.0));
            }
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
            // A headless run has no window to focus, no cursor and no input
            // method, it leaves the disk alone, and it hosts no API.
            Effect::ApiReply { .. }
            | Effect::FocusPage(_)
            | Effect::ForwardInput { .. }
            | Effect::SetImeAllowed(_)
            | Effect::SetImeCursorArea { .. }
            | Effect::SetCursor(_)
            | Effect::Save
            | Effect::WriteAsset { .. }
            | Effect::CopyAsset { .. }
            | Effect::SaveToolDefaults(_) => {}
        }
        Ok(())
    }

    /// Takes a frame from every page and waits for the images and Documents
    /// asked for so far.
    pub(super) fn settle(&mut self) -> anyhow::Result<()> {
        let deadline = Instant::now() + LOAD_TIMEOUT;
        loop {
            self.take_page_events()?;
            self.take_images()?;
            self.take_notes()?;
            if self.loading_images.is_empty() && self.loading_notes.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    images = self.loading_images.len(),
                    documents = self.loading_notes.len(),
                    "drawing without loads that never finished"
                );
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn take_page_events(&mut self) -> anyhow::Result<()> {
        let mut events = Vec::new();
        self.source.pump();
        self.source.drain_events(&mut events);
        for event in events {
            if let PageEvent::Loaded { page, http_status } = &event
                && let Some(entity) = self.entity_of(*page)
            {
                let notice = PageNotice::Loaded {
                    http_status: *http_status,
                };
                self.drive(|app| {
                    app.send(Event::Page {
                        page: entity,
                        notice,
                    })
                })?;
            }
            self.compositor.handle_page_event(event)?;
        }
        Ok(())
    }

    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|&(_, &host)| host == page)?;
        Some(entity.clone())
    }

    fn take_images(&mut self) -> anyhow::Result<()> {
        while let Some(loaded) = self.images.take() {
            if !self.loading_images.remove(&loaded.key) {
                continue;
            }
            let notice = match loaded.result {
                Ok(mips) => {
                    self.compositor
                        .set_image_mips(ImageId(loaded.key.0), &mips)?;
                    ImageNotice::Ready {
                        width: mips.size().width,
                        height: mips.size().height,
                    }
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
