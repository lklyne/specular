//! A text entity's size is its content's: what a document from disk gets
//! when the measure is the renderer's own, and what it keeps when the
//! measure is only an estimate.

use std::sync::Arc;

use specular_doc::{Entity, Kind, Rect, Text, TextStyle, WidthMode};
use specular_interact::{TextLayout, TextMeasure, TextSpec};
use specular_testkit::{FixedAdvance, TestApp, document, labelled, plain_text, sticky};

/// [`FixedAdvance`] standing in for the renderer's measure.
#[derive(Debug)]
struct Exact;

impl TextMeasure for Exact {
    fn layout(&self, text: &str, spec: &TextSpec) -> TextLayout {
        FixedAdvance::default().layout(text, spec)
    }

    fn is_exact(&self) -> bool {
        true
    }
}

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
        // Five characters and the 8 kept clear for the caret, on one line.
        plain_text("hugging", Rect::new(0.0, 600.0, 300.0, 300.0), "hello"),
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
fn a_loaded_document_gets_measured_text_sizes_with_no_undo_step_or_save() {
    let mut app = TestApp::empty();
    app.measure_with(Arc::new(Exact)).open(document(entities()));
    assert_eq!(
        [
            app.rect("wrapped"),
            app.rect("hugging"),
            app.rect("note"),
            app.rect("shape")
        ],
        [
            Rect::new(0.0, 0.0, 120.0, 60.0),
            Rect::new(0.0, 600.0, 64.0, 20.0),
            Rect::new(400.0, 0.0, 200.0, 316.0),
            Rect::new(400.0, 600.0, 200.0, 100.0),
        ]
    );
    assert!(!app.app().can_undo());
    assert!(
        !app.take_effects()
            .contains(&specular_interact::Effect::Save)
    );
}

#[test]
fn an_estimate_leaves_the_sizes_a_document_came_with() {
    let app = TestApp::with_entities(entities());
    assert_eq!(
        [app.rect("wrapped"), app.rect("note")],
        [
            Rect::new(0.0, 0.0, 120.0, 500.0),
            Rect::new(400.0, 0.0, 200.0, 50.0)
        ]
    );
}
