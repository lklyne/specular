//! [`ViewCache`]: what [`view`](crate::view) keeps from one frame to the
//! next, so a frame that shows what the last one did builds it again from
//! parts already made.
//!
//! Whoever draws frames owns one and passes it to every `view`. It changes
//! no scene: an entry is used only while what it was built from is equal to
//! what the app holds now. What a frame does not draw is let go of on the
//! next, so the cache is never larger than two frames' worth.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use specular_doc::{EntityId, Stroke};
use specular_interact::StackCache;

use crate::{Item, Row};

/// The parts of earlier frames a [`view`](crate::view) can use again.
#[derive(Debug, Default)]
pub struct ViewCache {
    /// The layout of the Document being edited.
    pub(crate) stacks: StackCache,
    /// The rows of each Document that is read, not edited.
    pub(crate) notes: Kept<NoteRows>,
    /// The outline of each stroke of each drawing.
    pub(crate) strokes: Kept<Vec<StrokeOutline>>,
    built: Cell<usize>,
}

/// A Document's markdown as rows, with what they were made from.
#[derive(Debug)]
pub(crate) struct NoteRows {
    pub(crate) text: Arc<str>,
    pub(crate) width: f32,
    pub(crate) rows: Vec<Row>,
}

/// One stroke's outline as the item drawn for it, with what it was made
/// from. A stroke too short to have an outline draws nothing.
#[derive(Debug)]
pub(crate) struct StrokeOutline {
    pub(crate) stroke: Stroke,
    /// The width the outline was built at.
    pub(crate) size: f64,
    pub(crate) item: Option<Item>,
}

impl ViewCache {
    /// How many Documents were parsed and strokes outlined by the latest
    /// `view`, because nothing kept matched. A performance pin reads it.
    pub fn built(&self) -> usize {
        self.built.get()
    }

    /// Starts a frame: what the last one did not use is dropped.
    pub(crate) fn begin(&self) {
        self.built.set(0);
        self.notes.begin();
        self.strokes.begin();
    }

    pub(crate) fn count_built(&self) {
        self.built.set(self.built.get() + 1);
    }
}

/// Values by entity, from this frame and the one before.
#[derive(Debug)]
pub(crate) struct Kept<V>(RefCell<Frames<V>>);

#[derive(Debug)]
struct Frames<V> {
    last: HashMap<EntityId, V>,
    now: HashMap<EntityId, V>,
}

impl<V> Default for Kept<V> {
    fn default() -> Self {
        Self(RefCell::new(Frames {
            last: HashMap::new(),
            now: HashMap::new(),
        }))
    }
}

impl<V> Kept<V> {
    fn begin(&self) {
        let mut frames = self.0.borrow_mut();
        frames.last = std::mem::take(&mut frames.now);
    }

    /// Takes what is kept for `id`, to be put back once it is brought up to
    /// date.
    pub(crate) fn take(&self, id: &EntityId) -> Option<V> {
        let mut frames = self.0.borrow_mut();
        let now = frames.now.remove(id);
        now.or_else(|| frames.last.remove(id))
    }

    pub(crate) fn put(&self, id: &EntityId, value: V) {
        self.0.borrow_mut().now.insert(id.clone(), value);
    }
}
