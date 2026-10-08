//! The item popup for pages, edges, files, several items, and while a text
//! is edited.

use specular_doc::{Color, ColorPreset, Edge, EdgeEnd, LineStyle, Rect};
use specular_testkit::{
    TestApp, assert_popup_snapshot, connected, document, page, plain_text, shape, sticky, with_edge,
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
    button page.back "Back" icon=ChevronLeft chord=cmd+[ disabled -> PageBack
    button page.forward "Forward" icon=ChevronRight chord=cmd+] disabled -> PageForward
    button page.reload "Reload" icon=Reload chord=cmd+r -> PageReload
    ---
    field page.url "Page address" value="https://example.com/p" placeholder="Type a URL" Wide submit=PageUrl
    ---
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
      controls
        field page.size.width "Page width" caption="W" value="375" Short submit=ViewportWidth
        field page.size.height "Page height" caption="H" value="667" Short submit=ViewportHeight
    ---
    toggle [ ] page.frame "Device frame" icon=Device -> SetProperty(DeviceFrame(true))
    button page.rotate "Rotate viewport" icon=Rotate -> SetProperty(Orientation(Landscape))
    ---
    button page.scheme "Color scheme: System. Click to change." icon=SchemeSystem -> SetProperty(ColorScheme(Some(Light)))
    ---
    button item.focus "Focus page" icon=Focus -> FocusSelection
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
    ---
    button item.arrange.row "Arrange in a row" icon=ArrangeRow -> Arrange(Row)
    button item.arrange.column "Arrange in a column" icon=ArrangeColumn -> Arrange(Column)
    button item.arrange.grid "Arrange in a grid" icon=ArrangeGrid -> Arrange(Grid)
    button item.annotate "Annotate 2 pages" icon=Annotate -> AnnotateSelection
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
    field edge.label "Edge label" value="" placeholder="Label…" Medium submit=EdgeLabel
    ---
    button edge.delete "Delete edge" icon=Trash chord=backspace -> Delete
    "#);
}

#[test]
fn a_selection_across_kinds_has_no_popup_yet() {
    let mut app = TestApp::with_entities([shape("a", A), sticky("b", B, "two")]);
    app.select(&["a", "b"]);
    assert_popup_snapshot!(app, @r#"
    anchor canvas 100,100 500x100 Above Center gap=14
    button item.arrange.row "Arrange in a row" icon=ArrangeRow -> Arrange(Row)
    button item.arrange.column "Arrange in a column" icon=ArrangeColumn -> Arrange(Column)
    button item.arrange.grid "Arrange in a grid" icon=ArrangeGrid -> Arrange(Grid)
    button item.annotate "Annotate 2 items" icon=Annotate -> AnnotateSelection
    button item.focus "Focus 2 items" icon=Focus -> FocusSelection
    "#);
}

#[test]
fn no_popup_for_nothing_a_drag_a_pair_of_edges_or_a_shape_label() {
    let none = |app: &TestApp, what: &str| assert_eq!(app.popup_snapshot(), "none", "{what}");

    let mut app = TestApp::with_entities([shape("a", A)]);
    none(&app, "nothing selected");
    app.select(&["a"]);
    assert_ne!(app.popup_snapshot(), "none");
    app.press((150.0, 150.0)).drag_to((200.0, 200.0));
    none(&app, "a drag in flight");
    app.release();
    assert_ne!(app.popup_snapshot(), "none");

    let doc = document([plain_text("a", A, "a"), plain_text("b", B, "b")]);
    let mut app = TestApp::from_document(connected(connected(doc, "e1", "a", "b"), "e2", "b", "a"));
    app.select(&["e1", "e2"]);
    none(&app, "two edges");

    let mut app = TestApp::with_entities([shape("a", A)]);
    app.double_click((150.0, 150.0));
    assert!(app.app().session().editing.is_some());
    none(&app, "a shape label being edited");
}
