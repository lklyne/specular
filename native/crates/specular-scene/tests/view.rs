//! Scene snapshots: what `view` draws for each kind and for the session
//! layer over them. Read a `.snap` before accepting a change to it.

use glam::Vec2;
use specular_doc::{
    Annotation, AnnotationAnchor, AnnotationId, AnnotationStatus, Author, BorderStyle, BrushType,
    Color, ColorPreset, Command, Drawing, Edge, EdgeEnd, Entity, EntityId, JsonMap, Kind,
    LineStyle, Point, Rect, RegionAnchor, Shape, ShapeKind, Stroke, Text, TextAlign, TextFont,
    TextStyle, WidthMode,
};
use specular_interact::{Action, Key};
use specular_scene::{Draw, view, view_without_chrome};
use specular_testkit::{
    TestApp, assert_scene_snapshot, connected, document, file, group, inside, page, shape, text,
};

const VIEWPORT: Vec2 = Vec2::new(1600.0, 1000.0);

fn text_entity(id: &str, rect: Rect, fields: Text) -> Entity {
    Entity::new(id, rect, Kind::Text(fields))
}

fn stroke(id: &str, brush: Option<BrushType>, points: &[(f64, f64)]) -> Stroke {
    Stroke {
        id: id.to_owned(),
        color: Color::Preset(ColorPreset::Green),
        width: 4.0,
        points: points.iter().map(|&(x, y)| Point::new(x, y)).collect(),
        brush,
        extra: JsonMap::new(),
    }
}

fn annotation(id: &str, anchor: AnnotationAnchor, status: AnnotationStatus) -> Annotation {
    Annotation {
        id: AnnotationId::new(id),
        anchor,
        author: Author::User,
        text: "look here".to_owned(),
        status,
        replies: Vec::new(),
        created_at: "2026-01-01T00:00:00.000Z".to_owned(),
        element_name: None,
        page_anchor: None,
        metadata: None,
        extra: JsonMap::new(),
    }
}

fn select(app: &mut TestApp, ids: &[&str]) {
    let items = ids
        .iter()
        .map(|id| EntityId::from(*id).into())
        .collect::<Vec<_>>();
    app.act(Action::Select(items));
}

#[test]
fn a_page_is_its_frame_with_a_border_and_a_title() {
    let mut named = page("p2", Rect::new(700.0, 100.0, 400.0, 300.0));
    named.label = Some("Checkout".to_owned());
    let app = TestApp::with_entities([page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)), named]);
    assert_scene_snapshot!(app);
}

