//! Placing an item by name rather than by a click: what `item.add` runs, so
//! a page, a script or an agent can put a sticky, a text or a shape at a
//! canvas point.

use glam::DVec2;
use specular_doc::{EntityId, ItemId, Kind, TextStyle};

use crate::place::{self, Placing};
use crate::{App, Effect, edit, live};

/// What [`Action::AddItem`](crate::Action::AddItem) places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewKind {
    /// A sticky note.
    Sticky,
    /// Plain text.
    Text,
    /// A shape, in the shape tool's style.
    Shape,
}

/// An item to place: its kind, its top-left on the canvas, and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct NewItem {
    /// What it is.
    pub kind: NewKind,
    /// Its top-left corner, in canvas units.
    pub at: DVec2,
    /// What it says. A shape's is its label.
    pub text: String,
}

/// Places `item` in the tool defaults' style, in front of everything, as
/// one undo step, and selects it.
pub(crate) fn add(app: &mut App, item: NewItem, effects: &mut Vec<Effect>) {
    let id = EntityId::from(app.fresh_id().as_str());
    let placing = match item.kind {
        NewKind::Sticky => Placing::Text(TextStyle::Sticky),
        NewKind::Text => Placing::Text(TextStyle::Plain),
        NewKind::Shape => Placing::Shape,
    };
    let mut entity = place::default_entity(app, placing, id.clone(), item.at);
    if let Kind::Text(text) = &mut entity.kind {
        text.text = item.text;
        entity.rect = edit::fitted(app, entity.rect, text);
    } else if let Kind::Shape(shape) = &mut entity.kind {
        shape.text = item.text;
    }
    live::create(app, entity, effects);
    app.session.selection.set([ItemId::Entity(id)]);
}
