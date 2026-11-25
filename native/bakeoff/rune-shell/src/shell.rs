//! [`Shell`]: the Rune app's state. An [`App`], its pages (synthetic or
//! CEF, see [`pages`](crate::pages)), and a cut-down runner for the effects
//! `update` returns.
//!
//! It stands in for `specular_shell::Runtime`, which the spike cannot use:
//! that runtime owns its window's compositor (`ShellWindow`), and here the
//! compositor only exists inside a Rune frame, on Rune's device. So nothing
//! in this file touches a GPU. What a compositor will need (page frames,
//! decoded images) waits here until [`canvas`](crate::canvas) takes it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Vec2;
use specular_compositor::GlyphMeasure;
use specular_core::{Camera, CssSize, PageEvent, PageId, PageSpec};
use specular_doc::{Document, EntityId};
use specular_interact::{
    Action, App, ControlId, Effect, Event, ImageKey, NoteNotice, resolve_file, update,
};
use specular_scene::{Scene, ViewCache};

use crate::files::{self, Decoded};
use crate::pages::{Pages, notice_of};

/// The clock `specular --snapshot` holds its app at.
pub const STILL_MS: u64 = 1_700_000_000_000;
/// The most page events kept while no compositor is taking them.
const MAX_WAITING: usize = 512;

/// Where the app's clock comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    /// Held at [`STILL_MS`], as the reference snapshot holds it.
    Still,
    /// [`STILL_MS`] plus Rune's seconds since start, which are virtual in a
    /// headless run.
    Running,
}

/// A decoded image and how many times its key has been loaded, so a
/// compositor can tell a redrawn svg from the one it uploaded.
#[derive(Debug, Clone)]
pub struct HeldImage {
    /// Goes up each time the key is decoded again.
    pub serial: u64,
    /// The pixels.
    pub decoded: Decoded,
}

/// The app and everything it does I/O through.
pub struct Shell {
    /// The app. Only [`Shell::dispatch`] changes it.
    pub app: App,
    /// The page source and what it reported.
    pub pages: Pages,
    /// The hosted page behind each page entity.
    pub hosts: HashMap<EntityId, PageId>,
    /// The folder relative file paths start from.
    space: Option<PathBuf>,
    /// What the pages reported that a compositor has not yet taken.
    pub page_events: Vec<PageEvent>,
    /// Pages closed since a compositor last looked.
    pub closed_pages: Vec<PageId>,
    /// The images the app holds, decoded.
    pub images: HashMap<ImageKey, HeldImage>,
    next_serial: u64,
    view_cache: ViewCache,
    note_heights: HashMap<EntityId, f32>,
    clock: Clock,
    /// The viewport in points.
    pub viewport: Vec2,
    /// Device pixels per point, once a frame has said.
    pub scale: f32,
    /// The dropdown of the chrome that is open. Renderer state, as the Kit
    /// keeps its own: the model does not say which list is open.
    pub open_list: Option<ControlId>,
    /// The chrome as a script, with `--chrome luau`. `None` is the Rust
    /// toolbar of [`chrome`](crate::chrome).
    pub panel: Option<crate::panel::Panel>,
    /// How many events have gone through `update`.
    pub dispatched: u64,
    /// The cursor and the input method, as effects last asked.
    pub asks: crate::input::Asks,
    /// When the window asked to close; the page source is shutting down.
    pub closing: Option<std::time::Instant>,
    /// The page source has shut down.
    pub closed: bool,
    /// `--probe-keys`: when to send digit 1 and keypad 1.
    pub probe_keys_at: Option<f64>,
}

impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell")
            .field("pages", &self.hosts.len())
            .field("images", &self.images.len())
            .finish_non_exhaustive()
    }
}

impl Shell {
    /// A shell with nothing open, for `AppSpec::init`.
    pub fn empty() -> Self {
        Self::new(crate::WINDOW, 1.0, Clock::Running)
    }

