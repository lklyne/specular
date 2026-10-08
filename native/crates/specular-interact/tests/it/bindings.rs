//! The binding table: which keys do what, and where.

use specular_core::Modifiers;
use specular_doc::{BrushType, Rect, ShapeKind};
use specular_interact::{
    Action, BINDINGS, Context, Effect, Event, Key, KeyInput, Tool, ToolDefaultPatch, ToolDefaults,
};
use specular_testkit::{CMD, CMD_SHIFT, CTRL, SHIFT, TestApp, text};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);

fn note() -> TestApp {
    let mut app = TestApp::with_entities([text("a", A)]);
    app.select(&["a"]);
    app
}

fn auto_repeat(app: &mut TestApp, key: Key) {
    app.send(Event::Key(KeyInput {
        key,
        pressed: true,
        repeat: true,
        text: None,
        character: None,
        modifiers: Modifiers::default(),
        windows_key_code: 0,
        native_key_code: 0,
    }));
}

fn saves(effects: &[Effect]) -> Vec<&ToolDefaults> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::SaveToolDefaults(defaults) => Some(&**defaults),
            _ => None,
        })
        .collect()
}

#[test]
fn no_two_rows_can_fire_for_the_same_key() {
    for (index, binding) in BINDINGS.iter().enumerate() {
        // The bracket keys restack on the canvas and walk the history of an
        // entered page: one key, two rows, never both at once.
        let shadowed = BINDINGS[..index].iter().any(|earlier| {
            earlier.chord == binding.chord && earlier.context.overlaps(binding.context)
        });
        assert!(!shadowed, "{:?} is bound twice", binding.chord);
    }
}

#[test]
fn only_zoom_to_fit_and_escape_fire_inside_an_entered_page() {
    let everywhere: Vec<_> = BINDINGS
        .iter()
        .filter(|binding| binding.context == Context::Always)
        .map(|binding| &binding.action)
        .collect();
    // Both are ways back out to the canvas. Every other key is the page's.
    assert_eq!(everywhere, [&Action::ZoomToFit, &Action::Cancel]);
}

#[test]
fn each_tool_key_arms_its_tool() {
    for (key, tool) in [
        ('p', Tool::AddPage),
        ('t', Tool::AddText),
        ('s', Tool::AddSticky),
        ('r', Tool::AddShape),
        ('o', Tool::AddShape),
        ('c', Tool::Comment),
        ('i', Tool::Inspect),
        ('m', Tool::Draw),
        ('v', Tool::Select),
    ] {
        let mut app = TestApp::empty();
        app.tool(Tool::Comment).key(Key::Char(key));
        assert_eq!(app.session().tool, tool, "{key}");
    }
}

#[test]
fn a_variant_key_arms_the_tool_and_writes_the_variant_to_the_defaults() {
    let mut app = TestApp::empty();
    app.key(Key::Char('o'));
    assert_eq!(app.app().tool_defaults().shape.kind, ShapeKind::Ellipse);
    app.chord(SHIFT, Key::Char('r'));
    assert_eq!(app.app().tool_defaults().shape.kind, ShapeKind::Diamond);
    app.key(Key::Char('r'));
    assert_eq!(app.app().tool_defaults().shape.kind, ShapeKind::Rectangle);
    assert_eq!(app.session().tool, Tool::AddShape);

    app.chord(SHIFT, Key::Char('m'));
    assert_eq!(app.app().tool_defaults().draw.brush, BrushType::Highlight);
    app.key(Key::Char('m'));
    assert_eq!(app.app().tool_defaults().draw.brush, BrushType::Pen);
    assert_eq!(app.session().tool, Tool::Draw);
}

#[test]
fn a_changed_default_is_saved_and_an_unchanged_one_is_not() {
    let mut app = TestApp::empty();
    // Rectangle is the default already.
    app.key(Key::Char('r'));
    assert_eq!(saves(app.effects()), [] as [&ToolDefaults; 0]);
    app.key(Key::Char('o'));
    let effects = app.take_effects();
    let saved = saves(&effects);
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0], app.app().tool_defaults());
    assert_eq!(saved[0].shape.kind, ShapeKind::Ellipse);
}

#[test]
fn loaded_defaults_replace_the_current_ones_and_are_not_saved_back() {
    let mut loaded = ToolDefaults::default();
    loaded.apply(ToolDefaultPatch::ShapeKind(ShapeKind::Hexagon));
    let mut app = TestApp::empty();
    app.send(Event::ToolDefaultsLoaded(Box::new(loaded.clone())));
    assert_eq!(app.app().tool_defaults(), &loaded);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn control_stands_in_for_command() {
    let mut app = note();
    app.key(Key::ArrowRight).chord(CTRL, Key::Char('z'));
    assert_eq!(app.rect("a"), A);
    app.chord(CMD_SHIFT, Key::Char('z'));
    assert_eq!(app.rect("a"), A.translated(5.0, 0.0));
    app.chord(CMD, Key::Char('z'));
    assert_eq!(app.rect("a"), A);
}

#[test]
fn a_held_arrow_keeps_nudging_and_a_held_backspace_deletes_once() {
    let mut app = note();
    app.key_down(Key::ArrowDown);
    auto_repeat(&mut app, Key::ArrowDown);
    auto_repeat(&mut app, Key::ArrowDown);
    assert_eq!(app.rect("a"), A.translated(0.0, 15.0));

    app.key(Key::Backspace);
    assert_eq!(app.document().entities().count(), 0);
    app.undo();
    auto_repeat(&mut app, Key::Backspace);
    assert_eq!(app.document().entities().count(), 1);
}
