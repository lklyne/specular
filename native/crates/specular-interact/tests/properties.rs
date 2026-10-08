//! The popup controls' property actions: each applies to the selected items
//! it means something for, skips the rest, and is one undo step.

use specular_doc::{
    BorderStyle, BrushType, Color, ColorPreset, Drawing, Entity, FillStyle, JsonMap, Kind,
    LineStyle, Point, Rect, ShapeKind, Stroke, TextAlign, TextFont, TextStyle, VerticalAlign,
};
use specular_interact::{Action, Key, Property, Tool, property};
use specular_testkit::{
    TestApp, assert_doc_snapshot, connected, document, drawing, file, group, page, plain_text,
    shape, sticky, text,
};

const RED: Color = Color::Preset(ColorPreset::Red);
const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);

fn set(app: &mut TestApp, property: Property) {
    app.act(Action::SetProperty(property));
}

/// A drawing of one stroke, with the rect the draw tool gives it.
fn ink(color: Color, width: f64, brush: Option<BrushType>) -> Entity {
    let pad = width / 2.0;
    let stroke = Stroke {
        id: "ink".to_owned(),
        color,
        width,
        points: vec![Point::new(120.0, 120.0), Point::new(160.0, 150.0)],
        brush,
        extra: JsonMap::new(),
    };
    Entity {
        kind: Kind::Drawing(Drawing {
            strokes: vec![stroke],
        }),
        ..drawing(
            "d",
            Rect::new(120.0 - pad, 120.0 - pad, 40.0 + width, 30.0 + width),
        )
    }
}

fn steps(app: &TestApp) -> bool {
    app.app().can_undo()
}

// Text.

#[test]
fn text_ink_size_and_font_apply_to_every_selected_text() {
    let mut app = TestApp::with_entities([sticky("a", A, "one"), sticky("b", B, "two")]);
    app.select(&["a", "b"]);
    set(&mut app, Property::Color(RED));
    set(&mut app, Property::TextSize(32.0));
    set(&mut app, Property::TextFont(TextFont::Mono));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":458,"text":"one","color":"1","specular":{"textStyle":"sticky","textSize":32,"textFont":"mono"}}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":458,"text":"two","color":"1","specular":{"textStyle":"sticky","textSize":32,"textFont":"mono"}}
    edges:
    specular: {"entityOrder":["a","b"]}
    "#);
    assert_eq!(property::read::text_size(app.app()), Some(32.0));
    assert_eq!(property::read::text_font(app.app()), Some(TextFont::Mono));
    assert_eq!(property::read::color(app.app()), Some(RED));
    app.undo().undo().undo();
    assert!(!steps(&app), "each pick was one step");
    app.redo().redo().redo().assert_undo_returns_to_start();
}

#[test]
fn a_text_style_swaps_to_what_the_other_creation_tool_stamps() {
    let mut app = TestApp::with_entities([sticky("a", A, "one"), plain_text("b", B, "two")]);
    app.select(&["a", "b"]);
    assert_eq!(property::read::text_style(app.app()), None);
    set(&mut app, Property::TextStyle(TextStyle::Plain));
    assert_eq!(
        property::read::text_style(app.app()),
        Some(TextStyle::Plain)
    );
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":64,"height":20,"text":"one","color":"1","specular":{"textStyle":"plain","widthMode":"auto","colorRole":"neutral"}}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"two","specular":{"textStyle":"plain","widthMode":"auto"}}
    edges:
    specular: {"entityOrder":["a","b"]}
    "#);
    set(&mut app, Property::TextStyle(TextStyle::Sticky));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":64,"height":200,"text":"one","color":"3","specular":{"textStyle":"sticky","widthMode":"fixed"}}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":200,"text":"two","color":"3","specular":{"textStyle":"sticky","widthMode":"fixed"}}
    edges:
    specular: {"entityOrder":["a","b"]}
    "#);
    app.undo().undo();
    assert!(!steps(&app), "each swap was one step");
    app.redo().redo().assert_undo_returns_to_start();
}

#[test]
fn a_style_the_text_already_has_records_no_step() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"]);
    set(&mut app, Property::TextStyle(TextStyle::Sticky));
    assert!(!steps(&app));
}

#[test]
fn a_size_outside_the_popup_range_is_held_to_it() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"]);
    set(&mut app, Property::TextSize(400.0));
    assert_eq!(property::read::text_size(app.app()), Some(256.0));
    set(&mut app, Property::TextSize(2.0));
    assert_eq!(property::read::text_size(app.app()), Some(8.0));
}

#[test]
fn a_property_that_would_change_nothing_records_no_step() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"]);
    set(&mut app, Property::Color(RED));
    set(&mut app, Property::Color(RED));
    app.undo();
    assert!(!steps(&app), "the second pick of red changed nothing");
    // A property for a kind the selection lacks changes nothing either.
    set(&mut app, Property::LineStyle(LineStyle::Dashed));
    assert!(!steps(&app));
}

