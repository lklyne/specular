//! [`Viewer`]: the app, driven by a browser, that can look and comment but
//! not change the canvas.
//!
//! The canvas is the desktop app's own [`App`], fed the browser's input as
//! [`Event`]s and answered for what it asks with what the published bundle
//! holds: a page is its snapshot, an image or a Document is its published
//! file, and a page asked what element is under a point has none. Nothing
//! here touches the GPU; what has to reach the renderer comes out as
//! [`Upload`]s.
//!
//! What a viewer may do is held two ways. Input is filtered first: in pan
//! mode a drag moves the camera and a click only focuses a comment, and keys
//! reach the app only while a comment is being written. After every input
//! the canvas is checked against the last one accepted, so whatever slips
//! through is put back: an entity, an edge or the stack order that changed,
//! or a comment that was edited or removed. A comment that is new is kept,
//! when the link allows comments, and goes out through
//! [`Viewer::take_new_comments`].

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use glam::Vec2;
use specular_core::{
    Camera, CpuFrame, CssSize, Modifiers, PageId, PixelSize, PointerButton, PointerEventKind,
};
use specular_doc::{Annotation, AnnotationId, CanvasError, Command, Document, EntityId};
use specular_interact::{
    Action, App, Appearance, Effect, Event, ImageKey, ImageNotice, KeyInput, NoteNotice,
    PageNotice, PointerInput, TextMeasure, Tool, WheelInput, update,
};

/// How far a press may move, in logical pixels, and still be a click.
const CLICK_SLOP: f32 = 3.0;

/// What the share link a viewer opened lets it do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Look only.
    View,
    /// Look and comment.
    Comment,
}

/// What a press on the canvas does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// A drag pans and a click focuses the comment under it.
    #[default]
    Pan,
    /// The app's comment tool: a click comments on a point, a drag on a
    /// region.
    Comment,
}

/// What the renderer has to be given.
#[derive(Debug)]
pub enum Upload {
    /// A page's snapshot, shown as its only frame.
    PageFrame {
        /// The renderer's page.
        page: PageId,
        /// The CSS size the page is laid out at.
        viewport: CssSize,
        /// Premultiplied BGRA texels.
        frame: CpuFrame,
    },
    /// A page is no longer shown.
    ClosePage(PageId),
    /// A decoded image: straight-alpha RGBA, `size.width * 4` bytes a row.
    Image {
        /// The texture's key.
        key: ImageKey,
        /// Its size.
        size: PixelSize,
        /// The texels.
        rgba: Vec<u8>,
    },
    /// An image is no longer shown.
    DropImage(ImageKey),
}

/// An svg for the page to draw: the file `file`, into `want` pixels or, when
/// there is no `want`, at its own size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgJob {
    /// The texture's key.
    pub image: ImageKey,
    /// The path the canvas writes it with.
    pub file: String,
    /// The device pixels to fill, keeping its proportions.
    pub want: Option<PixelSize>,
}

