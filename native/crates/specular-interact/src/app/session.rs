//! The state that is not saved with the document: the session, focus and selection.

use glam::Vec2;
use specular_core::{Camera, Modifiers};
use specular_doc::{Annotation, AnnotationId, EntityId, ItemId};

use crate::edit::TextEdit;
use crate::images::Images;
use crate::notes::Notes;
use crate::page_input::ButtonCapture;
use crate::panel::PanelUi;
use crate::saved::LoadedFits;
use crate::{Cursor, Gesture, Tool};

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
    /// The camera transition in flight, and whether the shell runs them.
    pub(crate) motion: crate::showing::motion::Motion,
    /// The pages turning on to their side or back up.
    pub(crate) turns: crate::showing::turn::Turns,
    /// The agent cursors, and the acts waiting for one to arrive.
    pub(crate) presence: crate::presence::Presence,
    /// The item tabs of the active canvas: their order and the lens each
    /// keeps.
    pub(crate) tabs: crate::showing::Tabs,
    /// Whether the eye is open: Device draws what is around its item. One
    /// choice for every tab and canvas, shut to begin with.
    pub(crate) others_shown: bool,
    /// The group stepped into by a double click. It stays while the
    /// selection is inside it, and Escape steps out of it.
    pub(crate) entered_group: Option<EntityId>,
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
    /// The page elements inside the comment region being dragged.
    pub(crate) region_preview: crate::comment::RegionPreview,
    /// The cursor the shell was last asked to show.
    pub cursor: Cursor,
    /// The chrome: whether the shell draws it, the open dropdown and menu,
    /// and the sidebar row picked last.
    pub panel: PanelUi,
    /// Whether the sidebar is shown, and its folds. See
    /// [`App::covered_left`].
    pub sidebar: crate::SidebarView,
    /// The right panel: whether the shell has one, whether it is open, and
    /// its width.
    pub(crate) chat: crate::ChatView,
    /// Where the pointer is, in logical screen pixels. `None` when it is
    /// outside the window.
    pub(crate) pointer: Option<Vec2>,
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
    /// The swipe being made on a touch page and the glide it left.
    pub(crate) flick: crate::flick::Flick,
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
    pub(crate) fn is_empty(&self) -> bool {
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
