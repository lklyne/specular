//! A text's size is its content's only when the measure is the renderer's
//! own: an estimate leaves the sizes the file had. With the renderer's
//! measure, see `fixtures/scenarios/text-fit.txt`.

use specular_doc::{Entity, Kind, Rect, Text, TextStyle, WidthMode};
use specular_testkit::{TestApp, document, labelled, plain_text, sticky};

/// Fixed-width plain text, which wraps 8 short of its width.
fn wrapping(id: &str, rect: Rect, content: &str) -> Entity {
    let text = Text {
        text: content.to_owned(),
        style: Some(TextStyle::Plain),
        width_mode: Some(WidthMode::Fixed),
        ..Text::default()
    };
    Entity::new(id, rect, Kind::Text(text))
}

fn entities() -> [Entity; 4] {
    [
        // Two words a line at 112 wide: three lines.
        wrapping(
            "wrapped",
            Rect::new(0.0, 0.0, 120.0, 500.0),
            "aaaa bbbb cccc dddd eeee",
        ),
        // Seventeen characters of 10 and the 8 kept clear for the caret, on one line.
        plain_text(
            "hugging",
            Rect::new(0.0, 600.0, 300.0, 300.0),
            "hello there world",
        ),
        // Fifteen lines of 20 and 8 of padding above and below.
        sticky(
            "note",
            Rect::new(400.0, 0.0, 200.0, 50.0),
            &"x\n".repeat(14),
        ),
        labelled("shape", Rect::new(400.0, 600.0, 200.0, 100.0), "a label"),
    ]
}

#[test]
fn an_estimating_measure_leaves_the_sizes_the_file_had() {
    let mut app = TestApp::empty();
    app.open(document(entities()));
    assert_eq!(
        [app.rect("wrapped"), app.rect("hugging"), app.rect("note")],
        [
            Rect::new(0.0, 0.0, 120.0, 500.0),
            Rect::new(0.0, 600.0, 300.0, 300.0),
            Rect::new(400.0, 0.0, 200.0, 50.0),
        ]
    );
}