#[test]
fn nothing_is_set_on_an_empty_selection() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    set(&mut app, Property::Color(RED));
    assert!(!steps(&app));
}

// A mixed selection.

fn everything() -> TestApp {
    let doc = document([
        text("t", A),
        shape("s", B),
        group("g", Rect::new(700.0, 100.0, 300.0, 300.0)),
        ink(Color::Neutral, 2.0, None),
        page("p", Rect::new(100.0, 700.0, 400.0, 300.0)),
        file("f", Rect::new(600.0, 700.0, 100.0, 100.0)),
    ]);
    TestApp::from_document(connected(doc, "e", "t", "s"))
}

#[test]
fn a_color_reaches_every_kind_that_has_one_and_skips_pages_and_files() {
    let mut app = everything();
    app.select(&["t", "s", "g", "d", "p", "f", "e"]);
    set(&mut app, Property::Color(RED));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"t","type":"text","x":100,"y":100,"width":200,"height":100,"text":"t","color":"1"}
      {"id":"s","type":"shape","x":400,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":"","color":"1"}
      {"id":"g","type":"group","x":700,"y":100,"width":300,"height":300,"color":"1","groupColor":"1"}
      {"id":"d","type":"drawing","x":119,"y":119,"width":42,"height":32,"strokes":[{"id":"ink","color":"1","width":2,"points":[{"x":120,"y":120},{"x":160,"y":150}]}]}
      {"id":"p","type":"link","x":100,"y":700,"width":400,"height":300,"url":"https://example.com/p"}
      {"id":"f","type":"file","x":600,"y":700,"width":100,"height":100,"file":"f.png"}
    edges:
      {"id":"e","fromNode":"t","toNode":"s","color":"1"}
    specular: {"entityOrder":["t","s","g","d","p","f","e"]}
    "#);
    assert_eq!(property::read::color(app.app()), Some(RED));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_property_for_one_kind_leaves_the_others_in_the_selection_alone() {
    let mut app = everything();
    app.select(&["t", "s", "d", "e"]);
    set(&mut app, Property::TextSize(24.0));
    assert_eq!(property::read::text_size(app.app()), Some(24.0));
    set(&mut app, Property::StrokeWidth(4.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"t","type":"text","x":100,"y":100,"width":200,"height":343,"text":"t","specular":{"textSize":24}}
      {"id":"s","type":"shape","x":400,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":"","strokeWidth":4,"specular":{"textSize":24}}
      {"id":"g","type":"group","x":700,"y":100,"width":300,"height":300}
      {"id":"d","type":"drawing","x":118,"y":118,"width":44,"height":34,"strokes":[{"id":"ink","color":"neutral","width":4,"points":[{"x":120,"y":120},{"x":160,"y":150}]}]}
      {"id":"p","type":"link","x":100,"y":700,"width":400,"height":300,"url":"https://example.com/p"}
      {"id":"f","type":"file","x":600,"y":700,"width":100,"height":100,"file":"f.png"}
    edges:
      {"id":"e","fromNode":"t","toNode":"s","strokeWidth":4}
    specular: {"entityOrder":["t","s","g","d","p","f","e"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_property_applies_to_a_selection_only_where_it_holds_a_target() {
    let enabled = |app: &TestApp, property: Property| property.applies_to(app.app());
    let mut app = everything();
    app.select(&["p", "f"]);
    assert!(!enabled(&app, Property::Color(RED)));
    assert!(enabled(&app, Property::ViewportPreset(0)));
    app.select(&["p", "t"]);
    assert!(enabled(&app, Property::Color(RED)));
    assert!(!enabled(&app, Property::ShapeKind(ShapeKind::Ellipse)));
    app.select(&[]);
    assert!(!enabled(&app, Property::Color(RED)));
}

// Shapes.

#[test]
fn shape_properties_set_the_silhouette_fill_and_label() {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B)]);
    app.select(&["a", "b"]);
    set(&mut app, Property::ShapeKind(ShapeKind::Diamond));
    set(&mut app, Property::TextSize(20.0));
    set(&mut app, Property::TextAlign(TextAlign::Left));
    set(&mut app, Property::TextVerticalAlign(VerticalAlign::Top));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":100,"y":100,"width":200,"height":100,"shapeKind":"diamond","text":"","specular":{"textSize":20,"textAlign":"left","textVerticalAlign":"top"}}
      {"id":"b","type":"shape","x":400,"y":100,"width":200,"height":100,"shapeKind":"diamond","text":"","specular":{"textSize":20,"textAlign":"left","textVerticalAlign":"top"}}
    edges:
    specular: {"entityOrder":["a","b"]}
    "#);
    assert_eq!(
        property::read::shape_kind(app.app()),
        Some(ShapeKind::Diamond)
    );
    assert_eq!(property::read::text_align(app.app()), Some(TextAlign::Left));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_shape_color_turns_a_transparent_fill_back_to_solid() {
    let mut app = TestApp::with_entities([shape("a", A)]);
    app.select(&["a"]);
    set(&mut app, Property::FillStyle(FillStyle::None));
    assert_eq!(property::read::fill_style(app.app()), Some(FillStyle::None));
    set(&mut app, Property::Color(RED));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":100,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":"","color":"1","specular":{"fillStyle":"solid"}}
    edges:
    specular: {"entityOrder":["a"]}
    "#);
    assert_eq!(
        property::read::fill_style(app.app()),
        Some(FillStyle::Solid)
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_shape_border_has_a_style_a_width_and_a_color() {
    let mut app = TestApp::with_entities([shape("a", A)]);
    app.select(&["a"]);
    set(&mut app, Property::BorderStyle(BorderStyle::Dashed));
    set(&mut app, Property::StrokeWidth(4.0));
    set(&mut app, Property::BorderColor(RED));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":100,"y":100,"width":200,"height":100,"shapeKind":"rectangle","text":"","strokeWidth":4,"borderStyle":"dashed","borderColor":"1"}
    edges:
    specular: {"entityOrder":["a"]}
    "#);
    assert_eq!(property::read::border_color(app.app()), Some(RED));
    assert_eq!(property::read::stroke_width(app.app()), Some(4.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn reads_say_mixed_when_the_selection_disagrees() {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B)]);
    app.select(&["a"]);
    set(&mut app, Property::ShapeKind(ShapeKind::Pill));
    app.select(&["a", "b"]);
    assert_eq!(property::read::shape_kind(app.app()), None);
    assert_eq!(
        property::read::fill_style(app.app()),
        Some(FillStyle::Solid)
    );
    assert_eq!(
        property::read::text_font(app.app()),
        None,
        "no text selected"
    );
}

// Groups and drawings.

#[test]
fn a_group_takes_a_color_and_not_its_members() {
    let doc = document([
        group("g", Rect::new(0.0, 0.0, 400.0, 300.0)),
        specular_testkit::inside("g", sticky("a", A, "one")),
    ]);
    let mut app = TestApp::from_document(doc);
    app.select(&["g"]);
    set(&mut app, Property::Color(RED));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"g","type":"group","x":0,"y":0,"width":400,"height":300,"color":"1","groupColor":"1"}
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"one","specular":{"textStyle":"sticky","parentGroupId":"g"}}
    edges:
    specular: {"entityOrder":["g","a"]}
    "#);
    app.assert_undo_returns_to_start();
}

// A gesture in flight.

#[test]
fn a_property_waits_while_a_drag_is_in_flight() {
    let mut app = TestApp::with_entities([sticky("a", A, "one")]);
    app.select(&["a"])
        .press((150.0, 150.0))
        .drag_to((200.0, 200.0));
    set(&mut app, Property::Color(RED));
    app.release();
    assert_eq!(property::read::color(app.app()), None);
    app.assert_undo_returns_to_start();
}

// A text edit in progress.

fn color_of(app: &TestApp, id: &str) -> Option<Color> {
    match &app.entity(id).kind {
        Kind::Text(text) => text.color.clone(),
        _ => None,
    }
}

#[test]
fn a_property_set_during_an_edit_survives_the_edits_end() {
    let mut app = TestApp::with_entities([sticky("n", A, "held")]);
    app.double_click((150.0, 150.0)).type_text("new");
    set(&mut app, Property::TextSize(24.0));
    set(&mut app, Property::Color(RED));
    assert_eq!(app.editing_text(), "new", "the edit carries on");
    app.key(Key::Escape);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"n","type":"text","x":100,"y":100,"width":200,"height":343,"text":"new","color":"1","specular":{"textStyle":"sticky","textSize":24}}
    edges:
    specular: {"entityOrder":["n"]}
    "#);
    assert_eq!(color_of(&app, "n"), Some(RED));
    app.undo();
    assert_eq!(color_of(&app, "n"), Some(RED), "the text was its own step");
    app.undo().undo();
    app.redo().redo().redo().assert_undo_returns_to_start();
}

#[test]
fn a_property_set_while_a_new_text_is_typed_belongs_to_its_one_step() {
    let mut app = TestApp::empty();
    app.tool(Tool::AddSticky).click((100.0, 100.0));
    set(&mut app, Property::Color(RED));
    assert!(!steps(&app), "the placement is not a step yet");
    app.type_text("note").key(Key::Escape);
    let id = app.selected().map(str::to_owned).unwrap_or_default();
    assert_eq!(color_of(&app, &id), Some(RED));
    app.undo();
    assert_eq!(app.document().entities().count(), 0);
    app.redo();
    assert_eq!(color_of(&app, &id), Some(RED));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_size_picked_during_an_edit_refits_the_text_being_typed() {
    let mut app = TestApp::with_entities([sticky("n", A, "held")]);
    app.double_click((150.0, 150.0));
    let before = app.rect("n");
    set(&mut app, Property::TextSize(32.0));
    assert!(app.rect("n").height > before.height);
    app.key(Key::Escape);
    app.assert_undo_returns_to_start();
}
