//! [`dock`]: the controls of the tool in hand, or of the selection.
//!
//! The tool's options win over the selection's (ADR 0008 §2). A selection of
//! one kind gets that kind's controls, acting on all of it (§4).

mod actions;
mod drawing;
mod edge;
mod file;
mod group;
mod page;
mod shape;
mod text;
mod tool;

use specular_doc::{Entity, ItemId, Kind};

use super::ControlsModel;
use crate::{App, Tool};

/// What the dock is about. Computed before anything is built, so its
/// controls are chosen by one exhaustive match.
enum Subject<'a> {
    /// There is nothing to offer controls for.
    Nothing,
    /// A tool that has options of its own is in hand.
    Tool(Tool),
    /// One edge is the whole selection.
    Edge,
    /// Entities of one kind are the whole selection.
    Entities(Family, Vec<&'a Entity>),
    /// The selection spans kinds.
    Mixed(Vec<&'a Entity>),
}

/// The kinds of entity, one set of controls each.
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

/// Whether `tool` has options of its own, which then take the place of any
/// selection's controls.
const fn has_options(tool: Tool) -> bool {
    match tool {
        Tool::AddPage | Tool::AddText | Tool::AddSticky | Tool::AddShape | Tool::Draw => true,
        Tool::Select | Tool::AddDocument | Tool::Comment | Tool::Inspect => false,
    }
}

fn subject(app: &App) -> Subject<'_> {
    let session = &app.session;
    if has_options(session.tool) {
        return Subject::Tool(session.tool);
    }
    if session
        .editing
        .as_ref()
        .is_some_and(|edit| !edit.keeps_dock())
    {
        return Subject::Nothing;
    }
    match session.selection.items() {
        [] => Subject::Nothing,
        [ItemId::Edge(id)] => app
            .document
            .edge(id)
            .map_or(Subject::Nothing, |_| Subject::Edge),
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

/// What the dock shows for `app` as it is now, or `None` when its bar is
/// empty: nothing selected, or a subject with no control to offer. A drag
/// changes none of it, so the bar holds still through one.
pub fn dock(app: &App) -> Option<ControlsModel> {
    match subject(app) {
        Subject::Tool(tool) => tool::controls(app, tool),
        Subject::Edge => Some(edge::controls(app)),
        Subject::Entities(family, entities) => Some(match family {
            Family::Text => text::controls(app, &entities),
            Family::Shape => shape::controls(app, &entities),
            Family::Drawing => drawing::controls(app, &entities),
            Family::Group => group::controls(app, &entities),
            Family::File => file::controls(app, &entities),
            Family::Page => page::controls(app, &entities),
        }),
        Subject::Mixed(entities) => mixed(&entities),
        Subject::Nothing => None,
    }
}

/// The controls of a selection that spans kinds (`MultiSelectPopup.tsx`):
/// what every kind shares, arranging, annotating and focusing. One item
/// alone has nothing to arrange with, and an edge with it nothing to
/// annotate.
fn mixed(entities: &[&Entity]) -> Option<ControlsModel> {
    if entities.len() < 2 {
        return None;
    }
    let noun = format!("{} items", entities.len());
    Some(ControlsModel {
        controls: actions::Actions::all(&noun, entities.len()).controls(),
    })
}
