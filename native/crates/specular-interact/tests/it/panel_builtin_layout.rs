//! Where the built-in panels are put: the toolbar across the top, the popup
//! beside what it points at and inside the viewport, a list under its
//! trigger.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::Rect;
use specular_interact::panel::builtin::{PanelRect, TOOLBAR_HEIGHT};
use specular_testkit::{TestApp, assert_panel_snapshot, page, shape, sticky};

const VIEWPORT: (f32, f32) = (1200.0, 800.0);
/// `POPUP_EDGE_MARGIN`.
const MARGIN: f32 = 8.0;

fn with(entity: specular_doc::Entity) -> TestApp {
    let id = entity.id.as_str().to_owned();
    let mut app = TestApp::with_entities([entity]);
    app.viewport(VIEWPORT).with_panels().select(&[&id]);
    app
}

fn popup(app: &TestApp) -> PanelRect {
    let Some(popup) = app.panel_layout().popup else {
        panic!("no popup");
    };
    popup.rect
}

fn open_list(app: &TestApp) -> PanelRect {
    let Some(list) = app.panel_layout().dropdown else {
        panic!("no dropdown is open");
    };
    list.rect
}

#[test]
fn the_toolbar_and_a_sticky_popup() {
    let app = with(sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"));
    assert_panel_snapshot!(app, @r#"
    toolbar 0,0 1200x44
      tool.select 372,8 32x28 ToolButton on: icon SelectTool 378,12 20x20
      - 412,14 1x16 Divider
      tool.draw 421,8 32x28 ToolButton: icon DrawPenTool 427,12 20x20 tint=1
      tool.sticky 457,8 32x28 ToolButton: icon StickyTool 463,12 20x20 tint=3
      tool.shape 493,8 32x28 ToolButton: icon ShapeTool 499,12 20x20 tint=1
      tool.page 529,8 32x28 ToolButton: icon PageTool 535,12 20x20
      tool.text 565,8 32x28 ToolButton: icon TextTool 571,12 20x20
      tool.document 601,8 32x28 ToolButton: icon DocumentTool 607,12 20x20
      - 641,14 1x16 Divider
      tool.comment 650,8 32x28 ToolButton: icon CommentTool 656,12 20x20
      tool.inspect 686,8 32x28 ToolButton: icon InspectTool 692,12 20x20
      - 726,14 1x16 Divider
      theme 735,8 32x28 ToolButton: icon SchemeSystem 741,12 20x20
      zoom 771,8 58x28 ToolMenu: text "100%" 780,8 34x28 Left; chevron 814,17 10x10
      sidebar.toggle 16,9 26x26 Subtle dimmed: glyph PanelLeft 22,15 14x14 Follow
    popup 456,252 288x34
      text.size 461,257 78x24 Button: text "Small" 467,257 50x24 Left; chevron 521,263 12x12
      text.font 543,257 68x24 Button: text "Sans" 549,257 40x24 Left; chevron 593,263 12x12
      - 619,261 1x16 Divider
      text.color 628,257 42x24 Button: dot none 632,261 16x16; chevron 652,263 12x12
      - 678,261 1x16 Divider
      item.annotate 687,257 24x24 Button: icon Annotate 692,262 14x14
      item.focus 715,257 24x24 Button: icon Focus 720,262 14x14
    "#);
}

#[test]
fn a_popup_follows_the_camera() {
    let mut app = with(sticky("t", Rect::new(500.0, 300.0, 200.0, 200.0), "note"));
    app.zoom(2.0);
    let popup = popup(&app);
    assert_eq!(
        (popup.centre().x, popup.bottom()),
        (1200.0 - MARGIN - popup.width / 2.0, 586.0)
    );
}

#[test]
fn a_popup_with_no_room_above_stays_under_the_toolbar_over_its_item() {
    // The Electron popup does not flip under its item: it stops at the edge
    // margin under the toolbar and covers the top of what it points at.
    let app = with(sticky("t", Rect::new(500.0, 60.0, 200.0, 200.0), "note"));
    assert_eq!(popup(&app).y, TOOLBAR_HEIGHT + MARGIN);
    let behind = with(sticky("t", Rect::new(500.0, -100.0, 200.0, 200.0), "note"));
    assert_eq!(popup(&behind).y, TOOLBAR_HEIGHT + MARGIN);
}

#[test]
fn a_popup_stops_at_the_sides_of_the_viewport() {
    let left = with(sticky("t", Rect::new(-150.0, 300.0, 200.0, 200.0), "note"));
    assert_eq!(popup(&left).x, MARGIN);
    let right = with(sticky("t", Rect::new(1150.0, 300.0, 200.0, 200.0), "note"));
    assert_eq!(popup(&right).right(), VIEWPORT.0 - MARGIN);
}

#[test]
fn a_popup_goes_when_its_item_leaves_the_canvas() {
    let gone = [
        Rect::new(-400.0, 300.0, 200.0, 200.0),
        Rect::new(1300.0, 300.0, 200.0, 200.0),
        Rect::new(500.0, 900.0, 200.0, 200.0),
        // Wholly behind the toolbar.
        Rect::new(500.0, -300.0, 200.0, 300.0),
    ];
    for rect in gone {
        let app = with(sticky("t", rect, "note"));
        assert!(app.panel_layout().popup.is_none(), "{rect:?}");
    }
}

#[test]
fn a_page_popup_is_as_wide_as_its_page_and_no_wider_than_the_viewport() {
    let app = with(page("p", Rect::new(300.0, 300.0, 700.0, 400.0)));
    let popup = popup(&app);
    assert_eq!((popup.x, popup.width), (300.0, 700.0));
    // A page narrower than the address field and its neighbours is still
    // given the room they need.
    let narrow = with(page("p", Rect::new(300.0, 300.0, 300.0, 400.0)));
    assert!(self::popup(&narrow).width > 550.0);
    let wide = with(page("p", Rect::new(-500.0, 300.0, 3000.0, 400.0)));
    let popup = self::popup(&wide);
    assert_eq!((popup.x, popup.width), (MARGIN, VIEWPORT.0 - MARGIN * 2.0));
}

#[test]
fn a_tool_popup_hangs_under_the_middle_of_the_toolbar() {
    let mut app = TestApp::empty();
    app.viewport(VIEWPORT)
        .with_panels()
        .click_control("tool.shape");
    let popup = popup(&app);
    assert_eq!((popup.centre().x, popup.y), (600.0, TOOLBAR_HEIGHT + 8.0));
}

#[test]
fn a_list_of_words_starts_under_its_trigger_and_a_row_of_glyphs_is_centred() {
    let mut app = with(shape("s", Rect::new(500.0, 300.0, 200.0, 200.0)));
    app.click_control("shape.size");
    let trigger = app.control_rect("shape.size");
    let list = open_list(&app);
    assert_eq!((list.x, list.y), (trigger.x, trigger.bottom() + 6.0));
    app.click_control("shape.size").click_control("shape.align");
    let trigger = app.control_rect("shape.align");
    let list = open_list(&app);
    assert_eq!(
        (list.centre().x, list.y),
        (trigger.centre().x, trigger.bottom() + 8.0)
    );
}

#[test]
fn a_list_with_no_room_under_its_trigger_opens_above_it() {
    let mut app = with(sticky("t", Rect::new(500.0, 760.0, 200.0, 200.0), "note"));
    app.click_control("text.size");
    let trigger = app.control_rect("text.size");
    let list = open_list(&app);
    assert_eq!(list.bottom(), trigger.y - 6.0);
}

#[test]
fn the_zoom_levels_hang_from_the_toolbar_and_open_over_the_popup() {
    let mut app = with(sticky("t", Rect::new(500.0, 60.0, 200.0, 200.0), "note"));
    app.click_control("zoom");
    let trigger = app.control_rect("zoom");
    let list = open_list(&app);
    assert_eq!(
        (list.centre().x, list.y),
        (trigger.centre().x, TOOLBAR_HEIGHT + 8.0)
    );
    assert_panel_snapshot!(app, @r#"
    toolbar 0,0 1200x44
      tool.select 372,8 32x28 ToolButton on: icon SelectTool 378,12 20x20
      - 412,14 1x16 Divider
      tool.draw 421,8 32x28 ToolButton: icon DrawPenTool 427,12 20x20 tint=1
      tool.sticky 457,8 32x28 ToolButton: icon StickyTool 463,12 20x20 tint=3
      tool.shape 493,8 32x28 ToolButton: icon ShapeTool 499,12 20x20 tint=1
      tool.page 529,8 32x28 ToolButton: icon PageTool 535,12 20x20
      tool.text 565,8 32x28 ToolButton: icon TextTool 571,12 20x20
      tool.document 601,8 32x28 ToolButton: icon DocumentTool 607,12 20x20
      - 641,14 1x16 Divider
      tool.comment 650,8 32x28 ToolButton: icon CommentTool 656,12 20x20
      tool.inspect 686,8 32x28 ToolButton: icon InspectTool 692,12 20x20
      - 726,14 1x16 Divider
      theme 735,8 32x28 ToolButton: icon SchemeSystem 741,12 20x20
      zoom 771,8 58x28 ToolMenu on hover: text "100%" 780,8 34x28 Left; chevron 814,17 10x10
      sidebar.toggle 16,9 26x26 Subtle dimmed: glyph PanelLeft 22,15 14x14 Follow
    popup 456,52 288x34
      text.size 461,57 78x24 Button: text "Small" 467,57 50x24 Left; chevron 521,63 12x12
      text.font 543,57 68x24 Button: text "Sans" 549,57 40x24 Left; chevron 593,63 12x12
      - 619,61 1x16 Divider
      text.color 628,57 42x24 Button: dot none 632,61 16x16; chevron 652,63 12x12
      - 678,61 1x16 Divider
      item.annotate 687,57 24x24 Button: icon Annotate 692,62 14x14
      item.focus 715,57 24x24 Button: icon Focus 720,62 14x14
    dropdown 715,52 170x178
      zoom.10 720,57 160x24 PresetRow: text "10%" 728,57 144x24 Left
      zoom.25 720,81 160x24 PresetRow: text "25%" 728,81 144x24 Left
      zoom.50 720,105 160x24 PresetRow: text "50%" 728,105 144x24 Left
      zoom.75 720,129 160x24 PresetRow: text "75%" 728,129 144x24 Left
      zoom.100 720,153 160x24 PresetRow on: text "100%" 728,153 144x24 Left; key "⌘0" 840,157 32x16
      zoom.150 720,177 160x24 PresetRow: text "150%" 728,177 144x24 Left
      zoom.200 720,201 160x24 PresetRow: text "200%" 728,201 144x24 Left
    "#);
}