/// Why a viewer could not be made.
#[derive(Debug, thiserror::Error)]
pub enum ViewerError {
    /// The published `.canvas` could not be read.
    #[error("the canvas could not be read: {0:?}")]
    Canvas(CanvasError),
    /// A comment from the server is not an annotation.
    #[error("a comment could not be read: {0}")]
    Comment(#[from] serde_json::Error),
}

/// A page the app asked to host.
#[derive(Debug, Clone, Copy)]
struct Host {
    page: PageId,
    viewport: CssSize,
}

/// A press in pan mode.
#[derive(Debug, Clone, Copy)]
struct Press {
    start: Vec2,
    last: Vec2,
    moved: bool,
}

/// See the module docs.
#[derive(Debug)]
pub struct Viewer {
    app: App,
    access: Access,
    mode: Mode,
    /// The canvas as last accepted. Input that changes anything else is put
    /// back to it.
    accepted: Document,
    /// The published files by the path the canvas writes them with.
    files: HashMap<String, Arc<[u8]>>,
    /// Whether every published file has arrived, so one not among them is
    /// missing rather than late.
    files_complete: bool,
    /// Each page's snapshot PNG.
    snapshots: HashMap<EntityId, Arc<[u8]>>,
    hosts: HashMap<EntityId, Host>,
    next_page: u64,
    /// Images to answer once their files arrive, with the device pixels an
    /// svg is drawn at when the app asked for a size.
    images_waiting: Vec<(ImageKey, String, Option<PixelSize>)>,
    /// Svgs for the page to draw, and the ones it is drawing.
    svg_jobs: Vec<SvgJob>,
    svgs_drawing: HashSet<ImageKey>,
    notes_waiting: Vec<String>,
    uploads: Vec<Upload>,
    /// The comments the canvas was published with: the desktop's own.
    published: Vec<Annotation>,
    /// The comments the server holds. One with the id of a published
    /// comment is the newer of the two.
    remote: Vec<Annotation>,
    /// Comments made here that the server has yet to list.
    posted: Vec<Annotation>,
    /// Comments made here that have yet to be sent.
    outbox: Vec<Annotation>,
    /// The server's list waited for a draft to close before it was shown.
    comments_waiting: bool,
    press: Option<Press>,
    viewport: Option<Vec2>,
    /// Device pixels to a logical pixel.
    scale: f32,
    cursor: specular_core::Cursor,
    /// Whether something changed that a frame has not shown yet.
    dirty: bool,
}

impl Viewer {
    /// Opens the published `.canvas` text. `id_seed` starts the ids of the
    /// comments made here; pass something random, since every viewer of the
    /// canvas draws from its own.
    pub fn new(canvas: &str, access: Access, id_seed: u64) -> Result<Self, ViewerError> {
        let document = Document::from_canvas_str(canvas).map_err(ViewerError::Canvas)?;
        let mut viewer = Self {
            app: App::new(id_seed),
            access,
            mode: Mode::Pan,
            accepted: document.clone(),
            files: HashMap::new(),
            files_complete: false,
            snapshots: HashMap::new(),
            hosts: HashMap::new(),
            next_page: 1,
            images_waiting: Vec::new(),
            svg_jobs: Vec::new(),
            svgs_drawing: HashSet::new(),
            notes_waiting: Vec::new(),
            uploads: Vec::new(),
            published: document.annotations().to_vec(),
            remote: Vec::new(),
            posted: Vec::new(),
            outbox: Vec::new(),
            comments_waiting: false,
            press: None,
            viewport: None,
            scale: 1.0,
            cursor: specular_core::Cursor::Default,
            dirty: true,
        };
        viewer.send(Event::DocumentOpened(Box::new(document)));
        viewer.accept();
        Ok(viewer)
    }

    fn can_comment(&self) -> bool {
        self.access == Access::Comment
    }

    /// The app as it stands, for the scene.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Lays text out with the renderer's own fonts, so a comment's caret
    /// sits where its glyphs are drawn.
    pub fn set_text_measure(&mut self, measure: Arc<dyn TextMeasure>) {
        self.app.set_text_measure(measure);
    }

    /// The canvas element is `viewport` logical pixels, each `scale` device
    /// pixels. The first size fits the canvas in it.
    pub fn resize(&mut self, viewport: Vec2, scale: f32) {
        let first = self.viewport.is_none();
        self.viewport = Some(viewport);
        self.scale = scale;
        self.send(Event::ViewportResized(viewport));
        if first {
            self.act(Action::ZoomToFit);
        }
    }

    /// Whether the canvas has to be drawn again. Reading it clears it.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// The cursor the canvas wants under the pointer.
    pub fn cursor(&self) -> specular_core::Cursor {
        if self.press.is_some_and(|press| press.moved) {
            specular_core::Cursor::Grabbing
        } else if self.mode == Mode::Pan {
            match self.cursor {
                specular_core::Cursor::Pointer => specular_core::Cursor::Pointer,
                _ => specular_core::Cursor::Grab,
            }
        } else {
            self.cursor
        }
    }

    /// What a press does now.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Switches what a press does. Commenting needs a link that allows it;
    /// without one this returns `false` and nothing changes.
    pub fn set_mode(&mut self, mode: Mode) -> bool {
        if mode == Mode::Comment && !self.can_comment() {
            return false;
        }
        self.mode = mode;
        self.press = None;
        let tool = match mode {
            Mode::Pan => Tool::Select,
            Mode::Comment => Tool::Comment,
        };
        self.act(Action::SetTool(tool));
        true
    }

