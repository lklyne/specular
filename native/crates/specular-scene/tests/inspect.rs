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

#[test]
fn a_bare_node_has_no_remainder_and_a_colour_that_is_not_rgb_has_no_square() {
    let mut app = TestApp::with_pages(1);
    app.viewport((1600.0, 1000.0))
        .tool(Tool::Inspect)
        .pointer_move((200.0, 150.0));
    let node = specular_core::InspectedNode {
        node_id: "n".to_owned(),
        tag_name: "div".to_owned(),
        name: "div".to_owned(),
        selector: "div".to_owned(),
        id_attribute: None,
        classes: Vec::new(),
        styles: vec![
            ("color".to_owned(), "transparent".to_owned()),
            ("background".to_owned(), "rgb(1, 2, 3)".to_owned()),
        ],
        bounding_box: specular_core::PixelRect {
            x: 10,
            y: 20,
            width: 100,
            height: 40,
        },
    };
    app.send(specular_interact::Event::Page {
        page: "p1".into(),
        notice: specular_interact::PageNotice::Inspected {
            point: (100.0, 50.0).into(),
            pick: false,
            node: Some(Box::new(node)),
        },
    });
    assert_scene_snapshot!(app);
}
