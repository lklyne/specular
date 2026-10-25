//! [`App`]: the document, its history, and the [`Session`].

mod session;

pub use self::session::{Focus, Selection, Session};

use std::borrow::Cow;

use specular_agent::{Repos, Threads};
use specular_doc::{AnnotationId, Document, Entity, EntityId, History, Kind, Page};

use crate::edit::Measurer;
use crate::space::Space;
use crate::{Appearance, Gesture, PagePlacement, Theme, ThemeState, ToolDefaults};

/// Everything the app knows. Only [`update`](crate::update) changes it.
#[derive(Debug, Clone, Default)]
pub struct App {
    pub(crate) document: Document,
    /// Each step carries the selection from either side of it, so undo and
    /// redo put back what was selected along with what changed.
    pub(crate) history: History<Selection>,
    pub(crate) session: Session,
    /// The folder of canvases. The fields above are its active canvas.
    pub(crate) space: Space,
    /// The folder a document opened on its own sits in. It is no space:
    /// only the document's relative file paths start from it.
    pub(crate) document_folder: Option<String>,
    /// Every thread of the space. Space-wide like `space`: a thread belongs
    /// to a canvas by its tab id, and switching canvas leaves them alone.
    pub(crate) threads: Threads,
    pub(crate) repos: Repos,
    /// The models the agent can run and the one chosen. App-wide: every
    /// thread's next run uses it.
    pub(crate) models: specular_agent::Models,
    pub(crate) tool_defaults: ToolDefaults,
    /// What is kept between launches besides the tool defaults.
    pub(crate) settings: crate::AppSettings,
    /// What the app is built from, as the shell told it.
    pub(crate) about: Vec<crate::AboutRow>,
    /// Why no space is open, while the user has yet to choose one.
    pub(crate) space_ask: Option<crate::SpaceAsk>,
    /// The appearance choice and the system's appearance.
    pub(crate) theme: ThemeState,
    /// Lays text out for the editor.
    pub(crate) measure: Measurer,
}

impl App {
    /// An app with an empty document. `id_seed` starts the sequence that new
    /// ids are drawn from; pass something that differs between launches.
    pub fn new(id_seed: u64) -> Self {
        Self {
            session: Session {
                id_state: id_seed,
                ..Session::default()
            },
            ..Self::default()
        }
    }

    /// The persisted, undoable state.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The state that is not saved: camera, selection, tool, gesture.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// What each creation tool stamps on the next entity it makes. App
    /// settings: not in the document and not in undo.
    pub fn tool_defaults(&self) -> &ToolDefaults {
        &self.tool_defaults
    }

    /// The appearance the user chose: light, dark, or the system's. An app
    /// setting: not in the document and not in undo.
    pub fn theme(&self) -> Theme {
        self.theme.choice()
    }

    /// Light or dark: what the choice and the system's appearance come to,
    /// which is what is drawn.
    pub fn appearance(&self) -> Appearance {
        self.theme.appearance()
    }

    /// The entity the gesture in flight is creating: a shape being dragged
    /// out or a stroke being drawn. It is in the document already, in front
    /// of everything, and is drawn like any other entity. Releasing makes it
    /// an undo step and cancelling takes it back.
    pub(crate) fn creating(&self) -> Option<&EntityId> {
        match &self.session.gesture {
            Some(Gesture::Place(drag)) => drag.live(),
            Some(Gesture::Draw(stroke)) => Some(stroke.drawing()),
            Some(
                Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::Marquee { .. }
                | Gesture::Comment(_)
                | Gesture::TextSelect(_)
                | Gesture::EdgeDrag(_)
                | Gesture::Line(_),
            )
            | None => None,
        }
    }

    /// The group being worked inside, if one was stepped into and the
    /// selection is still within it.
    pub fn entered_group(&self) -> Option<&EntityId> {
        self.session.entered_group.as_ref()
    }

    /// Whether there is a step to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether there is a step to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Where the page `id` sits and the viewport it is laid out at, or `None`
    /// when `id` is not a page.
    ///
    /// A page's viewport is its rect's size, as its handle is dragged too.
    /// A page that fills its tab sits at the rect the tab presents it at,
    /// laid out at that size. A page in a device frame sits inside the safe
    /// area of the screen its rect is, unless its document covers it.
    pub fn page_placement(&self, id: &EntityId) -> Option<PagePlacement> {
        let entity = self.document.entity(id)?;
        if !is_page(entity) {
            return None;
        }
        if let Some(rect) = crate::showing::presented_rect(self, entity) {
            return Some(PagePlacement {
                rect,
                viewport: PagePlacement::viewport_for(rect),
            });
        }
        let rect = crate::safe_area::within(entity.rect, self.page_inset(id));
        Some(PagePlacement {
            rect,
            viewport: PagePlacement::viewport_for(rect),
        })
    }