    /// A published file arrived: an image or a markdown file the canvas
    /// names, by the path it writes.
    pub fn add_file(&mut self, path: &str, bytes: Vec<u8>) {
        self.files.insert(path.to_owned(), bytes.into());
        self.answer_waiting();
    }

    /// Every published file has arrived. One still asked for is missing.
    pub fn files_complete(&mut self) {
        self.files_complete = true;
        self.answer_waiting();
    }

    /// A page's snapshot arrived: the PNG it was published with.
    pub fn add_snapshot(&mut self, page: &str, png: Vec<u8>) {
        let page = EntityId::new(page);
        self.snapshots.insert(page.clone(), png.into());
        if let Some(host) = self.hosts.get(&page).copied() {
            self.show_snapshot(&page, host);
        }
    }

    /// What the renderer has to be given since the last call.
    pub fn take_uploads(&mut self) -> Vec<Upload> {
        std::mem::take(&mut self.uploads)
    }

    /// The svgs the page has to draw since the last call. Each is answered
    /// with [`Viewer::svg_drawn`] or [`Viewer::svg_failed`].
    pub fn take_svg_jobs(&mut self) -> Vec<SvgJob> {
        std::mem::take(&mut self.svg_jobs)
    }

    /// The page drew the svg `image` into `size` pixels of straight-alpha
    /// RGBA. `intrinsic` is the size its own `width`, `height` and `viewBox`
    /// give it.
    pub fn svg_drawn(
        &mut self,
        image: ImageKey,
        intrinsic: PixelSize,
        size: PixelSize,
        rgba: Vec<u8>,
    ) {
        if !self.svgs_drawing.remove(&image) {
            return;
        }
        self.uploads.push(Upload::Image {
            key: image,
            size,
            rgba,
        });
        self.send(Event::Image {
            image,
            notice: ImageNotice::Ready {
                width: intrinsic.width,
                height: intrinsic.height,
            },
        });
    }

    /// The page could not draw the svg `image`.
    pub fn svg_failed(&mut self, image: ImageKey) {
        if self.svgs_drawing.remove(&image) {
            self.send(Event::Image {
                image,
                notice: ImageNotice::Failed,
            });
        }
    }

    /// The pointer moved, pressed or released at `screen`.
    pub fn pointer(&mut self, kind: PointerEventKind, screen: Vec2, modifiers: Modifiers) {
        let pans = |button| button == PointerButton::Middle;
        match kind {
            PointerEventKind::Down { button, .. }
                if self.mode == Mode::Pan || pans(button) || self.press.is_some() =>
            {
                self.press = Some(Press {
                    start: screen,
                    last: screen,
                    moved: false,
                });
                self.dirty = true;
            }
            PointerEventKind::Move if self.press.is_some() => self.pan_to(screen),
            PointerEventKind::Up { .. } if self.press.is_some() => {
                if let Some(press) = self.press.take()
                    && !press.moved
                    && self.mode == Mode::Pan
                {
                    let comment = self.app.comment_mark_at(screen);
                    self.act(Action::FocusComment(comment));
                }
                self.dirty = true;
            }
            // A double click in pan mode would start editing text.
            PointerEventKind::Down { button, .. } if self.mode == Mode::Comment => {
                self.input(Event::Pointer(PointerInput {
                    kind: PointerEventKind::Down {
                        button,
                        click_count: 1,
                    },
                    screen,
                    modifiers,
                }));
            }
            PointerEventKind::Up { button, .. } => {
                self.input(Event::Pointer(PointerInput {
                    kind: PointerEventKind::Up {
                        button,
                        click_count: 1,
                    },
                    screen,
                    modifiers,
                }));
            }
            PointerEventKind::Down { .. } | PointerEventKind::Move | PointerEventKind::Leave => {
                self.input(Event::Pointer(PointerInput {
                    kind,
                    screen,
                    modifiers,
                }));
            }
        }
    }