#[test]
fn plain_text_is_ink_in_its_font_size_and_colour() {
    let app = TestApp::with_entities([
        text_entity(
            "auto",
            Rect::new(100.0, 100.0, 200.0, 21.0),
            Text {
                text: "Unwrapped heading".to_owned(),
                style: Some(TextStyle::Plain),
                size: Some(32.0),
                ..Text::default()
            },
        ),
        text_entity(
            "fixed",
            Rect::new(100.0, 200.0, 200.0, 80.0),
            Text {
                text: "Wraps at its width".to_owned(),
                style: Some(TextStyle::Plain),
                width_mode: Some(WidthMode::Fixed),
                color: Some(Color::Preset(ColorPreset::Red)),
                font: Some(TextFont::Mono),
                ..Text::default()
            },
        ),
        text_entity(
            "hand",
            Rect::new(100.0, 300.0, 200.0, 21.0),
            Text {
                text: "By hand".to_owned(),
                style: Some(TextStyle::Plain),
                font: Some(TextFont::Hand),
                color: Some(Color::parse("#336699")),
                ..Text::default()
            },
        ),
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_sticky_note_is_a_card_with_clipped_wrapped_text() {
    let app = TestApp::with_entities([
        // The testkit's text is a sticky with no colour: yellow.
        text("s1", Rect::new(100.0, 100.0, 200.0, 200.0)),
        text_entity(
            "s2",
            Rect::new(400.0, 100.0, 200.0, 200.0),
            Text {
                text: "Purple\nnote".to_owned(),
                color: Some(Color::Preset(ColorPreset::Purple)),
                size: Some(56.0),
                ..Text::default()
            },
        ),
        text_entity(
            "empty",
            Rect::new(700.0, 100.0, 200.0, 200.0),
            Text::default(),
        ),
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn every_shape_kind_draws_its_silhouette() {
    let kinds = [
        ShapeKind::Rectangle,
        ShapeKind::Rounded,
        ShapeKind::Ellipse,
        ShapeKind::Diamond,
        ShapeKind::Triangle,
        ShapeKind::Hexagon,
        ShapeKind::Pill,
        ShapeKind::Parallelogram,
        ShapeKind::Chevron,
        ShapeKind::Cylinder,
    ];
    let shapes = kinds.into_iter().enumerate().map(|(index, kind)| {
        let rect = Rect::new(100.0 + 140.0 * index as f64, 100.0, 120.0, 80.0);
        Entity::new(format!("{kind:?}"), rect, Kind::Shape(Shape::new(kind)))
    });
    assert_scene_snapshot!(TestApp::with_entities(shapes));
}

#[test]
fn a_shape_takes_its_colour_border_and_label() {
    let styled = |id: &str, x: f64, kind: ShapeKind, edit: fn(&mut Shape)| {
        let mut fields = Shape::new(kind);
        edit(&mut fields);
        Entity::new(id, Rect::new(x, 100.0, 200.0, 100.0), Kind::Shape(fields))
    };
    let app = TestApp::with_entities([
        styled("labelled", 100.0, ShapeKind::Diamond, |shape| {
            shape.text = "Decide".to_owned();
            shape.color = Some(Color::Preset(ColorPreset::Red));
        }),
        styled("dashed", 400.0, ShapeKind::Rounded, |shape| {
            shape.border_style = Some(BorderStyle::Dashed);
            shape.border_color = Some(Color::Preset(ColorPreset::Cyan));
            shape.stroke_width = Some(4.0);
            shape.text = "Left".to_owned();
            shape.text_align = Some(TextAlign::Left);
            shape.text_size = Some(20.0);
        }),
        styled("hollow", 700.0, ShapeKind::Ellipse, |shape| {
            shape.fill_style = Some(specular_doc::FillStyle::None);
        }),
        styled("bare", 1000.0, ShapeKind::Cylinder, |shape| {
            shape.border_style = Some(BorderStyle::None);
        }),
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_drawing_is_the_outline_of_each_stroke() {
    let strokes = vec![
        stroke(
            "pen",
            None,
            &[
                (100.0, 100.0),
                (140.0, 100.0),
                (180.0, 110.0),
                (220.0, 140.0),
            ],
        ),
        stroke(
            "marker",
            Some(BrushType::Highlight),
            &[(100.0, 200.0), (160.0, 205.0), (220.0, 200.0)],
        ),
        stroke("nothing", None, &[]),
    ];
    let drawing = Entity::new(
        "d1",
        Rect::new(96.0, 96.0, 128.0, 112.0),
        Kind::Drawing(Drawing { strokes }),
    );
    assert_scene_snapshot!(TestApp::with_entities([drawing]));
}

#[test]
fn a_group_is_a_tinted_frame_with_its_title_above() {
    let mut tinted = group("g2", Rect::new(700.0, 100.0, 400.0, 300.0));
    tinted.label = Some("Flows".to_owned());
    if let Kind::Group(fields) = &mut tinted.kind {
        fields.color = Some(Color::Preset(ColorPreset::Green));
    }
    let app = TestApp::with_entities([
        group("g1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        inside("g1", shape("member", Rect::new(150.0, 150.0, 100.0, 60.0))),
        tinted,
    ]);
    assert_scene_snapshot!(app);
}

#[test]
fn an_edge_is_a_curve_with_arrowheads_and_a_label() {
    let boxes = document([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(500.0, 150.0, 100.0, 100.0)),
        shape("c", Rect::new(100.0, 500.0, 100.0, 100.0)),
    ]);
    let mut document = connected(boxes, "plain", "a", "b");
    let styled = Edge {
        from_end: Some(EdgeEnd::Arrow),
        to_end: Some(EdgeEnd::None),
        color: Some(Color::Preset(ColorPreset::Purple)),
        label: Some("then".to_owned()),
        stroke_width: Some(3.0),
        line_style: Some(LineStyle::Dashed),
        ..Edge::new("styled", "a", "c")
    };
    let at = document.stack_len();
    document
        .apply(Command::InsertEdge {
            edge: Box::new(styled),
            at,
        })
        .unwrap();
    let dangling = connected(document, "dangling", "a", "missing");
    assert_scene_snapshot!(TestApp::from_document(dangling));
}

#[test]
fn a_file_is_a_card_with_its_name() {
    let app = TestApp::with_entities([file("f1", Rect::new(100.0, 100.0, 240.0, 160.0))]);
    assert_scene_snapshot!(app);
}

#[test]
fn selecting_one_entity_outlines_it_and_adds_corner_handles() {
    let mut app = TestApp::with_pages(2);
    select(&mut app, &["p1"]);
    assert_scene_snapshot!(app);
}

#[test]
fn selecting_several_outlines_each_and_puts_handles_on_their_bounds() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 250.0, 100.0, 100.0)),
    ]);
    select(&mut app, &["a", "b"]);
    assert_scene_snapshot!(app);
}

#[test]
fn a_marquee_draws_its_rect_and_outlines_what_it_would_take() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(600.0, 100.0, 100.0, 100.0)),
    ]);
    app.press((50.0, 50.0)).drag_to((250.0, 250.0));
    assert_scene_snapshot!(app);
}

