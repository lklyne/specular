//! Group and ungroup (Cmd+G, Cmd+Shift+G): each is one undo step, the group
//! wraps its members with 24 units of room, and every group's members stay
//! in one run of the stack.

use specular_doc::Rect;
use specular_interact::{Action, Chord, Key, binding_of};
use specular_testkit::{CMD, CMD_SHIFT, TestApp, assert_doc_snapshot, group, inside, shape, text};

const A: Rect = Rect::new(100.0, 100.0, 100.0, 100.0);
const B: Rect = Rect::new(300.0, 100.0, 100.0, 100.0);
const C: Rect = Rect::new(500.0, 100.0, 100.0, 100.0);
const D: Rect = Rect::new(700.0, 100.0, 100.0, 100.0);

fn order(app: &TestApp) -> Vec<&str> {
    (app.document().order().iter())
        .map(specular_doc::ItemId::as_str)
        .collect()
}

fn parent<'a>(app: &'a TestApp, id: &str) -> Option<&'a str> {
    app.entity(id)
        .parent
        .as_ref()
        .map(specular_doc::EntityId::as_str)
}

#[test]
fn the_keys_are_cmd_g_and_cmd_shift_g() {
    assert_eq!(
        binding_of(&Action::Group).map(|binding| binding.chord),
        Some(Chord::char('g').cmd())
    );
    assert_eq!(
        binding_of(&Action::Ungroup).map(|binding| binding.chord),
        Some(Chord::char('g').cmd().shift())
    );
}

#[test]
fn grouping_wraps_the_selection_with_room_and_selects_the_group() {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B), shape("c", C)]);
    app.select(&["a", "b"]).chord(CMD, Key::Char('g'));
    let group = app.selected().expect("the group is selected").to_owned();
    assert_eq!(app.rect(&group), Rect::new(76.0, 76.0, 348.0, 148.0));
    assert_eq!(app.entity(&group).label.as_deref(), Some("Group"));
    assert_eq!(
        (parent(&app, "a"), parent(&app, "b")),
        (Some(&*group), Some(&*group))
    );
    assert_eq!(parent(&app, "c"), None);
    assert_eq!(order(&app), ["a", "b", group.as_str(), "c"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_new_group_is_saved_as_a_freeform_group_its_members_point_at() {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B)]);
    app.select(&["a", "b"]).act(Action::Group);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":100,"y":100,"width":100,"height":100,"shapeKind":"rectangle","text":"","parentGroupId":"e220a8397b1dcdaf"}
      {"id":"b","type":"shape","x":300,"y":100,"width":100,"height":100,"shapeKind":"rectangle","text":"","parentGroupId":"e220a8397b1dcdaf"}
      {"id":"e220a8397b1dcdaf","type":"group","x":76,"y":76,"width":348,"height":148,"label":"Group","layoutMode":"freeform","managedLayout":false}
    edges:
    specular: {"entityOrder":["a","b","e220a8397b1dcdaf"]}
    "#);
    app.assert_undo_returns_to_start();
}

#[test]
fn the_run_is_gathered_where_its_frontmost_member_was() {
    let mut app =
        TestApp::with_entities([shape("a", A), shape("x", B), shape("b", C), shape("y", D)]);
    app.select(&["a", "b"]).act(Action::Group);
    let group = app.selected().expect("the group is selected").to_owned();
    assert_eq!(order(&app), ["x", "a", "b", group.as_str(), "y"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn one_item_is_not_enough_to_group() {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B)]);
    app.select(&["a"]).act(Action::Group);
    app.select(&[]).act(Action::Group);
    assert!(!app.app().can_undo());
    app.take_effects();
    app.select(&["a"]).act(Action::Group);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn a_group_whose_members_are_all_selected_stands_for_them() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        inside("g", shape("b", B)),
        shape("c", C),
    ]);
    app.select(&["a", "b", "c"]).act(Action::Group);
    let outer = app.selected().expect("the group is selected").to_owned();
    assert_eq!(parent(&app, "g"), Some(&*outer));
    assert_eq!(parent(&app, "a"), Some("g"), "its members stay in it");
    assert_eq!(parent(&app, "c"), Some(&*outer));
    app.assert_undo_returns_to_start();
}

#[test]
fn items_taken_out_of_a_group_leave_it() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        inside("g", shape("b", B)),
        shape("c", C),
    ]);
    app.select(&["a", "c"]).act(Action::Group);
    let new = app.selected().expect("the group is selected").to_owned();
    assert_eq!(
        (parent(&app, "a"), parent(&app, "c")),
        (Some(&*new), Some(&*new))
    );
    assert_eq!(parent(&app, "b"), Some("g"));
    assert_eq!(
        parent(&app, &new),
        Some("g"),
        "it takes the first item's group"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn ungrouping_frees_the_members_and_selects_them() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        inside("g", shape("b", B)),
        shape("c", C),
    ]);
    app.select(&["g"]).chord(CMD_SHIFT, Key::Char('g'));
    assert!(app.document().entity(&"g".into()).is_none());
    assert_eq!((parent(&app, "a"), parent(&app, "b")), (None, None));
    assert_eq!(app.selected_ids(), ["a", "b"]);
    assert_eq!(order(&app), ["a", "b", "c"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn an_inner_group_ungrouped_leaves_its_members_in_the_outer_one() {
    let mut app = TestApp::with_entities([
        group("outer", Rect::new(0.0, 0.0, 900.0, 300.0)),
        inside("outer", group("inner", Rect::new(76.0, 76.0, 348.0, 148.0))),
        inside("outer", shape("c", C)),
        inside("inner", shape("a", A)),
        inside("inner", shape("b", B)),
    ]);
    app.select(&["inner"]).act(Action::Ungroup);
    assert_eq!(
        (parent(&app, "a"), parent(&app, "b")),
        (Some("outer"), Some("outer"))
    );
    assert_eq!(app.selected_ids(), ["a", "b"]);
    app.assert_undo_returns_to_start();
}

#[test]
fn ungrouping_needs_one_group_selected() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        shape("c", C),
    ]);
    app.select(&["g", "c"]).act(Action::Ungroup);
    app.select(&["a"]).act(Action::Ungroup);
    app.select(&[]).act(Action::Ungroup);
    assert!(!app.app().can_undo());
}

#[test]
fn ungrouping_takes_the_groups_edges_with_it() {
    let start = specular_testkit::document([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        shape("c", C),
    ]);
    let mut app = TestApp::from_document(specular_testkit::connected(start, "e", "g", "c"));
    app.select(&["g"]).act(Action::Ungroup);
    assert!(app.document().edges().next().is_none());
    app.assert_undo_returns_to_start();
}

#[test]
fn deleting_a_group_deletes_what_is_inside_it() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(76.0, 76.0, 348.0, 148.0)),
        inside("g", shape("a", A)),
        inside("g", text("t", B)),
        shape("c", C),
    ]);
    app.select(&["g"]).key(Key::Backspace);
    assert_eq!(order(&app), ["c"]);
    app.assert_undo_returns_to_start();
}
