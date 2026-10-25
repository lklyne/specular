//! What a click on a sidebar row selects, given the keys held
//! (`sidebarSelectionIntent`): a plain click only that row, a shift-click
//! the run of rows from the one picked last, and a command- or
//! control-click that row added to the selection or taken out of it.

use specular_core::Modifiers;
use specular_doc::ItemId;

use super::{RowKind, RowTarget, SidebarModel, SidebarRow};
use crate::panel::ControlId;
use crate::{Action, App};

/// The items a click can select, in the order the rows are shown. A group
/// is not one, and neither is a row inside a closed one.
fn selectable(model: &SidebarModel) -> Vec<ItemId> {
    fn walk(rows: &[SidebarRow], out: &mut Vec<ItemId>) {
        for row in rows {
            match (&row.target, row.kind) {
                (RowTarget::Entity(_), RowKind::Group { .. }) | (RowTarget::Comment(_), _) => {}
                (RowTarget::Entity(id), _) => out.push(ItemId::Entity(id.clone())),
            }
            if row.expanded == Some(true) {
                walk(&row.children, out);
            }
        }
    }
    let mut out = Vec::new();
    for (head, rows) in [
        (&model.notes_head, &model.notes),
        (&model.pages_head, &model.pages),
    ] {
        if !head.folded {
            walk(rows, &mut out);
        }
    }
    out
}

/// What a press on the row `pressed` of the sidebar sends, given the keys
/// held. Anything that is not a row of the list is sent as it is.
pub(crate) fn picked(
    app: &mut App,
    pressed: &ControlId,
    action: Action,
    keys: Modifiers,
) -> Action {
    let Action::Reveal { select, focus } = action else {
        return action;
    };
    if !pressed.as_str().starts_with("sidebar.") {
        return Action::Reveal { select, focus };
    }
    let order = selectable(&crate::sidebar(app));
    let position = |item: &ItemId| order.iter().position(|other| other == item);
    let current: Vec<ItemId> = app.session.selection.items().to_vec();
    let anchor = app.session.panel.anchor.clone();
    let select = match (position(&focus), anchor.as_ref().and_then(position)) {
        (Some(to), Some(from)) if keys.shift => {
            let mut picked = current;
            for item in &order[from.min(to)..=from.max(to)] {
                if !picked.contains(item) {
                    picked.push(item.clone());
                }
            }
            picked
        }
        (Some(_), _) if keys.meta || keys.control => {
            if current.contains(&focus) {
                current.into_iter().filter(|item| *item != focus).collect()
            } else {
                let mut picked = current;
                picked.push(focus.clone());
                picked
            }
        }
        _ => select,
    };
    if position(&focus).is_some() {
        app.session.panel.anchor = Some(focus.clone());
    }
    Action::Reveal { select, focus }
}
