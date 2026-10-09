//! Reading the canvas: `GET /canvas`.

use serde_json::json;
use specular_interact::App;

use crate::{Response, Step, tabs};

/// The document of `app`'s canvas as a JSON Canvas file would hold it, with
/// the camera and the tab identity in `appState`. The identity is the
/// user's: which canvas they are looking at and every canvas there is, so a
/// reader can tell which one answered.
pub(crate) fn read(app: &App, user: &App) -> Result<Step, Response> {
    let mut canvas = app
        .document_to_save()
        .to_canvas_value()
        .map_err(|error| Response::error(500, error.to_string()))?;
    let camera = app.canvas_camera();
    let identity = tabs::identity(user);
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

/// `value` as the `f64` that prints the same digits, so a zoom of 0.8 is
/// not written as 0.800000011920929.
fn widened(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(f64::from(value))
}
