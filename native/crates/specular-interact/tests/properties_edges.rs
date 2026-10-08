//! The drawing and edge properties: brush, stroke width, line style and the
//! arrowheads. Each is one undo step.

use specular_doc::{
    BrushType, Color, ColorPreset, Drawing, Edge, EdgeEnd, Entity, JsonMap, Kind, LineStyle, Point,
    Rect, Stroke,
};
use specular_interact::{Action, Property, property};
use specular_testkit::{
    TestApp, assert_doc_snapshot, connected, document, drawing, text, with_edge,
};

const RED: Color = Color::Preset(ColorPreset::Red);
const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);

fn set(app: &mut TestApp, property: Property) {
    app.act(Action::SetProperty(property));
}

fn steps(app: &TestApp) -> bool {
    app.app().can_undo()
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

#[test]
fn a_brush_moves_the_width_to_one_it_is_offered_in_and_regrows_the_rect() {
    let mut app = TestApp::with_entities([ink(Color::Neutral, 2.0, None)]);
    app.select(&["d"]);
    set(&mut app, Property::Brush(BrushType::Highlight));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"d","type":"drawing","x":116,"y":116,"width":48,"height":38,"strokes":[{"id":"ink","color":"neutral","width":8,"points":[{"x":120,"y":120},{"x":160,"y":150}],"brushType":"highlight"}]}
    edges:
    specular: {"entityOrder":["d"]}
    "#);
    assert_eq!(property::read::brush(app.app()), Some(BrushType::Highlight));
    assert_eq!(property::read::stroke_width(app.app()), Some(8.0));
    set(&mut app, Property::StrokeWidth(16.0));
    assert_eq!(app.rect("d"), Rect::new(112.0, 112.0, 56.0, 46.0));
    app.assert_undo_returns_to_start();
}

// Edges.

fn edged() -> TestApp {
    let doc = document([text("a", A), text("b", B)]);
    TestApp::from_document(connected(
        with_edge(doc, Edge::new("e1", "a", "b")),
        "e2",
        "b",
        "a",
    ))
}

#[test]
fn an_edge_takes_a_color_a_line_and_its_arrowheads() {
    let mut app = edged();
    app.select(&["e1"]);
    set(&mut app, Property::Color(RED));
    set(&mut app, Property::LineStyle(LineStyle::Dashed));
    set(&mut app, Property::StrokeWidth(3.0));
    set(&mut app, Property::FromEnd(EdgeEnd::Arrow));
    set(&mut app, Property::ToEnd(EdgeEnd::None));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
    edges:
      {"id":"e1","fromNode":"a","toNode":"b","fromEnd":"arrow","toEnd":"none","color":"1","strokeWidth":3,"lineStyle":"dashed"}
      {"id":"e2","fromNode":"b","toNode":"a"}
    specular: {"entityOrder":["a","b","e1","e2"]}
    "#);
    assert_eq!(
        property::read::line_style(app.app()),
        Some(LineStyle::Dashed)
    );
    assert_eq!(property::read::from_end(app.app()), Some(EdgeEnd::Arrow));
    assert_eq!(property::read::to_end(app.app()), Some(EdgeEnd::None));
    app.assert_undo_returns_to_start();
}

#[test]
fn every_selected_edge_takes_the_change_in_one_step() {
    let mut app = edged();
    app.select(&["e1", "e2"]);
    set(&mut app, Property::LineStyle(LineStyle::Dashed));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"text","x":100,"y":100,"width":200,"height":100,"text":"a"}
      {"id":"b","type":"text","x":400,"y":100,"width":200,"height":100,"text":"b"}
    edges:
      {"id":"e1","fromNode":"a","toNode":"b","lineStyle":"dashed"}
      {"id":"e2","fromNode":"b","toNode":"a","lineStyle":"dashed"}
    specular: {"entityOrder":["a","b","e1","e2"]}
    "#);
    app.undo();
    assert!(!steps(&app));
    app.redo().assert_undo_returns_to_start();
}