    /// A shell with an empty app and a viewport of `size` points.
    ///
    /// `scale` is the device pixels per point the first frame is expected
    /// at. Pages are hosted before any frame is drawn, and a page's texture
    /// is made at this scale; [`Shell::resized`] puts a wrong guess right.
    pub fn new(size: (f32, f32), scale: f32, clock: Clock) -> Self {
        let mut app = App::new(0);
        // Text is sized with the renderer's fonts from the start, before a
        // document is opened. `GlyphMeasure::new` loads the same fonts the
        // compositor draws with and needs no device, which matters here:
        // Rune hands over its device only inside the first frame.
        app.set_text_measure(Arc::new(GlyphMeasure::new()));
        let mut shell = Self {
            app,
            pages: Pages::synthetic(),
            hosts: HashMap::new(),
            space: None,
            page_events: Vec::new(),
            closed_pages: Vec::new(),
            images: HashMap::new(),
            next_serial: 0,
            view_cache: ViewCache::default(),
            note_heights: HashMap::new(),
            clock,
            viewport: Vec2::new(size.0, size.1),
            scale,
            open_list: None,
            panel: None,
            dispatched: 0,
            asks: crate::input::Asks::default(),
            closing: None,
            closed: false,
            probe_keys_at: None,
        };
        shell.dispatch(Event::Motion(false));
        shell.dispatch(Event::ViewportResized(shell.viewport));
        shell
    }

    /// Opens the `.canvas` file at `path`, at `camera` or fitted.
    pub fn open(&mut self, path: &Path, camera: Option<Camera>) -> Result<(), String> {
        let json = std::fs::read_to_string(path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        let document = Document::from_canvas_str(&json)
            .map_err(|error| format!("parsing {}: {error}", path.display()))?;
        self.space = std::path::absolute(path)
            .ok()
            .and_then(|path| path.parent().map(Path::to_owned));
        if let Some(folder) = &self.space {
            self.dispatch(Event::DocumentFolder(folder.to_string_lossy().into_owned()));
        }
        self.dispatch(Event::DocumentOpened(Box::new(document)));
        self.act(camera.map_or(Action::ZoomToFit, Action::SetCamera));
        Ok(())
    }

    /// Sends `event` through `update` and runs what comes back.
    pub fn dispatch(&mut self, event: Event) {
        self.dispatched += 1;
        for effect in update(&mut self.app, event) {
            self.run(effect);
        }
    }

    /// What a control of the chrome does when pressed.
    pub fn act(&mut self, action: Action) {
        self.dispatch(Event::Action(action));
    }

    /// The viewport changed, in points and device pixels per point.
    pub fn resized(&mut self, viewport: Vec2, scale: f32) {
        if scale != self.scale {
            self.scale = scale;
            for host in self.hosts.values() {
                let _ = self.pages.get().set_texture_scale(*host, scale);
                let _ = self.pages.get().set_painting(*host, true);
            }
        }
        if viewport != self.viewport {
            self.viewport = viewport;
            self.dispatch(Event::ViewportResized(viewport));
        }
    }

    /// The scene to draw now.
    pub fn scene(&self) -> Scene {
        specular_scene::view(&self.app, self.viewport, &self.view_cache)
    }

    /// Tells the app how tall each Document's rows came out, when that has
    /// changed. Returns whether it did, which moves what is below them.
    pub fn report_note_heights(&mut self, heights: &[(EntityId, f32)]) -> bool {
        let changed: Vec<_> = (heights.iter())
            .filter(|(entity, height)| self.note_heights.get(entity) != Some(height))
            .cloned()
            .collect();
        if changed.is_empty() {
            return false;
        }
        self.note_heights.extend(changed.iter().cloned());
        self.dispatch(Event::NoteHeights(changed));
        true
    }

    /// One turn of everything that is not drawing: the clock and the pages.
    /// `seconds` is Rune's time since start.
    pub fn turn(&mut self, seconds: f64) {
        let unix_ms = match self.clock {
            Clock::Still => STILL_MS,
            Clock::Running => STILL_MS + (seconds * 1_000.0) as u64,
        };
        self.dispatch(Event::Tick { unix_ms });
        let mut events = Vec::new();
        self.pages.take_events(&mut events);
        for event in events {
            // The reference snapshot shows each synthetic page's first
            // frame. With the clock held, so does this.
            if self.clock == Clock::Still
                && let PageEvent::Frame(frame) = &event
            {
                let _ = self.pages.get().set_painting(frame.page, false);
            }
            if let Some((host, notice)) = notice_of(&event)
                && let Some(page) = self.entity_of(host)
            {
                if let specular_interact::PageNotice::Title(title) = &notice {
                    eprintln!("rune-shell: page {page} title {title}");
                }
                self.dispatch(Event::Page { page, notice });
            }
            self.page_events.push(event);
        }
        // With no compositor (a `Runtime` test), nothing takes these.
        let waiting = self.page_events.len();
        if waiting > MAX_WAITING {
            self.page_events.drain(..waiting - MAX_WAITING);
        }
    }

    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|(_, host)| **host == page)?;
        Some(entity.clone())
    }

    /// Runs one effect. Only what a canvas needs to be shown is run; the
    /// rest (saving, the clipboard, agents, sharing, settings) is dropped,
    /// as a headless run of the real shell drops what would write.
    fn run(&mut self, effect: Effect) {
        if self.asks.note(&effect) {
            return;
        }
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => self.create_page(&page, &url, viewport),
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    let _ = self.pages.get().close_page(host);
                    self.closed_pages.push(host);
                }
            }
            Effect::SetPageViewport { page, viewport } => {
                if let Some(host) = self.hosts.get(&page) {
                    let _ = self.pages.get().set_viewport(*host, viewport);
                }
            }
            Effect::FocusPage(page) => {
                let host = page.and_then(|page| self.hosts.get(&page).copied());
                let _ = self.pages.get().set_focus(host);
            }
            Effect::ForwardInput { page, event } => {
                if let Some(host) = self.hosts.get(&page) {
                    let _ = self.pages.get().send_input(*host, &event);
                }
            }
            Effect::LoadImage { image, file } => self.load_image(image, &file, None),
            Effect::RasterImage { image, file, size } => {
                let device = |logical: u32| ((logical as f32 * self.scale).ceil() as u32).max(1);
                let want = specular_core::PixelSize::new(device(size.width), device(size.height));
                self.load_image(image, &file, Some(want));
            }
            Effect::DropImage(image) => {
                self.images.remove(&image);
            }
            Effect::LoadNote { file } => self.load_note(file),
            Effect::TrackElements { page, selectors } => {
                if let Some(host) = self.hosts.get(&page) {
                    let _ = self.pages.get().track_elements(*host, &selectors);
                }
            }
            _ => {}
        }
    }

    fn create_page(&mut self, page: &EntityId, url: &str, viewport: CssSize) {
        let mut spec = PageSpec::new(url, viewport);
        spec.texture_scale = self.scale;
        let mut device = self.app.page_device(page);
        device.pixel_ratio = device.pixel_ratio.or(Some(self.scale));
        spec.device = device;
        match self.pages.get().create_page(&spec) {
            Ok(host) => {
                self.hosts.insert(page.clone(), host);
            }
            Err(error) => eprintln!("rune-shell: creating page for {url}: {error}"),
        }
    }

    fn load_image(&mut self, image: ImageKey, file: &str, want: Option<specular_core::PixelSize>) {
        let loaded = resolve_file(file, self.space.as_deref())
            .ok_or(files::Failure::Failed)
            .and_then(|path| files::load(&path, want));
        let notice = match loaded {
            Ok(decoded) => {
                let notice = decoded.notice.clone();
                self.next_serial += 1;
                let serial = self.next_serial;
                self.images.insert(image, HeldImage { serial, decoded });
                notice
            }
            Err(failure) => failure.notice(),
        };
        self.dispatch(Event::Image { image, notice });
    }

    fn load_note(&mut self, file: String) {
        let read = resolve_file(&file, self.space.as_deref())
            .ok_or(std::io::ErrorKind::InvalidInput.into())
            .and_then(std::fs::read_to_string);
        let notice = match read {
            Ok(text) => NoteNotice::Text(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => NoteNotice::Missing,
            Err(_) => NoteNotice::Failed,
        };
        self.dispatch(Event::Note { file, notice });
    }
}