    fn pan_to(&mut self, screen: Vec2) {
        let Some(press) = self.press.as_mut() else {
            return;
        };
        press.moved |= screen.distance(press.start) > CLICK_SLOP;
        if !press.moved {
            return;
        }
        let delta = screen - press.last;
        press.last = screen;
        let camera = self.app.session().camera;
        self.act(Action::SetCamera(Camera::new(
            camera.pan + delta,
            camera.zoom,
        )));
    }

    /// A wheel or trackpad scroll. Held Cmd or Control zooms, as it does on
    /// the desktop; a browser reports a trackpad pinch that way too.
    pub fn wheel(&mut self, delta: Vec2, modifiers: Modifiers) {
        self.input(Event::Wheel(WheelInput { delta, modifiers }));
    }

    /// Two fingers on a touch screen: their midpoint is at `centre` and moved
    /// by `moved`, and they are `spread` times as far apart as they were.
    /// The canvas follows the fingers, zooming about their midpoint.
    pub fn touch_gesture(&mut self, centre: Vec2, moved: Vec2, spread: f32) {
        self.press = None;
        self.send(Event::Pointer(PointerInput {
            kind: PointerEventKind::Move,
            screen: centre,
            modifiers: Modifiers::default(),
        }));
        self.send(Event::Pinch {
            delta: spread - 1.0,
        });
        self.send(Event::Wheel(WheelInput {
            delta: moved,
            modifiers: Modifiers::default(),
        }));
        self.dirty = true;
    }

    /// A key went down or up. Returns whether the viewer used it, so the
    /// page can keep the browser from acting on it too.
    pub fn key(&mut self, input: KeyInput) -> bool {
        use specular_interact::Key;
        let shortcut = input.modifiers.meta || input.modifiers.control;
        // Undo would step back through the canvas, which a viewer has no
        // history of.
        if shortcut && matches!(input.key, Key::Char('z' | 'y')) {
            return true;
        }
        if self.app.comment_draft().is_some() {
            self.input(Event::Key(input));
            return true;
        }
        if !input.pressed {
            return false;
        }
        let action = match input.key {
            Key::Escape => Action::Cancel,
            Key::Char('=' | '+') if shortcut => Action::ZoomIn,
            Key::Char('-') if shortcut => Action::ZoomOut,
            Key::Char('0') if shortcut => Action::ZoomReset,
            Key::Char('1') if input.modifiers.shift => Action::ZoomToFit,
            _ => return false,
        };
        self.act(action);
        true
    }

    /// The wall clock, once a frame.
    pub fn tick(&mut self, unix_ms: u64) {
        self.send(Event::Tick { unix_ms });
        if self.comments_waiting && self.app.comment_draft().is_none() {
            self.show_comments();
        }
    }

    /// Fits the whole canvas in the view.
    pub fn zoom_to_fit(&mut self) {
        self.act(Action::ZoomToFit);
    }

    /// Zooms one step in or out about the middle of the view, as the app's
    /// zoom menu does.
    pub fn zoom_step(&mut self, closer: bool) {
        self.act(if closer {
            Action::ZoomIn
        } else {
            Action::ZoomOut
        });
    }

    /// Zooms to 100% about the middle of the view.
    pub fn zoom_reset(&mut self) {
        self.act(Action::ZoomReset);
    }

    /// Draws the canvas in the dark theme or the light, as the browser
    /// prefers. A canvas follows the system the way the app's does.
    pub fn set_dark(&mut self, dark: bool) {
        self.send(Event::SystemAppearance(if dark {
            Appearance::Dark
        } else {
            Appearance::Light
        }));
    }

    /// Gives the comment `id` the focus, or with `None` takes it away.
    pub fn focus_comment(&mut self, id: Option<AnnotationId>) {
        self.act(Action::FocusComment(id));
    }

    /// Brings the comment `id` into view and gives it the focus.
    pub fn reveal_comment(&mut self, id: AnnotationId) {
        self.act(Action::RevealComment(id));
    }

    /// The comment that has the focus.
    pub fn focused_comment(&self) -> Option<&AnnotationId> {
        self.app.focused_comment()
    }

