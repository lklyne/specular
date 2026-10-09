//! The dock's controls for a file (`FilePopup.tsx`): a Document's formatting buttons,
//! which can be pressed while its source is edited, then arrange and
//! annotate. Renaming the file has no action here.

use specular_doc::Entity;

use super::super::ControlsModel;
use super::super::build::{groups, noun};
use super::actions::Actions;

pub(super) fn controls(app: &crate::App, entities: &[&Entity]) -> ControlsModel {
    let all_documents = entities
        .iter()
        .all(|entity| crate::notes::note_file(&entity.kind).is_some());
    let noun = noun(entities.len(), "file", "files");
    ControlsModel {
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
