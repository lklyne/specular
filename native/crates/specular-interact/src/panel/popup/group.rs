//! The popup of a group (`GroupPopup.tsx`): its color.

use specular_doc::Entity;

use super::super::build::{color_dropdown, groups, noun};
use super::super::{Align, Control, ControlId, PaintRole, Palette, PopupModel};
use super::actions::Actions;
use crate::property::read;
use crate::{Action, Property};

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
            Actions::all(&noun, entities.len()).controls(),
        ]),
    }
}
