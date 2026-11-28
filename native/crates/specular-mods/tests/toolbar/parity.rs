//! The shipped toolbar against the native toolbar's model: the same
//! controls by id, what its props hide left out, and a press that reaches
//! the host.

use gpui::{Modifiers, TestAppContext, point, px};
use serde_json::Value;
use specular_interact::state;

use super::{Loaded, floating_app, window_app};

/// Every control id `panels/toolbar` lists for the toolbar region: the
/// tools, the theme button and the zoom readout.
fn listed_ids(toolbar: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for section in toolbar["sections"].as_array().into_iter().flatten() {
        for tool in section["tools"].as_array().into_iter().flatten() {
            ids.extend(tool["id"].as_str().map(str::to_owned));
        }
        ids.extend(section["zoom"]["id"].as_str().map(str::to_owned));
    }
    ids.extend(toolbar["theme"]["id"].as_str().map(str::to_owned));
    ids
}

/// Whether the tree names a control `id`: a button by that id, or the
/// zoom readout as the `Zoom` host component.
fn shows(tree: &str, id: &str) -> bool {
    if id == "zoom" {
        return tree.contains("module_component specular.Zoom \"zoom\"");
    }
    tree.contains(&format!("\"{id}\""))
}

#[gpui::test]
fn the_toolbar_shows_every_control_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    for app in [window_app(), floating_app()] {
        let place = state(app.app())["panels/toolbar"]["tools"].clone();
        let listed = listed_ids(&state(app.app())["panels/toolbar"]);
        assert!(
            listed.len() >= 11,
            "the model lists the tools, the theme and the zoom: {listed:?}"
        );
        for hidden in [
            &[][..],
            &["tool.inspect", "zoom"],
            &["theme", "tool.select"],
        ] {
            let tree = Loaded::new(&app, hidden, cx).tree();
            for id in &listed {
                let hide = hidden.contains(&id.as_str());
                assert_eq!(
                    shows(&tree, id),
                    !hide,
                    "`{id}` with {hidden:?} hidden, tools {place}:\n{tree}"
                );
            }
        }
    }
}

#[gpui::test]
fn a_click_on_a_tool_presses_it_through_the_host(cx: &mut TestAppContext) {
    let app = window_app();
    let mut loaded = Loaded::new(&app, &[], cx);
    // Down the docked strip, a click every few pixels, until the draw
    // tool's button has been hit.
    for y in (2..400).step_by(4) {
        let at = point(px(18.0), px(y as f32));
        loaded.vcx.simulate_click(at, Modifiers::default());
        loaded.vcx.run_until_parked();
        if loaded
            .host
            .0
            .borrow()
            .iter()
            .any(|run| run.contains("tool.draw"))
        {
            break;
        }
    }
    let runs = loaded.host.0.borrow().clone();
    assert!(
        runs.contains(&r#"control.press {"id":"tool.draw"}"#.to_owned()),
        "the draw tool's press reached the host: {runs:?}"
    );
    assert_eq!(
        runs.first().map(String::as_str),
        Some(r#"control.press {"id":"tool.select"}"#),
        "the first button down the strip is the select tool"
    );
}
