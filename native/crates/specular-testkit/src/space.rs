//! Spaces for tests: several canvases in one app, with no folder behind
//! them.

use specular_core::Camera;
use specular_doc::Document;
use specular_interact::{CanvasId, Event, OpenedCanvas, OpenedSpace};

use crate::TestApp;

/// A space as the shell would read it from a folder: one canvas for each
/// `(name, document)`, with the ids `tab_1`, `tab_2` and so on, each kept
/// in `<name>.canvas`, at zoom 1 with no pan. The first is the active one.
pub(crate) fn space<'a>(canvases: impl IntoIterator<Item = (&'a str, Document)>) -> OpenedSpace {
    let canvases = canvases
        .into_iter()
        .enumerate()
        .map(|(index, (name, document))| OpenedCanvas {
            id: CanvasId::new(format!("tab_{}", index + 1)),
            name: name.to_owned(),
            file: format!("{name}.canvas"),
            document,
            camera: Camera::default(),
            tabs: Vec::new(),
        })
        .collect();
    OpenedSpace {
        folder: Some("/space".to_owned()),
        canvases,
        active: None,
    }
}

impl TestApp {
    /// An app showing the first canvas of [`space`]`(canvases)`. The
    /// effects of opening it are dropped.
    pub fn with_space<'a>(canvases: impl IntoIterator<Item = (&'a str, Document)>) -> Self {
        let mut app = Self::empty();
        app.send(Event::SpaceOpened(Box::new(space(canvases))));
        app.take_effects();
        app
    }
}