    /// The comments the server holds now, oldest first. They replace the
    /// ones shown, once no comment is being written.
    pub fn set_comments(&mut self, comments: Vec<Annotation>) {
        let listed: HashSet<&AnnotationId> = comments.iter().map(|each| &each.id).collect();
        self.posted.retain(|each| !listed.contains(&each.id));
        self.remote = comments;
        self.comments_waiting = true;
        if self.app.comment_draft().is_none() {
            self.show_comments();
        }
    }

    /// The comments made here since the last call, for the page to send.
    pub fn take_new_comments(&mut self) -> Vec<Annotation> {
        std::mem::take(&mut self.outbox)
    }

    /// Every comment on the canvas: the ones it was published with, the
    /// server's, and the ones made here that the server has yet to list.
    fn comments(&self) -> Vec<Annotation> {
        let listed: HashSet<&AnnotationId> = self.remote.iter().map(|each| &each.id).collect();
        (self.published.iter())
            .filter(|each| !listed.contains(&each.id))
            .chain(&self.remote)
            .chain(&self.posted)
            .cloned()
            .collect()
    }

    fn show_comments(&mut self) {
        self.comments_waiting = false;
        let comments = self.comments();
        if self.app.document().annotations() == comments.as_slice() {
            return;
        }
        let focused = self.app.focused_comment().cloned();
        let document = with_comments(self.app.document().clone(), comments);
        self.send(Event::DocumentOpened(Box::new(document)));
        self.accept();
        if focused.is_some() {
            self.act(Action::FocusComment(focused));
        }
    }

    fn act(&mut self, action: Action) {
        self.input(Event::Action(action));
    }

    /// Sends input from the viewer, then holds the canvas to what a viewer
    /// may change.
    fn input(&mut self, event: Event) {
        let checks = !matches!(
            event,
            Event::Pointer(PointerInput {
                kind: PointerEventKind::Move | PointerEventKind::Leave,
                ..
            }) | Event::Wheel(_)
        ) || self.app.session().gesture.is_some();
        self.send(event);
        if checks {
            self.hold();
        }
    }

    /// Keeps a new comment the link allows and puts back anything else that
    /// input changed.
    fn hold(&mut self) {
        let document = self.app.document();
        let canvas_kept = document.entities().eq(self.accepted.entities())
            && document.edges().eq(self.accepted.edges())
            && document.order() == self.accepted.order();
        let before: HashMap<&AnnotationId, &Annotation> = (self.accepted.annotations().iter())
            .map(|each| (&each.id, each))
            .collect();
        let now: HashSet<&AnnotationId> =
            document.annotations().iter().map(|each| &each.id).collect();
        let kept = before
            .iter()
            .all(|(id, was)| document.annotation(id) == Some(*was) && now.contains(id));
        let new: Vec<Annotation> = (document.annotations().iter())
            .filter(|each| !before.contains_key(&each.id))
            .cloned()
            .collect();
        if canvas_kept && kept && (new.is_empty() || self.can_comment()) {
            if !new.is_empty() {
                self.posted.extend(new.iter().cloned());
                self.outbox.extend(new);
                self.accept();
            }
            return;
        }
        tracing::debug!("put back a change a viewer cannot make");
        let mut comments = self.comments();
        if self.can_comment() {
            self.posted.extend(new.iter().cloned());
            self.outbox.extend(new.iter().cloned());
            comments.extend(new);
        }
        let document = with_comments(self.accepted.clone(), comments);
        self.send(Event::DocumentOpened(Box::new(document)));
        self.accept();
    }

    fn accept(&mut self) {
        self.accepted = self.app.document().clone();
    }

    /// Sends `event` and runs what it asks for, and what that asks for.
    fn send(&mut self, event: Event) {
        // A canvas that reads its own markdown back, or fits a Document to
        // its text, changes without input: that is accepted as it comes.
        let unprompted = matches!(
            event,
            Event::Note { .. } | Event::Image { .. } | Event::NoteHeights(_)
        );
        let mut queue = vec![event];
        while let Some(event) = queue.pop() {
            self.dirty |= !matches!(event, Event::Tick { .. });
            let camera = self.app.session().camera;
            for effect in update(&mut self.app, event) {
                self.run(effect, &mut queue);
            }
            self.dirty |= self.app.session().camera != camera;
        }
        if unprompted {
            self.accept();
        }
    }

