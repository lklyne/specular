//! Question 4 of the spike: a scenario step on Rune's GPU-free `Runtime`.
//! No adapter is asked for, no compositor is made, nothing is drawn.

use std::path::PathBuf;

use rune::Env;
use rune_shell_spike::{Options, app};
use specular_interact::Tool;

#[test]
fn a_tap_on_a_toolbar_control_arms_its_tool_through_update() {
    let canvas =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kitchen-sink.canvas");
    let options = Options {
        canvas: Some(canvas),
        still: true,
        ..Options::default()
    };
    let size = options.size;
    // The same `App` `main` runs, without its compositor.
    let mut rt = app(options).headless_runtime(size, Env::new(false, 1.0));
    rt.frame(0.0);

    // The canvas opened with no device: its pages are hosted, its images
    // decoded and its Documents read.
    assert_eq!(rt.state.app.document().entities().count(), 165);
    assert_eq!(rt.state.hosts.len(), 2, "both pages are hosted");
    assert!(!rt.state.images.is_empty(), "images are decoded");
    assert_eq!(rt.state.app.session().tool, Tool::Select);

    let (x, y) = rt
        .find("tool.sticky")
        .expect("the sticky tool is a keyed element");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(rt.state.app.session().tool, Tool::AddSticky);

    // The view was built again from the model: the button now says so.
    let tree = rt.accessibility_json().to_string();
    assert!(
        tree.contains(r#""key":"tool.sticky","name":"Sticky note","states":["focused","checked"]"#),
        "{tree}"
    );
}
