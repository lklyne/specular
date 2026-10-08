//! What a click on a sidebar row selects, given the keys held
//! (`sidebarSelectionIntent`): a plain click only that row, a shift-click
//! the run of rows from the one picked last, and a command- or
//! control-click that row added to the selection or taken out of it.

use gpui_kit::Modifiers;
use specular_doc::ItemId;
use specular_interact::{RowKind, RowTarget, SidebarModel, SidebarRow};

/// The items a click can select, in the order the rows are shown. A group
/// is not one, and neither is a row inside a closed one.
pub(super) fn selectable(model: &SidebarModel) -> Vec<ItemId> {
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

/// The selection a click on the row for `focus` asks for. `plain` is what
/// the row selects with no key held, `current` the selection now, `anchor`
/// the row picked last and `order` every selectable row as shown.
pub(super) fn picked(
    order: &[ItemId],
    current: &[ItemId],
    anchor: Option<&ItemId>,
    focus: &ItemId,
    plain: Vec<ItemId>,
    keys: Modifiers,
) -> Vec<ItemId> {
    let position = |item: &ItemId| order.iter().position(|other| other == item);
    match (position(focus), anchor.and_then(position)) {
        (Some(to), Some(from)) if keys.shift => {
            let mut picked = current.to_vec();
            for item in &order[from.min(to)..=from.max(to)] {
                if !picked.contains(item) {
                    picked.push(item.clone());
                }
            }
            picked
        }
        (Some(_), _) if keys.platform || keys.control => {
            if current.contains(focus) {
                (current.iter().filter(|item| *item != focus).cloned()).collect()
            } else {
                let mut picked = current.to_vec();
                picked.push(focus.clone());
                picked
            }
        }
        _ => plain,
    }
}

#[cfg(test)]
mod tests {
    use specular_doc::EntityId;

    use super::*;

    fn item(name: &str) -> ItemId {
        ItemId::Entity(EntityId::from(name.to_owned()))
    }

    #[test]
    fn a_click_selects_by_the_keys_held() {
        let order = ["a", "b", "c", "d"].map(item);
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let command = Modifiers {
            platform: true,
            ..Modifiers::default()
        };
        // (keys, selection, anchor, clicked, picked)
        let cases = [
            (Modifiers::default(), vec!["a"], Some("a"), "c", vec!["c"]),
            (shift, vec!["a"], Some("a"), "c", vec!["a", "b", "c"]),
            (shift, vec!["d"], Some("d"), "b", vec!["d", "b", "c"]),
            (shift, vec!["a"], None, "c", vec!["c"]),
            (command, vec!["a"], Some("a"), "c", vec!["a", "c"]),
            (command, vec!["a", "c"], Some("a"), "c", vec!["a"]),
        ];
        for (keys, current, anchor, clicked, expected) in cases {
            let current: Vec<ItemId> = current.into_iter().map(item).collect();
            let anchor = anchor.map(item);
            let focus = item(clicked);
            let got = picked(
                &order,
                &current,
                anchor.as_ref(),
                &focus,
                vec![focus.clone()],
                keys,
            );
            let expected: Vec<ItemId> = expected.into_iter().map(item).collect();
            assert_eq!(got, expected, "{keys:?} on {clicked}");
        }
    }
}
