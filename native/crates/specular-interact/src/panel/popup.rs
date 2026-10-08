//! [`popup_for`]: the popup of the tool in hand, or of the selection.
//!
//! The tool's popup wins over the selection's (ADR 0008 §2). A selection of
//! one kind gets that kind's popup, acting on all of it (§4).

mod actions;
mod drawing;
mod edge;
mod file;
mod group;
mod page;
mod shape;
mod text;
mod tool;

use specular_doc::{Edge, Entity, ItemId, Kind, Rect};

use super::{Align, Placement, PopupAnchor, PopupModel};
use crate::{App, Gesture, TITLE_GAP, TITLE_LINE, Tool};

/// The space between a selection and its popup, in screen pixels.
const SELECTION_GAP: f32 = 14.0;

/// The space between a title and a popup over it.
const TITLE_CLEARANCE: f32 = 8.0;

/// What the popup is about. Computed before anything is built, so each
/// popup is chosen by one exhaustive match.
enum Subject<'a> {
    /// There is nothing to point at.
    Nothing,
    /// A drag or a draw is in flight, and a popup would be in its way.
    Busy,
    /// A tool that has a popup of its own is in hand.
    Tool(Tool),
    /// One edge is the whole selection.
    Edge(&'a Edge),
    /// Entities of one kind are the whole selection.
    Entities(Family, Vec<&'a Entity>),
    /// The selection spans kinds.
    Mixed(Vec<&'a Entity>),
}

/// The kinds of entity, one popup each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Text,
    Shape,
    Drawing,
    Group,
    File,
    Page,
}

fn family(kind: &Kind) -> Family {
    match kind {
        Kind::Text(_) => Family::Text,
        Kind::Shape(_) => Family::Shape,
        Kind::Drawing(_) => Family::Drawing,
        Kind::Group(_) => Family::Group,
        Kind::File(_) => Family::File,
        Kind::Page(_) => Family::Page,
    }
}

/// Whether `tool` has a popup of its own, which then takes the place of any
/// selection's.
const fn has_popup(tool: Tool) -> bool {
    match tool {
        Tool::AddPage | Tool::AddText | Tool::AddSticky | Tool::AddShape | Tool::Draw => true,
        Tool::Select | Tool::AddDocument | Tool::Comment => false,
    }
}

fn subject(app: &App) -> Subject<'_> {
    let session = &app.session;
    // Selecting text inside an edit is part of the edit, not a drag.
    let busy = match &session.gesture {
        Some(Gesture::TextSelect(_)) | None => false,
        Some(
            Gesture::Move(_)
            | Gesture::Resize(_)
            | Gesture::Marquee { .. }
            | Gesture::Comment(_)
            | Gesture::Place(_)
            | Gesture::Draw(_)
            | Gesture::EdgeDrag(_)
            | Gesture::Line(_),
        ) => true,
    };
    // A tool's popup stays while its tool is drawing.
    if has_popup(session.tool) {
        return Subject::Tool(session.tool);
    }
    if busy {
        return Subject::Busy;
    }
    if session
        .editing
        .as_ref()
        .is_some_and(|edit| !edit.keeps_popup())
    {
        return Subject::Nothing;
    }
    match session.selection.items() {
        [] => Subject::Nothing,
        [ItemId::Edge(id)] => app
            .document
            .edge(id)
            .map_or(Subject::Nothing, Subject::Edge),
        items => {
            let entities: Option<Vec<&Entity>> = items
                .iter()
                .map(|item| match item {
                    ItemId::Entity(id) => app.document.entity(id),
                    ItemId::Edge(_) => None,
                })
                .collect();
            let Some(entities) = entities else {
                let entities = (items.iter())
                    .filter_map(|item| match item {
                        ItemId::Entity(id) => app.document.entity(id),
                        ItemId::Edge(_) => None,
                    })
                    .collect();
                return Subject::Mixed(entities);
            };
            let Some(first) = entities.first().map(|entity| family(&entity.kind)) else {
                return Subject::Nothing;
            };
            if entities.iter().all(|entity| family(&entity.kind) == first) {
                Subject::Entities(first, entities)
            } else {
                Subject::Mixed(entities)
            }
        }
    }
}

/// The popup to show for `app` as it is now, or `None` when there is none:
/// nothing selected, a drag in flight, or a subject with no control to offer.
pub fn popup_for(app: &App) -> Option<PopupModel> {
    match subject(app) {
        Subject::Tool(tool) => tool::popup(app, tool),
        Subject::Edge(edge) => edge::popup(app, edge),
        Subject::Entities(family, entities) => match family {
            Family::Text => Some(text::popup(app, &entities)),
            Family::Shape => Some(shape::popup(app, &entities)),
            Family::Drawing => Some(drawing::popup(app, &entities)),
            Family::Group => Some(group::popup(app, &entities)),
            Family::File => Some(file::popup(app, &entities)),
            Family::Page => Some(page::popup(app, &entities)),
        },
        Subject::Mixed(entities) => mixed(&entities),
        Subject::Nothing | Subject::Busy => None,
    }
}

/// The popup of a selection that spans kinds (`MultiSelectPopup.tsx`): what
/// every kind shares, arranging, annotating and focusing. One item alone has
/// nothing to arrange with, and an edge with it nothing to annotate.
fn mixed(entities: &[&Entity]) -> Option<PopupModel> {
    if entities.len() < 2 {
        return None;
    }
    let noun = format!("{} items", entities.len());
    Some(PopupModel {
        anchor: over(entities, Align::Center),
        controls: actions::Actions::all(&noun, entities.len()).controls(),
    })
}

/// An anchor over the union of `entities`.
fn over(entities: &[&Entity], align: Align) -> PopupAnchor {
    PopupAnchor::Canvas {
        bounds: union(entities),
        placement: Placement::Above,
        align,
        gap: SELECTION_GAP,
    }
}

/// An anchor over `entities` that clears the title line above them: the
/// line, the gap under it and [`TITLE_CLEARANCE`] more, so the popup never
/// sits on a page's address or a group's name.
fn over_titled(entities: &[&Entity], align: Align) -> PopupAnchor {
    match over(entities, align) {
        PopupAnchor::Canvas {
            bounds,
            placement,
            align,
            ..
        } => PopupAnchor::Canvas {
            bounds,
            placement,
            align,
            gap: TITLE_CLEARANCE + TITLE_LINE + TITLE_GAP,
        },
        anchor @ (PopupAnchor::Toolbar { .. } | PopupAnchor::Point(_)) => anchor,
    }
}

fn union(entities: &[&Entity]) -> Rect {
    (entities.iter())
        .map(|entity| entity.rect)
        .reduce(crate::geometry::union)
        .unwrap_or_default()
}
