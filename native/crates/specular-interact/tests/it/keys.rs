//! `keys.json` (ADR 0059): the person's keys come before the table's, a
//! `null` takes a table key away, a `layout:` section fires only while its
//! layout is in force, the menus and the `keys` key show what is bound,
//! and a file that does not read says where.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use specular_interact::{Chord, Event, Key, Keymap, MenuEntry, Tool, menus, state};
use specular_testkit::TestApp;

const KEYS: &str = r#"{
  "canvas": { "s": "tool.set:{\"tool\":\"shape\"}", "r": null, "n": "tool.set:{\"tool\":\"sticky\"}", "cmd+shift+k": "layout.switch" },
  "layout:classic": { "s": { "command": "tool.set", "args": { "tool": "draw" } } }
}"#;

fn with_keys() -> TestApp {
    let mut app = TestApp::empty();
    app.send(Event::KeymapLoaded(Keymap::parse(KEYS).expect("reads")));
    app
}

#[test]
fn the_persons_keys_come_first_and_null_takes_a_key_away() {
    let mut app = with_keys();
    app.key(Key::Char('s'));
    assert_eq!(app.session().tool, Tool::AddShape, "s is shape, not sticky");
    app.key(Key::Escape).key(Key::Char('r'));
    assert_eq!(app.session().tool, Tool::Select, "r does nothing now");

    let keys = &state(app.app())["keys"];
    assert_eq!(keys[0]["from"], "keys.json");
    let switch = (keys.as_array().expect("a list").iter())
        .find(|row| row["command"] == "layout.switch")
        .expect("listed");
    assert_eq!(switch["keys"], "⇧⌘K");
    let tools = menus(app.app())
        .into_iter()
        .find(|menu| menu.title == "Tools");
    let chords: Vec<_> = (tools.expect("a Tools menu").entries.into_iter())
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some((item.label.into_owned(), item.chord.map(Chord::text))),
            MenuEntry::Separator => None,
        })
        .filter(|(label, _)| label == "Shape" || label == "Sticky note")
        .collect();
    assert_eq!(
        chords,
        [
            ("Sticky note".to_owned(), Some("N".to_owned())),
            ("Shape".to_owned(), None)
        ],
        "the menus show the person's key, and none for one taken away"
    );

    app.key(Key::Escape);
    app.send(Event::Action(specular_interact::Action::Layout(
        specular_interact::LayoutAction::Switch(Some("classic".to_owned())),
    )));
    app.key(Key::Char('s'));
    assert_eq!(app.session().tool, Tool::Draw, "the layout's own s");
}

#[test]
fn a_file_that_does_not_read_says_where() {
    for (text, says) in [
        (
            r#"{"canvas":{"cmd+":"edit.undo"}}"#,
            "canvas \"cmd+\": not a key",
        ),
        (r#"{"canvas":{"x":"no.such"}}"#, "no command `no.such`"),
        (r#"{"elsewhere":{}}"#, "`elsewhere` is not"),
        (
            r#"{"canvas":{"x":"tool.set:{\"tool\":\"nope\"}"}}"#,
            "canvas \"x\": tool.set",
        ),
    ] {
        let error = Keymap::parse(text).expect_err(text);
        assert!(error.contains(says), "{text}: {error}");
    }
}

#[test]
fn control_shift_l_moves_to_the_next_layout_and_the_view_menu_shows_it() {
    let mut app = TestApp::empty();
    let start = app.app().layout_name().to_owned();
    let held = specular_core::Modifiers {
        control: true,
        shift: true,
        ..Default::default()
    };
    app.chord(held, Key::Char('l'));
    let next = app.app().layout_name().to_owned();
    assert_ne!(next, start, "Control+Shift+L switched layout");
    app.chord(held, Key::Char('l'));
    assert_ne!(app.app().layout_name(), next, "and again");

    let view = menus(app.app())
        .into_iter()
        .find(|menu| menu.title == "View");
    let chord = (view.expect("a View menu").entries.into_iter())
        .find_map(|entry| match entry {
            MenuEntry::Item(item) if item.label == "Next Layout" => {
                Some(item.chord.map(Chord::text))
            }
            _ => None,
        })
        .expect("listed");
    assert_eq!(chord.as_deref(), Some("⇧⌘L"));
}
