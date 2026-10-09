//! The menu model: its shortcuts come from the binding table, and its
//! enabled and checked states follow the app.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{ItemId, Rect};
use specular_interact::{
    Action, BINDINGS, Chord, Key, Menu, MenuEntry, MenuItem, binding_of, menus,
};
use specular_testkit::{TestApp, connected, document, text};

fn items(menus: &[Menu]) -> Vec<&MenuItem> {
    (menus.iter())
        .flat_map(|menu| &menu.entries)
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some(item),
            MenuEntry::Separator => None,
        })
        .collect()
}

fn item<'a>(menus: &'a [Menu], label: &str) -> &'a MenuItem {
    (items(menus).into_iter())
        .find(|item| item.label == label)
        .unwrap_or_else(|| panic!("no menu item {label:?}"))
}

#[test]
fn every_shortcut_is_the_binding_tables_chord_for_the_same_action() {
    let app = TestApp::with_pages(1);
    for item in items(&menus(app.app())) {
        let bound = binding_of(&item.action).map(|binding| binding.chord);
        // Back and Forward show no key: their chord restacks a selected page.
        if matches!(item.action, Action::PageBack | Action::PageForward) {
            assert_eq!(item.chord, None, "{}", item.label);
            continue;
        }
        assert_eq!(item.chord, bound, "{}", item.label);
        // No two rows give one action different contexts under one chord,
        // so the menu and the key agree on where it works.
        // Rows may share a chord only where their contexts cannot both hold.
        if let Some(chord) = item.chord {
            let own = binding_of(&item.action).map(|binding| binding.context);
            let rivals = BINDINGS.iter().filter(|binding| {
                binding.chord == chord && own.is_some_and(|own| own.overlaps(binding.context))
            });
            assert_eq!(rivals.count(), 1, "{}", item.label);
        }
    }
}

#[test]
fn the_shortcuts_are_the_expected_keys() {
    let app = TestApp::with_pages(1);
    let menus = menus(app.app());
    let chord = |label: &str| item(&menus, label).chord;
    assert_eq!(chord("Undo"), Some(Chord::char('z').cmd()));
    assert_eq!(chord("Redo"), Some(Chord::char('z').cmd().shift()));
    assert_eq!(chord("Cut"), Some(Chord::char('x').cmd()));
    assert_eq!(chord("Copy"), Some(Chord::char('c').cmd()));
    assert_eq!(chord("Paste"), Some(Chord::char('v').cmd()));
    assert_eq!(chord("Duplicate"), Some(Chord::char('d').cmd()));
    assert_eq!(chord("Delete"), Some(Chord::key(Key::Backspace)));
    assert_eq!(chord("Select all"), Some(Chord::char('a').cmd()));
    assert_eq!(chord("Bring forward"), Some(Chord::char(']').cmd()));
    assert_eq!(chord("Send backward"), Some(Chord::char('[').cmd()));
    assert_eq!(
        chord("Bring to front"),
        Some(Chord::char(']').cmd().shift())
    );
    assert_eq!(chord("Send to back"), Some(Chord::char('[').cmd().shift()));
    assert_eq!(chord("Zoom in"), Some(Chord::char('=').cmd()));
    assert_eq!(chord("Zoom out"), Some(Chord::char('-').cmd()));
    assert_eq!(chord("Zoom to 100%"), Some(Chord::char('0').cmd()));
    assert_eq!(chord("Zoom to fit"), Some(Chord::char('1').cmd()));
    assert_eq!(chord("Toggle sidebar"), Some(Chord::char('b').cmd()));
    assert_eq!(chord("Select"), Some(Chord::char('v')));
    assert_eq!(chord("Shape"), Some(Chord::char('r')));
    assert_eq!(chord("Draw"), Some(Chord::char('m')));
    assert_eq!(chord("Document"), None);
}

#[test]
fn the_active_tool_is_the_one_checked() {
    let mut app = TestApp::with_pages(1);
    let checked = |app: &TestApp| -> Vec<String> {
        (menus(app.app()).iter())
            .filter(|menu| menu.title == "Tools")
            .flat_map(|menu| items(std::slice::from_ref(menu)))
            .filter(|item| item.checked == Some(true))
            .map(|item| item.label.to_string())
            .collect()
    };
    assert_eq!(checked(&app), ["Select"]);
    let shape = item(&menus(app.app()), "Shape").action.clone();
    app.act(shape);
    assert_eq!(checked(&app), ["Shape"]);
    assert_eq!(item(&menus(app.app()), "Undo").checked, None);
}

#[test]
fn items_with_nothing_to_act_on_are_disabled() {
    let mut app = TestApp::with_pages(2);
    let enabled = |app: &TestApp, label: &str| item(&menus(app.app()), label).enabled;
    let arrange = [
        "Bring forward",
        "Send backward",
        "Bring to front",
        "Send to back",
    ];
    for label in ["Undo", "Redo", "Cut", "Copy", "Duplicate", "Delete"]
        .into_iter()
        .chain(arrange)
    {
        assert!(!enabled(&app, label), "{label}");
    }
    for label in ["Paste", "Select all", "Zoom in", "Zoom to fit", "Shape"] {
        assert!(enabled(&app, label), "{label}");
    }
    app.select(&["p1"]).act(Action::Duplicate);
    for label in ["Undo", "Cut", "Copy", "Duplicate", "Delete"]
        .into_iter()
        .chain(arrange)
    {
        assert!(enabled(&app, label), "{label}");
    }
    app.undo();
    assert!(enabled(&app, "Redo"));
    assert!(!enabled(&app, "Undo"));

    // Nothing on the canvas to select or fit.
    let empty = TestApp::empty();
    for label in ["Select all", "Zoom to fit"] {
        assert!(!enabled(&empty, label), "{label}");
    }

    // An edge alone is deleted but not cut, copied or duplicated.
    let entities = [
        text("t1", Rect::new(100.0, 100.0, 100.0, 100.0)),
        text("t2", Rect::new(500.0, 100.0, 100.0, 100.0)),
    ];
    let mut edged = TestApp::from_document(connected(document(entities), "e1", "t1", "t2"));
    edged.act(Action::Select(vec![ItemId::Edge("e1".into())]));
    for label in ["Cut", "Copy", "Duplicate"] {
        assert!(!enabled(&edged, label), "{label}");
    }
    assert!(enabled(&edged, "Delete"));
}

#[test]
fn an_entered_page_keeps_the_canvas_items_for_itself() {
    let mut app = TestApp::with_pages(1);
    app.double_click((200.0, 150.0));
    let menus = menus(app.app());
    // Keys go to the page, so the shell must let these chords through.
    for label in [
        "Cut",
        "Copy",
        "Paste",
        "Delete",
        "Select all",
        "Shape",
        "Zoom in",
    ] {
        assert!(!item(&menus, label).enabled, "{label}");
    }
    // Bound everywhere, as in Electron.
    assert!(item(&menus, "Zoom to fit").enabled);
}
