//! Placing an item by name rather than by a click: what `item.add` runs, so
//! a page, a script or an agent can put a sticky, a text or a shape at a
//! canvas point.

use glam::DVec2;
use specular_doc::{EntityId, ItemId, Kind, TextStyle};

use crate::mods::{self, NewModItem};
use crate::place::{self, Placing};
use crate::{App, Effect, edit, live};

/// What [`Action::AddItem`](crate::Action::AddItem) places.
#[derive(Debug, Clone, PartialEq)]
pub enum NewKind {
    /// A sticky note.
    Sticky,
    /// Plain text.
    Text,
    /// A shape, in the shape tool's style.
    Shape,
    /// A mod's shape by its `<mod>/<shape>` name, in the shape tool's
    /// style (ADR 0060).
    CustomShape(String),
    /// An item of an installed mod (ADR 0057).
    Mod(Box<NewModItem>),
}

/// An item to place: its kind, its top-left on the canvas, and its text.
#[derive(Debug, Clone, PartialEq)]
pub struct NewItem {
    /// What it is.
    pub kind: NewKind,
    /// Its top-left corner, in canvas units, or `None` to centre it in the
    /// view.
    pub at: Option<DVec2>,
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
        NewKind::Shape | NewKind::CustomShape(_) => Placing::Shape,
        NewKind::Mod(new) => return mods::add_item(app, *new, item.at, effects),
    };
    let custom = match &item.kind {
        NewKind::CustomShape(name) => match app.mod_catalog().shape_named(name) {
            Some(custom) => Some(custom),
            None => return,
        },
        NewKind::Sticky | NewKind::Text | NewKind::Shape | NewKind::Mod(_) => None,
    };
    let at = item.at.unwrap_or_else(|| view_middle(app));
    let mut entity = place::default_entity(app, placing, id.clone(), at);
    if let Kind::Text(text) = &mut entity.kind {
        text.text = item.text;
        entity.rect = edit::fitted(app, entity.rect, text);
    } else if let Kind::Shape(shape) = &mut entity.kind {
        shape.text = item.text;
        if let Some(custom) = custom {
            shape.shape = specular_doc::ShapeKind::Custom;
            shape.custom = Some(custom);
        }
    }
    live::create(app, entity, effects);
    app.session.selection.set([ItemId::Entity(id)]);
}

/// The canvas point in the middle of the view.
pub(crate) fn view_middle(app: &App) -> DVec2 {
    let middle = app.session.viewport / 2.0;
    app.session.camera.screen_to_world(middle).as_dvec2()
}
