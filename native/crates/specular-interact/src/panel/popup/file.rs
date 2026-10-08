//! The popup of a file (`FilePopup.tsx`): a Document's formatting buttons,
//! which can be pressed while its source is edited, then arrange, annotate
//! and focus. Renaming the file has no action here.

use specular_doc::Entity;

use super::super::build::{groups, noun};
use super::super::{Align, PopupModel};
use super::actions::Actions;

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> PopupModel {
    let all_documents = entities
        .iter()
        .all(|entity| crate::notes::note_file(&entity.kind).is_some());
    let noun = noun(entities.len(), "file", "files");
    let align = if entities.len() == 1 {
        Align::Stretch
    } else {
        Align::Center
    };
    PopupModel {
        anchor: super::over(entities, align),
        controls: groups(vec![
            if entities.len() == 1 && all_documents {
                super::text::formats(app, true)
            } else {
                Vec::new()
            },
            Actions::all(&noun, entities.len()).controls(),
        ]),
    }
}
