//! What the polish batch after the QA pass changed in what is drawn, each
//! kept as the test that failed before it.

use specular_doc::{
    BrushType, Color, ColorPreset, Drawing, Entity, JsonMap, Kind, Point, Rect, Stroke,
};
use specular_testkit::{TestApp, file, group, note, page, plain_text, shape, sticky};

const BOX: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

fn scene_of(entity: Entity) -> String {
    TestApp::with_entities([entity]).scene_snapshot()
}

/// The line of the shadow under a card at [`BOX`], if the scene has one.
fn shadow(scene: &str) -> Option<&str> {
    scene
        .lines()
        .find(|line| line.starts_with("canvas shadow 100,102 200x200"))
}

#[test]
fn a_sticky_a_file_card_and_a_document_cast_a_shadow_under_their_card() {
    let mut document = TestApp::with_entities([note("n", BOX, "plan.md")]);
    document.note_text("plan.md", "# Plan");
    let scenes = [
        scene_of(sticky("s", BOX, "note")),
        scene_of(file("f", BOX)),
        document.scene_snapshot(),
    ];
    for scene in scenes {
        let shadow = shadow(&scene);
        assert!(
            shadow.is_some_and(|line| line.ends_with("blur=8 fill=#00000014")),
            "{scene}"
        );
        // It is under the card: the first thing drawn.
        assert_eq!(scene.lines().next(), shadow, "{scene}");
    }
}

#[test]
fn a_shape_and_a_plain_text_cast_none() {
    for scene in [
        scene_of(shape("s", BOX)),
        scene_of(plain_text("t", BOX, "hi")),
    ] {
        assert!(!scene.contains("shadow"), "{scene}");
    }
}

fn stroke(brush: BrushType) -> Entity {
    let stroke = Stroke {
        id: "s1".to_owned(),
        color: Color::Preset(ColorPreset::Yellow),
        width: 8.0,
        points: vec![Point::new(110.0, 150.0), Point::new(290.0, 150.0)],
        brush: Some(brush),
        extra: JsonMap::new(),
    };
    let drawing = Drawing {
        strokes: vec![stroke],
    };
    Entity::new("d", BOX, Kind::Drawing(drawing))
}

#[test]
fn the_highlighter_is_multiplied_in_and_the_pen_is_painted_over() {
    let highlight = scene_of(stroke(BrushType::Highlight));
    let pen = scene_of(stroke(BrushType::Pen));
    assert!(highlight.contains("blend=multiply"), "{highlight}");
    assert!(!pen.contains("blend="), "{pen}");
}

fn title(scene: &str, text: &str) -> String {
    let quoted = format!("screen text {text:?}");
    (scene.lines())
        .find(|line| line.starts_with(&quoted))
        .unwrap_or_default()
        .to_owned()
}

fn titled() -> TestApp {
    let labelled = Entity {
        label: Some("A long title for a small group".to_owned()),
        ..group("g", Rect::new(100.0, 100.0, 120.0, 80.0))
    };
    TestApp::with_entities([labelled, page("p", Rect::new(400.0, 100.0, 400.0, 300.0))])
}

#[test]
fn a_title_is_no_wider_than_what_it_names_and_ends_in_an_ellipsis() {
    let scene = titled().scene_snapshot();
    let group = title(&scene, "A long title for a small group");
    assert!(group.contains(" wrap=120 ellipsis "), "{group}\n{scene}");
    assert!(group.contains(" 11/15.4 "), "{group}");
}

#[test]
fn titles_keep_their_size_to_half_zoom_and_shrink_with_the_canvas_below_it() {
    let mut app = titled();
    let size = |app: &TestApp| {
        let scene = app.scene_snapshot();
        let line = title(&scene, "A long title for a small group");
        // "<size>/<line height>" is the field before the weight.
        let field = line.split(' ').find(|field| field.contains('/'));
        field.map(str::to_owned).unwrap_or_default()
    };
    assert_eq!(size(app.zoom(2.0)), "11/15.4");
    assert_eq!(size(app.zoom(0.5)), "11/15.4");
    assert_eq!(size(app.zoom(0.25)), "5.5/7.7");
    // The page's title follows the same rule.
    let scene = app.scene_snapshot();
    let page = (scene.lines())
        .find(|line| line.starts_with("screen text") && line.contains("wrap=100 "))
        .unwrap_or_default();
    assert!(page.contains(" 5.5/7.7 "), "{scene}");
}