    /// What the hosted page `id` last reported of itself: its live title,
    /// address, load, history and scroll. `None` until it reports anything.
    pub fn page_state(&self, id: &EntityId) -> Option<&crate::PageState> {
        self.session.pages.get(id)
    }

    /// How far the page `id` is scrolled, in its CSS pixels. Zero for a page
    /// that has not said.
    pub(crate) fn page_scroll(&self, id: &EntityId) -> glam::DVec2 {
        self.page_state(id)
            .map_or(glam::DVec2::ZERO, |state| state.scroll)
    }

    /// What is known about the image file a file entity names by `file`, or
    /// `None` when it is not an image or has not been asked for.
    pub fn image(&self, file: &str) -> Option<&crate::Image> {
        self.session.images.get(file)
    }

    /// What is known about the markdown file a file entity names by `file`,
    /// or `None` when it is not a Document or has not been asked for.
    pub fn note(&self, file: &str) -> Option<&crate::NoteState> {
        self.session.notes.get(file)
    }

    /// How far the Document `entity` is scrolled down, in canvas units.
    pub fn note_scroll(&self, entity: &EntityId) -> f32 {
        self.session.notes.scroll(entity)
    }

    /// The page `id` is, or `None` when it is not one. An HTML file's URL
    /// is its file's, resolved against [`files_folder`](Self::files_folder).
    pub fn page(&self, id: &EntityId) -> Option<Cow<'_, Page>> {
        page_of(self, self.document.entity(id)?)
    }

    /// Where the page `id` is: the address it last reported, else the one
    /// it has for a page that has reported nothing.
    pub fn page_address(&self, id: &EntityId) -> Option<String> {
        (self.page_state(id).and_then(|state| state.url.clone()))
            .or_else(|| Some(self.page(id)?.url.clone()))
            .filter(|url| !url.is_empty())
    }

    /// The page `id`'s address as an anchor records it, so a `.canvas`
    /// names a file of its own folder by its relative path.
    pub fn page_anchor_url(&self, id: &EntityId) -> Option<String> {
        crate::anchor::anchor_page_url(&self.page_address(id)?, self.files_folder())
    }

    /// The folder relative file paths start from: the space folder, else
    /// the folder of a document opened on its own.
    pub(crate) fn files_folder(&self) -> Option<&str> {
        (self.space.folder()).or(self.document_folder.as_deref())
    }

    /// The pages, back-to-front, each with its placement.
    pub fn pages(&self) -> impl Iterator<Item = (&EntityId, PagePlacement)> {
        (self.document.entities())
            .filter_map(|entity| Some((&entity.id, self.page_placement(&entity.id)?)))
    }

    /// An id no entity or edge in the document uses.
    pub(crate) fn fresh_id(&mut self) -> String {
        loop {
            let id = self.session.next_id();
            let taken = self.document.entity(&EntityId::from(id.as_str())).is_some()
                || (self.document.edge(&specular_doc::EdgeId::from(id.as_str()))).is_some();
            if !taken {
                return id;
            }
        }
    }

    /// An annotation id nothing in the document uses, entities and edges
    /// included: a draft's edit is keyed by it beside theirs.
    pub(crate) fn fresh_annotation_id(&mut self) -> AnnotationId {
        loop {
            let id = AnnotationId::new(self.fresh_id());
            if self.document.annotation(&id).is_none() {
                return id;
            }
        }
    }
}

/// The page `entity` is, or `None` for every other kind: a link node, or
/// an HTML file node with its path resolved against the files folder as
/// its URL.
pub(crate) fn page_of<'a>(app: &App, entity: &'a Entity) -> Option<Cow<'a, Page>> {
    let mut page = entity.kind.page_fields()?;
    if let Kind::File(file) = &entity.kind {
        page.to_mut().url = crate::space_file::file_page_url(&file.file, app.files_folder());
    }
    Some(page)
}

/// Whether `entity` is a page: a link node or an HTML file node.
pub(crate) fn is_page(entity: &Entity) -> bool {
    match &entity.kind {
        Kind::Page(_) => true,
        Kind::File(file) => file.is_html(),
        Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => false,
    }
}
