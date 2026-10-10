//! [`App`]: the document, its history, and the [`Session`].

use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_doc::{
    Annotation, AnnotationId, Document, Entity, EntityId, History, ItemId, Kind, Page,
};

use specular_agent::{Repos, Threads};

use crate::edit::{Measurer, TextEdit};
use crate::images::Images;
use crate::notes::Notes;
use crate::page_input::ButtonCapture;
use crate::panel::builtin::PanelUi;
use crate::saved::LoadedFits;
use crate::space::Space;
use crate::{Appearance, Cursor, Gesture, PagePlacement, Theme, ThemeState, Tool, ToolDefaults};

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
    /// Every thread of the space. Space-wide like `space`: a thread belongs
    /// to a canvas by its tab id, and switching canvas leaves them alone.
    pub(crate) threads: Threads,
    pub(crate) repos: Repos,
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
    pub fn creating(&self) -> Option<&EntityId> {
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
    /// laid out at that size.
    pub fn page_placement(&self, id: &EntityId) -> Option<PagePlacement> {
        let entity = self.document.entity(id)?;
        page_of(entity)?;
        if let Some(rect) = crate::showing::presented_rect(self, entity) {
            return Some(PagePlacement {
                rect,
                viewport: PagePlacement::viewport_for(rect),
            });
        }
        Some(PagePlacement {
            rect: entity.rect,
            viewport: PagePlacement::viewport_for(entity.rect),
        })
    }

    /// What the hosted page `id` last reported of itself: its live title,
    /// address, load, history and scroll. `None` until it reports anything.
    pub fn page_state(&self, id: &EntityId) -> Option<&crate::PageState> {
        self.session.pages.get(id)
    }

    /// How far the page `id` is scrolled, in its CSS pixels. Zero for a page
    /// that has not said.
    pub fn page_scroll(&self, id: &EntityId) -> glam::DVec2 {
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

    /// The pages, back-to-front, each with its placement.
    pub fn pages(&self) -> impl Iterator<Item = (&EntityId, &Page, PagePlacement)> {
        self.document.entities().filter_map(|entity| {
            let page = page_of(entity)?;
            Some((&entity.id, page, self.page_placement(&entity.id)?))
        })
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

/// The page fields of `entity`, or `None` for every other kind.
pub(crate) fn page_of(entity: &Entity) -> Option<&Page> {
    match &entity.kind {
        Kind::Page(page) => Some(page),
        Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
    }
}

/// The state that is not saved with the document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Session {
    /// The canvas camera.
    pub camera: Camera,
    /// The canvas viewport in logical pixels.
    pub viewport: Vec2,
    /// What is selected.
    pub selection: Selection,
    /// The active tool.
    pub tool: Tool,
    /// The drag in flight, which owns the pointer until the button comes up.
    pub gesture: Option<Gesture>,
    /// The entity under the pointer: its body, or its title, handles or
    /// anchors.
    pub hover: Option<EntityId>,
    /// What keys go to.
    pub focus: Focus,
    /// The item a tab shows under the chrome, with the camera the canvas
    /// gets back. `None` while the canvas is shown.
    pub(crate) item_view: Option<crate::showing::ItemView>,
    /// The page whose tab is pressed and not yet clicked: its host is laid
    /// out for the tab ahead of the click. Nothing drawn follows it.
    pub(crate) prepared: Option<EntityId>,
    /// The item tabs of the active canvas: their order, and the lens and
    /// the Canvas-lens camera each keeps.
    pub(crate) tabs: crate::showing::Tabs,
    /// Whether the eye is shut: an item view draws its item and nothing
    /// else. One choice for every tab and canvas.
    pub(crate) others_hidden: bool,
    /// The group stepped into by a double click. It stays while the
    /// selection is inside it, and Escape steps out of it.
    pub entered_group: Option<EntityId>,
    /// The text, sticky or shape label being edited, with the edits so far.
    /// It stays in editing only while its entity is the whole selection, and
    /// Escape ends it. A comment draft's text is edited here too, whatever
    /// is selected.
    pub editing: Option<TextEdit>,
    /// The comment being written: an annotation with no text that is not in
    /// the document. Its text is the edit in `editing`.
    pub(crate) comment_draft: Option<Annotation>,
    /// The comment the keys act on.
    pub(crate) focused_comment: Option<AnnotationId>,
    /// The cursor the shell was last asked to show.
    pub cursor: Cursor,
    /// The built-in panels: whether they are on, the open dropdown, and the
    /// control under the pointer.
    pub panel: PanelUi,
    /// Whether the sidebar is shown, and its folds. See
    /// [`App::covered_left`].
    pub sidebar: crate::SidebarView,
    /// The right panel: whether the shell has one, whether it is open, and
    /// its width.
    pub chat: crate::ChatView,
    /// Where the pointer is, in logical screen pixels. `None` when it is
    /// outside the window.
    pub pointer: Option<Vec2>,
    /// The size on disk of each text that was measured when the document
    /// was opened.
    pub(crate) loaded_fits: LoadedFits,
    /// The modifier keys held at the latest pointer or key event.
    pub(crate) modifiers: Modifiers,
    /// The wall clock at the latest tick, in milliseconds since the Unix
    /// epoch.
    pub now_ms: u64,
    /// The page the pointer's moves are going to, which is owed a leave.
    pub(crate) inspect: crate::inspect::InspectState,
    pub(crate) pointer_page: Option<EntityId>,
    /// Which page got each held button's press.
    pub(crate) captured: ButtonCapture,
    /// What each hosted page last reported of itself.
    pub(crate) pages: crate::page_state::PageStates,
    /// What following a sync set remembers between events.
    pub(crate) sync: crate::sync::SyncState,
    /// The element questions put to pages and what each page tracks.
    pub(crate) attach: crate::attach::Attachments,
    /// The images file entities show, and how far each has loaded.
    pub(crate) images: Images,
    /// The text of the Documents file entities show, and their scroll.
    pub(crate) notes: Notes,
    /// State of the id sequence.
    pub(crate) id_state: u64,
}

impl Session {
    /// The next id in the sequence: 16 hex digits from a splitmix64 step.
    pub(crate) fn next_id(&mut self) -> String {
        self.id_state = self.id_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.id_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        format!("{:016x}", z ^ (z >> 31))
    }
}

/// What keys go to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Focus {
    /// The canvas: keys are bindings.
    #[default]
    Canvas,
    /// The entered page (ADR 0022): keys are forwarded to it, apart from
    /// Escape, and so is the pointer over its body. A page stays entered
    /// only while it is the whole selection.
    Page(EntityId),
}

impl Focus {
    /// The focused page, if a page has focus.
    pub fn page(&self) -> Option<&EntityId> {
        match self {
            Self::Canvas => None,
            Self::Page(page) => Some(page),
        }
    }
}

/// The selected entities and edges, in the order they were selected.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection(Vec<ItemId>);

impl Selection {
    /// The selected items.
    pub fn items(&self) -> &[ItemId] {
        &self.0
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `item` is selected.
    pub fn contains(&self, item: &ItemId) -> bool {
        self.0.contains(item)
    }

    /// The selected entities.
    pub fn entities(&self) -> impl Iterator<Item = &EntityId> {
        self.0.iter().filter_map(|item| match item {
            ItemId::Entity(id) => Some(id),
            ItemId::Edge(_) => None,
        })
    }

    /// The selected entity when exactly one item is selected and it is an
    /// entity.
    pub fn single_entity(&self) -> Option<&EntityId> {
        match self.0.as_slice() {
            [ItemId::Entity(id)] => Some(id),
            _ => None,
        }
    }

    /// Replaces the selection, dropping repeats.
    pub(crate) fn set(&mut self, items: impl IntoIterator<Item = ItemId>) {
        self.0.clear();
        for item in items {
            if !self.0.contains(&item) {
                self.0.push(item);
            }
        }
    }

    /// Selects `item` if it is not selected, and deselects it if it is.
    pub(crate) fn toggle(&mut self, item: ItemId) {
        match self.0.iter().position(|selected| *selected == item) {
            Some(index) => {
                self.0.remove(index);
            }
            None => self.0.push(item),
        }
    }

    /// Keeps only the items `keep` accepts.
    pub(crate) fn retain(&mut self, keep: impl FnMut(&ItemId) -> bool) {
        self.0.retain(keep);
    }
}
