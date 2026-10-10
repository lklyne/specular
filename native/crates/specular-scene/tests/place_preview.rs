//! The scene snapshot for what an armed creation tool shows under the
//! pointer.

use specular_interact::Tool;
use specular_testkit::{TestApp, assert_scene_snapshot};

#[test]
fn an_armed_sticky_tool_draws_its_card_faded_over_the_page_under_the_pointer() {
    let mut app = TestApp::with_pages(1);
    app.tool(Tool::AddSticky).pointer_move((205.0, 195.0));
    assert_scene_snapshot!(app);
}
