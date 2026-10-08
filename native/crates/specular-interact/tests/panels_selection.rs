//! The item popup for pages, edges, files, several items, and while a text
//! is edited.

use specular_doc::{Color, ColorPreset, Edge, EdgeEnd, LineStyle, Rect};
use specular_testkit::{
    TestApp, assert_popup_snapshot, document, note, page, plain_text, shape, sticky, with_edge,
};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const RED: Color = Color::Preset(ColorPreset::Red);

#[test]
fn a_page_has_size_frame_rotation_and_color_scheme() {
    let mut app = TestApp::with_entities([page("p", Rect::new(100.0, 100.0, 375.0, 667.0))]);
    app.select(&["p"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 375x667 Above Stretch gap=28.5
    dropdown page.size "Page size" shows text="Custom"
      options list
        option [ ] page.size.0 "iPhone SE" trailing="375×667" -> SetProperty(ViewportPreset(0))
        option [ ] page.size.1 "iPhone 14 Pro" text="iPhone Pro" trailing="393×852" -> SetProperty(ViewportPreset(1))
        option [ ] page.size.2 "iPhone 14 Pro Max" text="iPhone Pro Max" trailing="430×932" -> SetProperty(ViewportPreset(2))
        option [ ] page.size.9 "iPhone Duo (cover)" trailing="466×678" -> SetProperty(ViewportPreset(9))
        option [ ] page.size.10 "iPhone Duo (open)" trailing="626×890" -> SetProperty(ViewportPreset(10))
      options list
        option [ ] page.size.3 "iPad Mini" trailing="744×1133" -> SetProperty(ViewportPreset(3))
        option [ ] page.size.4 "iPad Pro 11" trailing="834×1194" -> SetProperty(ViewportPreset(4))
        option [ ] page.size.5 "iPad Pro 12.9" trailing="1024×1366" -> SetProperty(ViewportPreset(5))
      options list
        option [ ] page.size.6 "Laptop" trailing="1280×800" -> SetProperty(ViewportPreset(6))
        option [ ] page.size.7 "Desktop" trailing="1440×900" -> SetProperty(ViewportPreset(7))
        option [ ] page.size.8 "Desktop XL" trailing="1920×1080" -> SetProperty(ViewportPreset(8))
      options list
        option [x] page.size.custom "Custom" -> SetProperty(CustomViewport)
    ---
    toggle [ ] page.frame "Device frame" icon=Device -> SetProperty(DeviceFrame(true))
    button page.rotate "Rotate viewport" icon=Rotate -> SetProperty(Orientation(Landscape))
    ---
    button page.scheme "Color scheme: System. Click to change." icon=SchemeSystem -> SetProperty(ColorScheme(Some(Light)))
    "#);
}

#[test]
fn several_pages_share_a_size_list_without_custom() {
    let mut app = TestApp::with_pages(2);
    app.select(&["p1", "p2"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 1000x300 Above Center gap=28.5
    dropdown page.size "Page size" shows text="Multiple"
      options list
        option [ ] page.size.0 "iPhone SE" trailing="375×667" -> SetProperty(ViewportPreset(0))
        option [ ] page.size.1 "iPhone 14 Pro" text="iPhone Pro" trailing="393×852" -> SetProperty(ViewportPreset(1))
        option [ ] page.size.2 "iPhone 14 Pro Max" text="iPhone Pro Max" trailing="430×932" -> SetProperty(ViewportPreset(2))
        option [ ] page.size.9 "iPhone Duo (cover)" trailing="466×678" -> SetProperty(ViewportPreset(9))
        option [ ] page.size.10 "iPhone Duo (open)" trailing="626×890" -> SetProperty(ViewportPreset(10))
      options list
        option [ ] page.size.3 "iPad Mini" trailing="744×1133" -> SetProperty(ViewportPreset(3))
        option [ ] page.size.4 "iPad Pro 11" trailing="834×1194" -> SetProperty(ViewportPreset(4))
        option [ ] page.size.5 "iPad Pro 12.9" trailing="1024×1366" -> SetProperty(ViewportPreset(5))
      options list
        option [ ] page.size.6 "Laptop" trailing="1280×800" -> SetProperty(ViewportPreset(6))
        option [ ] page.size.7 "Desktop" trailing="1440×900" -> SetProperty(ViewportPreset(7))
        option [ ] page.size.8 "Desktop XL" trailing="1920×1080" -> SetProperty(ViewportPreset(8))
    ---
    toggle [ ] page.frame "Toggle device frame for selected pages" icon=Device -> SetProperty(DeviceFrame(true))
    "#);
}

#[test]
fn a_selected_edge_has_color_stroke_arrowheads_and_delete() {
    let doc = document([plain_text("a", A, "a"), plain_text("b", B, "b")]);
    let edge = Edge {
        color: Some(RED),
        line_style: Some(LineStyle::Dashed),
        from_end: Some(EdgeEnd::Arrow),
        ..Edge::new("e", "a", "b")
    };
    let mut app = TestApp::from_document(with_edge(doc, edge));
    app.select(&["e"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 350,150 0x0 Above Center gap=12
    dropdown edge.color "Set edge color" shows color=1
      controls
        swatches edge.color.swatches Vivid/Ink: neutral purple blue cyan green yellow orange *red
    ---
    dropdown edge.stroke "Edge stroke" shows icon=Border
      controls
        toggle [x] edge.stroke.thin "Thin edge" icon=StrokeThin -> SetProperty(StrokeWidth(1.5))
        toggle [ ] edge.stroke.thick "Thick edge" icon=StrokeThick -> SetProperty(StrokeWidth(3.0))
        ---
        toggle [ ] edge.stroke.solid "Regular edge" icon=LineSolid -> SetProperty(LineStyle(Solid))
        toggle [x] edge.stroke.dashed "Dashed edge" icon=LineDashed -> SetProperty(LineStyle(Dashed))
    ---
    toggle [x] edge.start "Toggle start arrowhead" icon=ArrowStart -> SetProperty(FromEnd(None))
    toggle [x] edge.end "Toggle end arrowhead" icon=ArrowEnd -> SetProperty(ToEnd(None))
    ---
    button edge.delete "Delete edge" icon=Trash chord=backspace -> Delete
    "#);
}

#[test]
fn a_selection_across_kinds_has_no_popup_yet() {
    let mut app = TestApp::with_entities([shape("a", A), sticky("b", B, "two")]);
    app.select(&["a", "b"]);
    assert_popup_snapshot!(app, @r#"
    none
    "#);
}

#[test]
fn no_popup_while_a_drag_is_in_flight() {
    let mut app = TestApp::with_entities([shape("a", A)]);
    app.select(&["a"]);
    assert_ne!(app.popup_snapshot(), "none");
    app.press((150.0, 150.0)).drag_to((200.0, 200.0));
    assert_popup_snapshot!(app, @r#"
    none
    "#);
    app.release();
    assert_ne!(app.popup_snapshot(), "none");
}

#[test]
fn a_sticky_being_edited_adds_the_formatting_buttons() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.double_click((150.0, 150.0));
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 200x100 Above Center gap=14
    dropdown text.size "Set text size" shows text="Small"
      options list
        option [x] text.size.14 "Small" -> SetProperty(TextSize(14.0))
        option [ ] text.size.32 "Medium" -> SetProperty(TextSize(32.0))
        option [ ] text.size.56 "Large" -> SetProperty(TextSize(56.0))
        option [ ] text.size.96 "Extra large" -> SetProperty(TextSize(96.0))
        option [ ] text.size.144 "Huge" -> SetProperty(TextSize(144.0))
      controls
        stepper text.size.custom "Custom text size in pixels" value=14 dec -> SetProperty(TextSize(13.0)) inc -> SetProperty(TextSize(15.0))
    dropdown text.font "Set text font" shows text="Sans" font=Sans
      options list
        option [x] text.font.sans "Sans" text="Sans" font=Sans -> SetProperty(TextFont(Sans))
        option [ ] text.font.mono "Mono" text="Mono" font=Mono -> SetProperty(TextFont(Mono))
        option [ ] text.font.hand "Hand" text="Hand" font=Hand -> SetProperty(TextFont(Hand))
    ---
    dropdown text.color "Set sticky note color" shows hollow
      controls
        swatches text.color.swatches Soft/Fill: neutral purple blue cyan green yellow orange red
    ---
    button format.bold "Bold" icon=Bold chord=cmd+b -> Format(Bold)
    button format.strikethrough "Strikethrough" icon=Strikethrough chord=cmd+shift+x -> Format(Strike)
    button format.bullets "Bullet list" icon=BulletList chord=cmd+shift+8 -> Format(BulletList)
    "#);
}

#[test]
fn a_document_being_edited_has_the_formatting_buttons() {
    let mut app =
        TestApp::with_entities([note("n", Rect::new(100.0, 100.0, 300.0, 300.0), "plan.md")]);
    app.note_text("plan.md", "hello");
    app.select(&["n"]);
    assert_popup_snapshot!(app, @r#"
    none
    "#);
    app.double_click((150.0, 200.0));
    assert!(app.app().session().editing.is_some());
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 300x300 Above Stretch gap=14
    button format.bold "Bold" icon=Bold chord=cmd+b -> Format(Bold)
    button format.strikethrough "Strikethrough" icon=Strikethrough chord=cmd+shift+x -> Format(Strike)
    button format.bullets "Bullet list" icon=BulletList chord=cmd+shift+8 -> Format(BulletList)
    "#);
}
