//! Where the built-in panels are put: three rows across the top (the tab
//! row, the toolbar and the dock) and a list under the row its trigger is
//! in.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::Rect;
use specular_interact::panel::builtin::{CHROME_HEIGHT, PanelRect};
use specular_testkit::{TestApp, assert_panel_snapshot, page, shape, sticky};

const VIEWPORT: (f32, f32) = (1200.0, 800.0);

fn with(entity: specular_doc::Entity) -> TestApp {
    let id = entity.id.as_str().to_owned();
    let mut app = TestApp::with_entities([entity]);
    app.viewport(VIEWPORT).with_panels().select(&[&id]);
    app
}

fn dock(app: &TestApp) -> PanelRect {
    let Some(dock) = app.panel_layout().dock else {
        panic!("no dock");
    };
    dock.rect
}

fn open_list(app: &TestApp) -> PanelRect {
    let Some(list) = app.panel_layout().dropdown else {
        panic!("no dropdown is open");
    };
    list.rect
}

#[test]
fn the_three_rows_with_a_sticky_in_the_dock() {
    let app = with(sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"));
    assert_panel_snapshot!(app, @r#"
    tabs 0,0 1200x38
      view.canvas 86,5 180x28 ToolButton on: icon File 93,12 14x14; text "Canvas" 111,5 148x28 Left
      view.add 270,5 28x28 ToolButton: icon Plus 277,12 14x14
    toolbar 0,38 1200x40
      tool.select 372,44 32x28 ToolButton on: icon SelectTool 378,48 20x20
      - 412,50 1x16 Divider
      tool.draw 421,44 32x28 ToolButton: icon DrawPenTool 427,48 20x20 tint=1
      tool.sticky 457,44 32x28 ToolButton: icon StickyTool 463,48 20x20 tint=3
      tool.shape 493,44 32x28 ToolButton: icon ShapeTool 499,48 20x20 tint=1
      tool.page 529,44 32x28 ToolButton: icon PageTool 535,48 20x20
      tool.text 565,44 32x28 ToolButton: icon TextTool 571,48 20x20
      tool.document 601,44 32x28 ToolButton: icon DocumentTool 607,48 20x20
      - 641,50 1x16 Divider
      tool.comment 650,44 32x28 ToolButton: icon CommentTool 656,48 20x20
      tool.inspect 686,44 32x28 ToolButton: icon InspectTool 692,48 20x20
      - 726,50 1x16 Divider
      theme 735,44 32x28 ToolButton: icon SchemeSystem 741,48 20x20
      zoom 771,44 58x28 ToolMenu: text "100%" 780,44 34x28 Left; chevron 814,53 10x10
    dock 0,78 1200x40
      text.size 12,86 78x24 Button: text "Small" 18,86 50x24 Left; chevron 72,92 12x12
      text.font 94,86 68x24 Button: text "Sans" 100,86 40x24 Left; chevron 144,92 12x12
      - 170,90 1x16 Divider
      text.color 179,86 42x24 Button: dot none 183,90 16x16; chevron 203,92 12x12
      - 229,90 1x16 Divider
      item.annotate 238,86 24x24 Button: icon Annotate 243,91 14x14
      item.focus 266,86 24x24 Button: icon Focus 271,91 14x14
    "#);
}

#[test]
fn the_dock_is_there_and_ends_the_chrome_whatever_is_selected() {
    let mut app = with(sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"));
    let filled = dock(&app);
    assert_eq!(filled.bottom(), CHROME_HEIGHT);
    // A pan and a zoom move the sticky and leave the dock where it is.
    app.wheel((300.0, 200.0)).zoom(2.0);
    assert_eq!(dock(&app), filled);
    app.select(&[]);
    assert_eq!(dock(&app), filled);
    assert!(
        app.panel_layout()
            .dock
            .is_some_and(|dock| dock.nodes.is_empty())
    );
}

#[test]
fn a_pages_address_takes_the_room_the_other_controls_leave() {
    let mut app = with(page("p", Rect::new(300.0, 300.0, 700.0, 400.0)));
    let room = |app: &TestApp| {
        let dock = dock(app);
        let address = app.control_rect("page.url");
        let last = (app.panel_layout().dock.iter())
            .flat_map(|dock| &dock.nodes)
            .map(|node| node.rect.right())
            .fold(0.0, f32::max);
        (address.width, dock.right() - last)
    };
    let (narrow, pad) = room(&app);
    let reload = app.control_rect("page.reload").width;
    app.viewport((1600.0, 800.0));
    let (wide, wide_pad) = room(&app);
    assert_eq!(
        wide - narrow,
        400.0,
        "the address takes all of the new room"
    );
    assert_eq!(
        (pad, wide_pad),
        (12.0, 12.0),
        "the row ends at the dock's padding"
    );
    assert_eq!(app.control_rect("page.reload").width, reload);
}

#[test]
fn a_list_hangs_under_the_row_its_trigger_is_in() {
    let mut app = with(shape("s", Rect::new(500.0, 300.0, 200.0, 200.0)));
    // A list of words starts under its trigger.
    app.click_control("shape.size");
    let trigger = app.control_rect("shape.size");
    let list = open_list(&app);
    assert_eq!((list.x, list.y), (trigger.x, CHROME_HEIGHT + 6.0));
    // A row of glyphs is centred under it.
    app.click_control("shape.size").click_control("shape.align");
    let trigger = app.control_rect("shape.align");
    let list = open_list(&app);
    assert_eq!(
        (list.centre().x, list.y),
        (trigger.centre().x, CHROME_HEIGHT + 8.0)
    );
}

#[test]
fn the_zoom_levels_hang_from_the_toolbar_and_open_over_the_dock() {
    let mut app = with(sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"));
    app.click_control("zoom");
    let trigger = app.control_rect("zoom");
    let list = open_list(&app);
    assert_eq!(list.centre().x, trigger.centre().x);
    assert_eq!(list.y, dock(&app).y + 8.0);
    // The list is in front: a press on it never reaches the dock under it.
    app.click_control("zoom.50");
    assert!((app.session().camera.zoom - 0.5).abs() < 1e-6);
    assert_eq!(app.selected_ids(), ["t"]);
}
