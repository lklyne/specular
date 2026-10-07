//! The menu model: its shortcuts come from the binding table, and its
//! enabled and checked states follow the app.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{BrushType, ShapeKind};
use specular_interact::{
    Action, BINDINGS, Chord, Key, Menu, MenuEntry, MenuItem, Tool, ToolDefaultPatch, binding_of,
    menus,
};
use specular_testkit::{CMD, TestApp};

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

fn labels(menu: &Menu) -> Vec<&'static str> {
    (menu.entries.iter())
        .map(|entry| match entry {
            MenuEntry::Item(item) => item.label,
            MenuEntry::Separator => "-",
        })
        .collect()
}

#[test]
fn the_menus_are_edit_tools_and_view() {
    let app = TestApp::with_pages(1);
    let menus = menus(app.app());
    let [edit, tools, view] = menus.as_slice() else {
        panic!("three menus");
    };
    assert_eq!(
        (edit.title, labels(edit)),
        (
            "Edit",
            vec![
                "Undo",
                "Redo",
                "-",
                "Cut",
                "Copy",
                "Paste",
                "Duplicate",
                "Delete",
                "-",
                "Select all"
            ]
        )
    );
    assert_eq!(
        (tools.title, labels(tools)),
        (
            "Tools",
            vec![
                "Select",
                "Page",
                "Text",
                "Sticky note",
                "Document",
                "Shape",
                "Draw",
                "Comment"
            ]
        )
    );
    assert_eq!(
        (view.title, labels(view)),
        (
            "View",
            vec!["Zoom in", "Zoom out", "Zoom to 100%", "Zoom to fit"]
        )
    );
}

#[test]
fn every_shortcut_is_the_binding_tables_chord_for_the_same_action() {
    let app = TestApp::with_pages(1);
    for item in items(&menus(app.app())) {
        let bound = binding_of(&item.action).map(|binding| binding.chord);
        assert_eq!(item.chord, bound, "{}", item.label);
        // No two rows give one action different contexts under one chord,
        // so the menu and the key agree on where it works.
        if let Some(chord) = item.chord {
            let rows = BINDINGS.iter().filter(|binding| binding.chord == chord);
            assert_eq!(rows.count(), 1, "{}", item.label);
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
    assert_eq!(chord("Zoom in"), Some(Chord::char('=').cmd()));
    assert_eq!(chord("Zoom out"), Some(Chord::char('-').cmd()));
    assert_eq!(chord("Zoom to 100%"), Some(Chord::char('0').cmd()));
    assert_eq!(chord("Zoom to fit"), Some(Chord::char('1').cmd()));
    assert_eq!(chord("Select"), Some(Chord::char('v')));
    assert_eq!(chord("Shape"), Some(Chord::char('r')));
    assert_eq!(chord("Draw"), Some(Chord::char('m')));
    assert_eq!(chord("Document"), None);
}

#[test]
fn a_tool_item_runs_what_its_key_runs() {
    let app = TestApp::with_pages(1);
    let menus = menus(app.app());
    assert_eq!(
        item(&menus, "Shape").action,
        Action::SetToolVariant(ToolDefaultPatch::ShapeKind(ShapeKind::Rectangle))
    );
    assert_eq!(
        item(&menus, "Draw").action,
        Action::SetToolVariant(ToolDefaultPatch::Brush(BrushType::Pen))
    );
    assert_eq!(item(&menus, "Text").action, Action::SetTool(Tool::AddText));
    assert_eq!(
        item(&menus, "Document").action,
        Action::SetTool(Tool::AddDocument)
    );
}

#[test]
fn the_active_tool_is_the_one_checked() {
    let mut app = TestApp::with_pages(1);
    let checked = |app: &TestApp| -> Vec<&'static str> {
        (items(&menus(app.app())).into_iter())
            .filter(|item| item.checked == Some(true))
            .map(|item| item.label)
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
    for label in ["Undo", "Redo", "Cut", "Copy", "Duplicate", "Delete"] {
        assert!(!enabled(&app, label), "{label}");
    }
    for label in ["Paste", "Select all", "Zoom in", "Zoom to fit", "Shape"] {
        assert!(enabled(&app, label), "{label}");
    }
    app.select(&["p1"]).act(Action::Duplicate);
    for label in ["Undo", "Cut", "Copy", "Duplicate", "Delete"] {
        assert!(enabled(&app, label), "{label}");
    }
    app.undo();
    assert!(enabled(&app, "Redo"));
    assert!(!enabled(&app, "Undo"));
}

#[test]
fn an_empty_canvas_has_nothing_to_select_or_fit() {
    let app = TestApp::empty();
    let menus = menus(app.app());
    assert!(!item(&menus, "Select all").enabled);
    assert!(!item(&menus, "Zoom to fit").enabled);
    assert!(item(&menus, "Zoom to 100%").enabled);
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

#[test]
fn nothing_can_be_chosen_mid_drag() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"])
        .press((200.0, 150.0))
        .drag_to((300.0, 250.0));
    let menus = menus(app.app());
    assert!(items(&menus).iter().all(|item| !item.enabled));
    app.release();
}

#[test]
fn a_chosen_item_does_what_the_key_does() {
    let mut by_menu = TestApp::with_pages(2);
    let mut by_key = TestApp::with_pages(2);
    let select_all = item(&menus(by_menu.app()), "Select all").action.clone();
    by_menu.act(select_all);
    by_key.chord(CMD, Key::Char('a'));
    assert_eq!(by_menu.selected_ids(), ["p1", "p2"]);
    assert_eq!(by_menu.selected_ids(), by_key.selected_ids());
}
