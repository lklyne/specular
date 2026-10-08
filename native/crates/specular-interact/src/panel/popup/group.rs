//! The popup of a group (`GroupPopup.tsx`): its color, and for one group
//! its layout: packed as a row or a column, and the gap it packs with.

use specular_doc::Entity;

use super::super::build::{color_dropdown, groups, noun, toggle};
use super::super::{
    Align, Control, ControlId, Face, Icon, PaintRole, Palette, PopupModel, Stepper,
};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, LayoutAxis, Property};

/// How far one press of the gap stepper goes: a grid square.
const GAP_STEP: f64 = 20.0;

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> PopupModel {
    let color = read::color(app);
    let color: Control = color_dropdown(
        ControlId::new("group.color"),
        &noun(entities.len(), "group", "groups"),
        Palette::Vivid,
        PaintRole::Fill,
        color.as_ref(),
        None,
        |color| Action::SetProperty(Property::Color(color)),
    );
    let titled = entities.iter().any(|entity| {
        entity
            .label
            .as_deref()
            .is_some_and(|label| !label.is_empty())
    });
    let anchor = if titled {
        super::over_titled(entities, Align::Center)
    } else {
        super::over(entities, Align::Center)
    };
    let noun = noun(entities.len(), "group", "groups");
    PopupModel {
        anchor,
        controls: groups(vec![
            vec![color],
            layout(app, entities),
            Actions::all(&noun, entities.len()).controls(),
        ]),
    }
}

/// One group's layout: a toggle for each direction, the one that is on
/// turning the layout off, and the gap while it is on.
fn layout(app: &crate::App, entities: &[&Entity]) -> Vec<Control> {
    let [group] = entities else {
        return Vec::new();
    };
    let managed = app.group_layout(&group.id);
    let id = ControlId::new("group.layout");
    let directions = [
        (LayoutAxis::X, "row", "Lay out as a row", Icon::ArrangeRow),
        (
            LayoutAxis::Y,
            "column",
            "Lay out as a column",
            Icon::ArrangeColumn,
        ),
    ];
    let mut controls: Vec<Control> = (directions.into_iter())
        .map(|(axis, name, label, icon)| {
            let on = managed.is_some_and(|(now, _)| now == axis);
            let next = (!on).then_some(axis);
            Control::Toggle(toggle(
                id.child(name),
                label,
                Face::icon(icon),
                on,
                Action::GroupLayout(next),
            ))
        })
        .collect();
    if let Some((_, gap)) = managed {
        controls.push(Control::Stepper(Stepper {
            id: ControlId::new("group.gap"),
            label: "Gap".into(),
            value: format!("{gap}").into(),
            decrement: Action::GroupGap((gap - GAP_STEP).max(0.0)),
            can_decrement: gap > 0.0,
            increment: Action::GroupGap(gap + GAP_STEP),
            can_increment: true,
        }));
    }
    controls
}
