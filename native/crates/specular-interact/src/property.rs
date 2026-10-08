//! [`Property`]: one field of the selected items that a popup control sets.
//!
//! A property applies to every selected item it means something for and
//! skips the rest, so one pick on a mixed selection changes what it can. The
//! whole change is one undo step, and nothing is recorded when it would
//! change nothing. Tool defaults are not touched: only the tool popup writes
//! those (ADR 0008).
//!
//! [`read`] has the matching reads: the value the whole selection shares.

mod apply;
mod page;
pub mod read;

use specular_doc::{
    BorderStyle, BrushType, Color, ColorScheme, Command, EdgeEnd, Entity, EntityId, FillStyle,
    ItemId, LineStyle, ShapeKind, TextAlign, TextFont, TextStyle, VerticalAlign,
};

use crate::{App, Effect, edit, update};

/// A page's orientation, which turns its preset size across or upright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Taller than it is wide, for a phone-shaped preset.
    Portrait,
    /// Wider than it is tall.
    Landscape,
}

/// A field of the selection to set. Each names the kinds it applies to.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Text ink or sticky fill, shape fill, group color, every stroke of a
    /// drawing, or edge color. On a shape it also turns a transparent fill
    /// back to solid.
    Color(Color),
    /// A shape's border color.
    BorderColor(Color),
    /// A text's size, or a shape's label size, in pixels.
    TextSize(f64),
    /// A text's typeface.
    TextFont(TextFont),
    /// Whether a text is a sticky card or plain text. The text takes what
    /// the creation tool of the new style stamps where the two differ: the
    /// width mode and the color from the tool defaults.
    TextStyle(TextStyle),
    /// A shape label's horizontal alignment.
    TextAlign(TextAlign),
    /// A shape label's vertical alignment.
    TextVerticalAlign(VerticalAlign),
    /// A shape's silhouette.
    ShapeKind(ShapeKind),
    /// Whether a shape's body is painted. The color is kept.
    FillStyle(FillStyle),
    /// A shape's border style.
    BorderStyle(BorderStyle),
    /// A shape's border width, a drawing's stroke width or an edge's line
    /// width.
    StrokeWidth(f64),
    /// A drawing's brush. Each stroke's width moves to the nearest width the
    /// new brush is offered in.
    Brush(BrushType),
    /// An edge's line style.
    LineStyle(LineStyle),
    /// An edge's label. Empty takes the label off.
    Label(String),
    /// The endpoint shape at an edge's start.
    FromEnd(EdgeEnd),
    /// The endpoint shape at an edge's end.
    ToEnd(EdgeEnd),
    /// A page's viewport preset, by index into
    /// [`VIEWPORT_PRESETS`](specular_doc::VIEWPORT_PRESETS). Sets the page's
    /// size, device and preset.
    ViewportPreset(u32),
    /// A page keeps the size it has as a custom size, with no device.
    CustomViewport,
    /// A page's width as a custom size, in pixels. Its height stays.
    ViewportWidth(f64),
    /// A page's height as a custom size, in pixels. Its width stays.
    ViewportHeight(f64),
    /// A page's orientation.
    Orientation(Orientation),
    /// Whether a page is drawn in its device frame.
    DeviceFrame(bool),
    /// A page's color-scheme override. `None` follows the system.
    ColorScheme(Option<ColorScheme>),
}

impl Property {
    /// Whether the selection holds an item this property means something
    /// for, whether or not it would change.
    pub fn applies_to(&self, app: &App) -> bool {
        app.session.selection.items().iter().any(|item| match item {
            ItemId::Entity(id) => (app.document.entity(id)).is_some_and(|e| self.reaches(app, e)),
            ItemId::Edge(id) => {
                (app.document.edge(id)).is_some_and(|e| apply::edge(self, e).is_some())
            }
        })
    }

    fn reaches(&self, app: &App, entity: &Entity) -> bool {
        apply::entity(self, entity, &app.tool_defaults).is_some()
    }

    /// The commands that set this property on the selection, leaving out
    /// every item it would not change. The text being edited keeps its rect:
    /// the edit refits it.
    fn commands(&self, app: &App) -> Vec<Command> {
        let edited = edit::edited(app);
        let mut commands = Vec::new();
        for item in app.session.selection.items() {
            match item {
                ItemId::Entity(id) => {
                    let Some(entity) = app.document.entity(id) else {
                        continue;
                    };
                    let Some(mut next) = apply::entity(self, entity, &app.tool_defaults) else {
                        continue;
                    };
                    if edited != Some(id) {
                        apply::refit(app, entity, &mut next);
                    }
                    commands.extend(entity_changes(entity, &next));
                }
                ItemId::Edge(id) => {
                    let Some(edge) = app.document.edge(id) else {
                        continue;
                    };
                    if let Some(next) = apply::edge(self, edge)
                        && next != *edge
                    {
                        commands.push(Command::ReplaceEdge(Box::new(next)));
                    }
                }
            }
        }
        commands
    }
}

/// Makes `page` a page of custom size, keeping the size `rect` has.
pub(crate) fn make_custom(page: &mut specular_doc::Page, rect: &mut specular_doc::Rect) {
    page::set(&Property::CustomViewport, page, rect);
}

fn entity_changes(before: &Entity, after: &Entity) -> Vec<Command> {
    let mut commands = Vec::new();
    if after.rect != before.rect {
        commands.push(Command::SetRect {
            id: before.id.clone(),
            rect: after.rect,
        });
    }
    if after.kind != before.kind {
        commands.push(Command::SetKind {
            id: before.id.clone(),
            kind: Box::new(after.kind.clone()),
        });
    }
    commands
}

/// Whether `command` changes the entity `id`.
fn touches(command: &Command, id: &EntityId) -> bool {
    match command {
        Command::SetKind { id: target, .. } | Command::SetRect { id: target, .. } => target == id,
        Command::Batch(commands) => commands.iter().any(|command| touches(command, id)),
        Command::InsertEntity { .. }
        | Command::RemoveEntity(_)
        | Command::SetLabel { .. }
        | Command::SetParent { .. }
        | Command::SetAnchor { .. }
        | Command::InsertEdge { .. }
        | Command::RemoveEdge(_)
        | Command::ReplaceEdge(_)
        | Command::SetOrder(_)
        | Command::InsertAnnotation { .. }
        | Command::RemoveAnnotation(_)
        | Command::ReplaceAnnotation(_)
        | Command::SetNote { .. } => false,
    }
}

/// Sets `property` on the selection as one undo step, unless a drag is in
/// flight. A text edit carries on: the working text is not the document's
/// yet, so the property is its own step, and the edit's end keeps it. An
/// entity the edit placed for itself has no step to join, so it takes the
/// change with none, and the edit's end records it with the entity.
pub(crate) fn set(app: &mut App, property: &Property, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_some() {
        return;
    }
    let commands = property.commands(app);
    let (unrecorded, recorded): (Vec<Command>, Vec<Command>) = match edit::unrecorded(app) {
        Some(id) => commands
            .into_iter()
            .partition(|command| touches(command, id)),
        None => (Vec::new(), commands),
    };
    for command in unrecorded {
        if let Err(error) = app.document.apply(command) {
            tracing::warn!("property refused: {error}");
        }
    }
    if !recorded.is_empty() {
        update::document_step(app, Command::Batch(recorded), effects);
    }
    edit::refit_edited(app);
}
