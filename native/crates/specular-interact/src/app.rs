//! [`App`]: the document, its history, and the [`Session`].

use glam::Vec2;
use specular_core::Camera;
use specular_doc::{Document, Entity, EntityId, History, ItemId, Kind, Page};

use crate::page_input::ButtonCapture;
use crate::{Gesture, PagePlacement, Tool};

/// Everything the app knows. Only [`update`](crate::update) changes it.
#[derive(Debug, Clone, Default)]
pub struct App {
    pub(crate) document: Document,
    pub(crate) history: History,
    pub(crate) session: Session,
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
    /// A page's viewport is its rect's size. While its handle is being
    /// dragged the viewport stays at the size the drag started from, so the
    /// page stretches until the release re-lays it out.
    pub fn page_placement(&self, id: &EntityId) -> Option<PagePlacement> {
        let entity = self.document.entity(id)?;
        page_of(entity)?;
        let laid_out_at = match &self.session.gesture {
            Some(Gesture::Resize { entity, start, .. }) if entity == id => *start,
            Some(
                Gesture::Resize { .. }
                | Gesture::Move { .. }
                | Gesture::Marquee { .. }
                | Gesture::CommentRegion { .. },
            )
            | None => entity.rect,
        };
        Some(PagePlacement {
            rect: entity.rect,
            viewport: PagePlacement::viewport_for(laid_out_at),
        })
    }

    /// The pages, back-to-front, each with its placement.
    pub fn pages(&self) -> impl Iterator<Item = (&EntityId, &Page, PagePlacement)> {
        self.document.entities().filter_map(|entity| {
            let page = page_of(entity)?;
            Some((&entity.id, page, self.page_placement(&entity.id)?))
        })
    }

    /// An annotation id nothing in the document uses.
    pub(crate) fn fresh_annotation_id(&mut self) -> specular_doc::AnnotationId {
        loop {
            let id = specular_doc::AnnotationId::new(self.session.next_id());
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
    /// Where the pointer is, in logical screen pixels. `None` when it is
    /// outside the window.
    pub pointer: Option<Vec2>,
    /// The wall clock at the latest tick, in milliseconds since the Unix
    /// epoch.
    pub now_ms: u64,
    /// The page the pointer's moves are going to, which is owed a leave.
    pub(crate) pointer_page: Option<EntityId>,
    /// Which page got each held button's press.
    pub(crate) captured: ButtonCapture,
    /// State of the id sequence.
    id_state: u64,
}

impl Session {
    /// The region the comment tool is dragging out, in canvas space.
    pub fn comment_preview(&self) -> Option<specular_doc::Rect> {
        match &self.gesture {
            Some(Gesture::CommentRegion { start, current, .. }) => {
                Some(crate::geometry::spanning(*start, *current))
            }
            Some(Gesture::Move { .. } | Gesture::Resize { .. } | Gesture::Marquee { .. })
            | None => None,
        }
    }

    /// The next id in the sequence: 16 hex digits from a splitmix64 step.
    fn next_id(&mut self) -> String {
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
