//! The popup of a file (`FilePopup.tsx`). A Document being edited has the
//! formatting buttons. Nothing else in it has an action to run here.

use specular_doc::Entity;

use super::super::{Align, PopupModel};

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> Option<PopupModel> {
    let controls = super::text::formats(app);
    if controls.is_empty() {
        return None;
    }
    let align = if entities.len() == 1 {
        Align::Stretch
    } else {
        Align::Center
    };
    Some(PopupModel {
        anchor: super::over(entities, align),
        controls,
    })
}