    fn run(&mut self, effect: Effect, queue: &mut Vec<Event>) {
        match effect {
            Effect::CreatePage { page, viewport, .. } => {
                let host = Host {
                    page: PageId(self.next_page),
                    viewport,
                };
                self.next_page += 1;
                self.hosts.insert(page.clone(), host);
                if self.snapshots.contains_key(&page) {
                    self.show_snapshot(&page, host);
                }
            }
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    self.uploads.push(Upload::ClosePage(host.page));
                }
            }
            Effect::SetPageViewport { page, viewport } => {
                if let Some(host) = self.hosts.get_mut(&page) {
                    host.viewport = viewport;
                    let host = *host;
                    if self.snapshots.contains_key(&page) {
                        self.show_snapshot(&page, host);
                    }
                }
            }
            Effect::LoadImage { image, file } => self.ask_image(image, file, None),
            Effect::RasterImage { image, file, size } => self.ask_image(image, file, Some(size)),
            Effect::DropImage(image) => {
                self.images_waiting
                    .retain(|(waiting, ..)| *waiting != image);
                self.svg_jobs.retain(|job| job.image != image);
                self.svgs_drawing.remove(&image);
                self.uploads.push(Upload::DropImage(image));
            }
            Effect::LoadNote { file } => {
                self.notes_waiting.push(file);
                self.answer_waiting();
            }
            Effect::DropNote { file } => self.notes_waiting.retain(|waiting| *waiting != file),
            // A snapshot has no elements: a comment there is on the point.
            Effect::QueryElement { page, point } => queue.push(Event::ElementAt {
                page,
                point,
                element: None,
            }),
            Effect::QueryRegionGrab { region, .. } => queue.push(Event::RegionGrab {
                region,
                grabs: Vec::new(),
            }),
            Effect::SetCursor(cursor) => self.cursor = cursor,
            // There is no page to talk to, no disk, no clipboard and no
            // agent.
            Effect::ApiReply { .. }
            | Effect::AgentGo { .. }
            | Effect::Navigate { .. }
            | Effect::AskScrollProgress(_)
            | Effect::AskScrollRoom { .. }
            | Effect::ScrollPage { .. }
            | Effect::CapturePage(_)
            | Effect::AskCandidates { .. }
            | Effect::ReplayPointer { .. }
            | Effect::FocusPage(_)
            | Effect::ForwardInput { .. }
            | Effect::EditField(_)
            | Effect::SetImeAllowed(_)
            | Effect::SetImeCursorArea { .. }
            | Effect::Save
            | Effect::WriteCanvas(_)
            | Effect::RenameCanvasFile { .. }
            | Effect::TrashCanvasFile { .. }
            | Effect::SaveSpaceMeta
            | Effect::WriteClipboard(_)
            | Effect::ReadClipboard
            | Effect::WriteAsset { .. }
            | Effect::CopyAsset { .. }
            | Effect::WriteNote { .. }
            | Effect::CreateNote { .. }
            | Effect::InspectAt { .. }
            | Effect::SaveRepos
            | Effect::PickRepoFolder { .. }
            | Effect::ChooseSpace { .. }
            | Effect::OpenSpace(_)
            | Effect::RevealSpace
            | Effect::Quit
            | Effect::SaveSettings(_)
            | Effect::PreviewRegion { .. }
            | Effect::CaptureElement { .. }
            | Effect::TrackElements { .. }
            | Effect::SaveToolDefaults(_)
            | Effect::SaveAgentModel(_)
            | Effect::SaveTheme(_)
            | Effect::SetPageColorScheme { .. }
            | Effect::SetPageDevice { .. }
            | Effect::LoadThreads
            | Effect::WriteThread(_)
            | Effect::WriteThreadIndex
            | Effect::RunAgent(_)
            | Effect::CancelAgent(_) => {}
        }
    }

    /// Answers `image` once its file arrives, an svg drawn at `size`
    /// logical pixels when the app asks for one. A browser draws svgs, with
    /// its own fonts, so they are handed to the page.
    fn ask_image(&mut self, image: ImageKey, file: String, size: Option<PixelSize>) {
        let device = |logical: u32| ((logical as f32 * self.scale).ceil() as u32).max(1);
        let want = size.map(|size| PixelSize::new(device(size.width), device(size.height)));
        self.images_waiting.push((image, file, want));
        self.answer_waiting();
    }

    /// Answers the images and Documents asked for whose files have arrived,
    /// and, once every file has, the ones that are not there.
    fn answer_waiting(&mut self) {
        let mut events = Vec::new();
        let complete = self.files_complete;
        let files = &self.files;
        let uploads = &mut self.uploads;
        let svg_jobs = &mut self.svg_jobs;
        let svgs_drawing = &mut self.svgs_drawing;
        self.images_waiting.retain(|(image, file, want)| {
            let notice = match files.get(file) {
                Some(_) if is_svg(file) => {
                    svg_jobs.retain(|job| job.image != *image);
                    svg_jobs.push(SvgJob {
                        image: *image,
                        file: file.clone(),
                        want: *want,
                    });
                    svgs_drawing.insert(*image);
                    return false;
                }
                Some(bytes) => match decode(bytes) {
                    Some((size, rgba)) => {
                        uploads.push(Upload::Image {
                            key: *image,
                            size,
                            rgba,
                        });
                        ImageNotice::Ready {
                            width: size.width,
                            height: size.height,
                        }
                    }
                    None => ImageNotice::Failed,
                },
                None if complete => ImageNotice::Missing,
                None => return true,
            };
            events.push(Event::Image {
                image: *image,
                notice,
            });
            false
        });
        self.notes_waiting.retain(|file| {
            let notice = match files.get(file) {
                Some(bytes) => NoteNotice::Text(String::from_utf8_lossy(bytes).into_owned()),
                None if complete => NoteNotice::Missing,
                None => return true,
            };
            events.push(Event::Note {
                file: file.clone(),
                notice,
            });
            false
        });
        for event in events {
            self.send(event);
        }
    }

    /// Hands the renderer `page`'s snapshot as its frame and tells the app
    /// the page has loaded.
    fn show_snapshot(&mut self, page: &EntityId, host: Host) {
        let Some((size, rgba)) = self.snapshots.get(page).and_then(|png| decode(png)) else {
            return;
        };
        self.uploads.push(Upload::PageFrame {
            page: host.page,
            viewport: host.viewport,
            frame: CpuFrame {
                size,
                stride: size.width * 4,
                bgra: premultiplied_bgra(&rgba),
                dirty: Vec::new(),
            },
        });
        self.send(Event::Page {
            page: page.clone(),
            notice: PageNotice::Loaded { http_status: 200 },
        });
    }

    /// Which renderer page shows each page entity.
    pub fn page_of(&self, entity: &EntityId) -> Option<PageId> {
        self.hosts.get(entity).map(|host| host.page)
    }
}