#[test]
fn the_entered_page_gets_a_hover_border() {
    let mut app = TestApp::with_pages(1);
    app.click((200.0, 200.0))
        .click((200.0, 200.0))
        .pointer_move((210.0, 210.0));
    select(&mut app, &[]);
    assert_scene_snapshot!(app);
}

#[test]
fn the_comment_tool_previews_its_region() {
    let mut app = TestApp::with_pages(1);
    app.key(Key::Char('c'))
        .press((600.0, 500.0))
        .drag_to((700.0, 580.0));
    assert_scene_snapshot!(app);
}

#[test]
fn open_comments_show_as_regions_badges_and_dots() {
    let mut document = document([page("p1", Rect::new(100.0, 100.0, 400.0, 300.0))]);
    let page_id = || EntityId::from("p1");
    let region = AnnotationAnchor::Region(RegionAnchor::Canvas {
        canvas_rect: Rect::new(600.0, 100.0, 200.0, 100.0),
    });
    let mut thread = annotation(
        "on-page",
        AnnotationAnchor::Page {
            page_id: page_id(),
            offset_x: 0.5,
            offset_y: 0.5,
        },
        AnnotationStatus::Acknowledged,
    );
    thread.replies = (0..11)
        .map(|index| specular_doc::Reply {
            author: Author::Agent,
            text: format!("reply {index}"),
            timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
            extra: JsonMap::new(),
        })
        .collect();
    let annotations = [
        annotation("region", region.clone(), AnnotationStatus::Pending),
        annotation("done", region, AnnotationStatus::Resolved),
        thread,
        annotation(
            "on-element",
            AnnotationAnchor::Element {
                page_id: page_id(),
                selector: "#buy".to_owned(),
                element_path: None,
                bounding_box: None,
            },
            AnnotationStatus::Pending,
        ),
        annotation(
            "on-canvas",
            AnnotationAnchor::Canvas {
                canvas_x: 900.0,
                canvas_y: 400.0,
            },
            AnnotationStatus::Pending,
        ),
    ];
    for (at, annotation) in annotations.into_iter().enumerate() {
        document
            .apply(Command::InsertAnnotation {
                annotation: Box::new(annotation),
                at,
            })
            .unwrap();
    }
    assert_scene_snapshot!(TestApp::from_document(document));
}

#[test]
fn chrome_keeps_its_pixel_size_when_zoomed_out() {
    let mut app = TestApp::with_pages(1);
    select(&mut app, &["p1"]);
    app.zoom(0.5);
    assert_scene_snapshot!(app);
}

#[test]
fn without_chrome_only_the_content_is_drawn() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        text("s1", Rect::new(600.0, 100.0, 200.0, 200.0)),
    ]);
    select(&mut app, &["p1"]);
    let scene = view_without_chrome(app.app(), VIEWPORT);
    let kinds: Vec<&str> = scene
        .items
        .iter()
        .map(|item| match item.draw {
            Draw::Page(_) => "page",
            Draw::Shadow(_) => "shadow",
            Draw::Rect(_) => "rect",
            Draw::Text(_) => "text",
            Draw::Ellipse(_)
            | Draw::Polygon(_)
            | Draw::Path(_)
            | Draw::Column(_)
            | Draw::Image(_) => "other",
        })
        .collect();
    assert_eq!(kinds, ["page", "shadow", "rect", "text"]);
}

#[test]
fn entities_outside_the_viewport_are_left_out() {
    let app = TestApp::with_entities([
        text("near", Rect::new(100.0, 100.0, 200.0, 200.0)),
        text("far", Rect::new(9000.0, 100.0, 200.0, 200.0)),
    ]);
    // A sticky is a shadow, a card and its text.
    assert_eq!(view(app.app(), VIEWPORT).items.len(), 3);
}
