//! Spaces for tests: several canvases in one app, with no folder behind
//! them.

use specular_core::Camera;
use specular_doc::Document;
use specular_interact::{Action, CanvasAction, CanvasId, Event, OpenedCanvas, OpenedSpace};

use crate::TestApp;

/// A space as the shell would read it from a folder: one canvas for each
/// `(name, document)`, with the ids `tab_1`, `tab_2` and so on, each kept
/// in `<name>.canvas`, at zoom 1 with no pan. The first is the active one.
pub fn space<'a>(canvases: impl IntoIterator<Item = (&'a str, Document)>) -> OpenedSpace {
    let canvases = canvases
        .into_iter()
        .enumerate()
        .map(|(index, (name, document))| OpenedCanvas {
            id: CanvasId::new(format!("tab_{}", index + 1)),
            name: name.to_owned(),
            file: format!("{name}.canvas"),
            document,
            camera: Camera::default(),
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

    /// The names of the space's canvases, in order.
    pub fn canvas_names(&self) -> Vec<&str> {
        (self.app().space().canvases().iter())
            .map(|canvas| canvas.name.as_str())
            .collect()
    }

    /// The name of the canvas the app shows.
    pub fn active_canvas(&self) -> &str {
        &self.app().space().active().name
    }

    /// The id of the canvas called `name`.
    #[track_caller]
    pub fn canvas_id(&self, name: &str) -> CanvasId {
        match self.app().space().resolve(name) {
            Ok(canvas) => canvas.id.clone(),
            Err(error) => panic!("{error}"),
        }
    }

    /// Shows the canvas called `name`.
    #[track_caller]
    pub fn switch_to(&mut self, name: &str) -> &mut Self {
        let id = self.canvas_id(name);
        self.act(Action::Canvas(CanvasAction::Switch(id)))
    }
}