/// `document` with exactly `comments` as its annotations.
fn with_comments(mut document: Document, comments: Vec<Annotation>) -> Document {
    let ids: Vec<AnnotationId> = (document.annotations().iter())
        .map(|each| each.id.clone())
        .collect();
    for id in ids {
        let _ = document.apply(Command::RemoveAnnotation(id));
    }
    for (at, annotation) in comments.into_iter().enumerate() {
        let _ = document.apply(Command::InsertAnnotation {
            annotation: Box::new(annotation),
            at,
        });
    }
    document
}

/// Whether the file at `path` is an svg.
fn is_svg(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("svg"))
}

/// An image file's first frame as straight-alpha RGBA.
fn decode(bytes: &[u8]) -> Option<(PixelSize, Vec<u8>)> {
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = PixelSize::new(decoded.width(), decoded.height());
    Some((size, decoded.into_raw()))
}

/// Straight RGBA as the premultiplied BGRA a page frame is.
fn premultiplied_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for texel in rgba.as_chunks::<4>().0 {
        let alpha = u16::from(texel[3]);
        let times = |channel: u8| ((u16::from(channel) * alpha + 127) / 255) as u8;
        out.extend_from_slice(&[times(texel[2]), times(texel[1]), times(texel[0]), texel[3]]);
    }
    out
}
