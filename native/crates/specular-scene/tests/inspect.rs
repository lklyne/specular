//! Scene snapshot of the inspect tool: the outline on the node and the card
//! over it. Text is measured by the testkit's `FixedAdvance`.

use specular_interact::Tool;
use specular_testkit::{TestApp, assert_scene_snapshot};

#[test]
fn the_outline_and_the_popover_are_drawn_over_the_canvas() {
    let mut app = TestApp::with_pages(1);
    app.viewport((1600.0, 1000.0))
        .tool(Tool::Inspect)
        .pointer_move((200.0, 150.0))
        .answer_inspect();
    assert_scene_snapshot!(app);
}
