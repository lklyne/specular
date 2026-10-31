//! Scene snapshots: what `view` draws for each kind and for the session
//! layer over them. Read a `.snap` before accepting a change to it.

use specular_doc::{
    Color, ColorPreset, Command, Edge, EdgeEnd, Entity, EntityId, Kind, LineStyle, Rect, Shape,
    ShapeKind, Text,
};
use specular_interact::{Action, Event, ImageNotice, Theme};
use specular_testkit::{
    TestApp, assert_scene_snapshot, connected, document, file, page, plain_text, shape, sticky,
    text,
};

fn text_entity(id: &str, rect: Rect, fields: Text) -> Entity {
    Entity::new(id, rect, Kind::Text(fields))
}

fn select(app: &mut TestApp, ids: &[&str]) {
    let items = ids
        .iter()
        .map(|id| EntityId::from(*id).into())
        .collect::<Vec<_>>();
    app.act(Action::Select(items));
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
fn selecting_one_entity_outlines_it_and_adds_corner_handles() {
    let mut app = TestApp::with_pages(2);
    select(&mut app, &["p1"]);
    assert_scene_snapshot!(app);
}

#[test]
fn chrome_keeps_its_pixel_size_when_zoomed_out() {
    // p1 has the handles.
    let mut app = TestApp::with_pages(2);
    select(&mut app, &["p1"]);
    app.zoom(0.5);
    assert_scene_snapshot!(app);
}

#[test]
fn the_dark_theme_draws_ink_paper_shapes_and_frames_in_its_own_colours() {
    let mut red = shape("s", Rect::new(500.0, 100.0, 200.0, 100.0));
    if let Kind::Shape(fields) = &mut red.kind {
        fields.color = Some(Color::Preset(ColorPreset::Red));
        fields.text = "Decide".to_owned();
    }
    let mut app = TestApp::with_entities([
        page("p", Rect::new(100.0, 300.0, 400.0, 300.0)),
        plain_text("t", Rect::new(100.0, 100.0, 300.0, 40.0), "ink"),
        sticky("n", Rect::new(750.0, 100.0, 200.0, 200.0), "paper"),
        red,
    ]);
    app.send(Event::ThemeLoaded(Theme::Dark));
    assert_scene_snapshot!(app);
}

/// An image file `id` in a device frame of `style` on the iPad Pro 13.
fn framed_image(id: &str, rect: Rect, style: &str) -> Entity {
    let mut image = file(id, rect);
    if let Kind::File(fields) = &mut image.kind {
        fields.metadata = serde_json::json!({
            "showDeviceFrame": true,
            "deviceId": "ipad-pro-13",
            "deviceFrameStyle": style,
            "deviceFrameColor": "dark",
        })
        .as_object()
        .cloned();
    }
    image
}

/// Answers the load of `file` as a `width` by `height` image.
fn loaded(app: &mut TestApp, file: &str, (width, height): (u32, u32)) {
    let key = app.app().image(file).map(|image| image.key);
    assert!(key.is_some(), "{file} was asked for");
    if let Some(image) = key {
        app.send(Event::Image {
            image,
            notice: ImageNotice::Ready { width, height },
        });
    }
}

#[test]
fn a_framed_screenshot_sits_on_its_devices_screen_in_clay_or_its_drawing() {
    // Half the preset's size, so the shell is drawn at half its bezel.
    let mut app = TestApp::with_entities([
        framed_image("clay", Rect::new(100.0, 100.0, 516.0, 688.0), "clay"),
        framed_image("real", Rect::new(800.0, 100.0, 516.0, 688.0), "realistic"),
    ]);
    loaded(&mut app, "clay.png", (2064, 2752));
    loaded(&mut app, "real.png", (2064, 2752));
    let art =
        specular_doc::DeviceArt::IpadPro13InchesM4.asset(specular_doc::DeviceFrameColor::Dark);
    loaded(&mut app, art, (1200, 1600));
    assert_scene_snapshot!(app);
}
