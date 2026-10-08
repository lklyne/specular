//! Reading the canvas: `GET /canvas` and `GET /tabs`.

use serde_json::{Value, json};
use specular_interact::App;

use crate::{Response, Step, Tab};

/// The document as a JSON Canvas file would hold it, with the camera and
/// the tab identity in `appState` so a reader knows which canvas answered.
pub(crate) fn read(app: &App, tab: &Tab) -> Result<Step, Response> {
    let mut canvas = app
        .document_to_save()
        .to_canvas_value()
        .map_err(|error| Response::error(500, error.to_string()))?;
    let camera = app.session().camera;
    let identity = tabs(app, tab);
    if let Some(top) = canvas.as_object_mut() {
        let state = top.entry("appState").or_insert_with(|| json!({}));
        if !state.is_object() {
            *state = json!({});
        }
        state["zoom"] = json!(widened(camera.zoom));
        state["pan"] = json!({ "x": widened(camera.pan.x), "y": widened(camera.pan.y) });
        state["activeTab"] = identity["activeTab"].clone();
        state["tabs"] = identity["tabs"].clone();
    }
    Ok(Step::Answer(canvas))
}

/// Every canvas in the workspace, which here is the one that is open.
pub(crate) fn tabs(app: &App, tab: &Tab) -> Value {
    json!({
        "activeTab": { "id": tab.id, "name": tab.name },
        "tabs": [{
            "id": tab.id,
            "name": tab.name,
            "entityCount": app.document().entities().count(),
        }],
    })
}

/// `value` as the `f64` that prints the same digits, so a zoom of 0.8 is
/// not written as 0.800000011920929.
fn widened(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}
