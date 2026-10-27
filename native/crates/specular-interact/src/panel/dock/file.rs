//! The dock's controls for a file (`FilePopup.tsx`): a Document's formatting buttons,
//! which can be pressed while its source is edited, the device frame of
//! images, then arrange and annotate. Renaming the file has no action here.

use specular_doc::{Entity, Kind};

use super::super::build::{groups, noun};
use super::super::{Control, ControlId, ControlsModel, Dropdown, Face, Icon};
use super::actions::Actions;
use super::page::{frame_choice, frame_color_choice};
use crate::App;
use crate::property::read;

pub(super) fn controls(app: &App, entities: &[&Entity]) -> ControlsModel {
    let all_documents = entities
        .iter()
        .all(|entity| crate::notes::note_file(&entity.kind).is_some());
    let all_images = entities
        .iter()
        .all(|entity| matches!(&entity.kind, Kind::File(file) if file.is_image()));
    let noun = noun(entities.len(), "file", "files");
    ControlsModel {
        controls: groups(vec![
            if entities.len() == 1 && all_documents {
                super::text::formats(app, true)
            } else {
                Vec::new()
            },
            if all_images {
                vec![frame_dropdown(app)]
            } else {
                Vec::new()
            },
            Actions::all(&noun, entities.len()).controls(),
        ]),
    }
}

/// The device frame the images are shown in, and its colour once it is
/// drawn. The device is the one each image's size says it was taken on.
fn frame_dropdown(app: &App) -> Control {
    let mut content = vec![frame_choice(app, "image.frame")];
    content
        .extend(read::any_device_frame(app).then(|| frame_color_choice(app, "image.frame-color")));
    let summary = match read::device_frame(app) {
        Some(None) => "No frame",
        Some(Some(_)) => read::framed_device(app).map_or("Frame", |device| device.label),
        None => "Multiple",
    };
    Control::Dropdown(Dropdown {
        id: ControlId::new("image.device"),
        label: "Device frame".into(),
        summary: Face {
            icon: Some(Icon::Device),
            ..Face::text(summary)
        },
        content,
    })
}