/// `AppSpec::poll`: Rune calls it at the start of every frame, and when
/// the loop is woken (a page painted, with CEF in a window).
pub fn poll(shell: &mut Shell, seconds: f64, tx: &mut rune::el::Tx) -> bool {
    #[cfg(feature = "cef")]
    if shell.pages.spin {
        crate::cef::spin();
    }
    if let Some(asked) = shell.closing {
        if shell.closed {
            return false;
        }
        shell.pages.get().pump();
        if shell.pages.poll_shutdown() {
            shell.closed = true;
            eprintln!(
                "rune-shell: page source shut down {} ms after the close",
                asked.elapsed().as_millis()
            );
            tx.close_window();
        } else {
            tx.wake_after(std::time::Duration::from_millis(8));
        }
        return false;
    }
    if shell.probe_keys_at.is_some_and(|at| seconds >= at) {
        shell.probe_keys_at = None;
        crate::keys::probe(shell);
    }
    let before = shell.dispatched;
    let waiting = shell.page_events.len();
    shell.turn(seconds);
    // A synthetic page paints on every pump; only a real one asks for a
    // frame when it does.
    let painted = shell.pages.name == "cef" && shell.page_events.len() > waiting;
    let scripted = crate::panel::poll(shell, seconds, tx);
    // The tick alone changes nothing the chrome shows.
    scripted || painted || shell.dispatched > before + 1
}
