//! What the context menu holds for each thing under the pointer: the model
//! alone, as a renderer of any kind would be handed it.

use specular_doc::Rect;
use specular_interact::{MenuTarget, context_menu};
use specular_testkit::{
    TestApp, assert_menu_snapshot, connected, document, group, inside, page, shape, sticky,
};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const SECOND: Rect = Rect::new(700.0, 100.0, 200.0, 100.0);

fn opened(entities: impl IntoIterator<Item = specular_doc::Entity>) -> TestApp {
    let mut app = TestApp::with_entities(entities);
    app.with_panels();
    app
}

#[test]
fn empty_canvas_offers_paste_and_select_all() {
    let mut app = opened([shape("a", A)]);
    app.right_click((900.0, 600.0));
    assert_menu_snapshot!(app, @r#"
    anchor point 900,600
    choices menu "Menu"
      options list
        option [ ] menu.paste "Paste" chord=cmd+v -> Paste
      options list
        option [ ] menu.select-all "Select all" chord=cmd+a -> SelectAll
    "#);
}

#[test]
fn a_sticky_has_the_edit_the_order_and_delete() {
    let mut app = opened([sticky("n", A, "note")]);
    app.right_click((150.0, 150.0));
    assert_menu_snapshot!(app, @r#"
    anchor point 150,150
    choices menu "Menu"
      options list
        option [ ] menu.cut "Cut" chord=cmd+x -> Cut
        option [ ] menu.copy "Copy" chord=cmd+c -> Copy
        option [ ] menu.paste "Paste" chord=cmd+v -> Paste
        option [ ] menu.duplicate "Duplicate" chord=cmd+d -> Duplicate
      options list
        option [ ] menu.bring-forward "Bring forward" chord=cmd+] -> BringForward
        option [ ] menu.send-backward "Send backward" chord=cmd+[ -> SendBackward
        option [ ] menu.bring-to-front "Bring to front" chord=cmd+shift+] -> BringToFront
        option [ ] menu.send-to-back "Send to back" chord=cmd+shift+[ -> SendToBack
      options list
        option [ ] menu.annotate-selection "Annotate selection" -> AnnotateSelection
      options list
        option [ ] menu.delete "Delete" chord=backspace -> Delete
    "#);
}

#[test]
fn a_page_has_its_history_first() {
    let mut app = opened([page("p", Rect::new(100.0, 100.0, 375.0, 400.0))]);
    app.right_click((200.0, 300.0));
    assert_menu_snapshot!(app, @r#"
    anchor point 200,300
    choices menu "Menu"
      options list
        option [ ] menu.back "Back" disabled -> PageBack
        option [ ] menu.forward "Forward" disabled -> PageForward
        option [ ] menu.reload "Reload" chord=cmd+r -> PageReload
      options list
        option [ ] menu.cut "Cut" chord=cmd+x -> Cut
        option [ ] menu.copy "Copy" chord=cmd+c -> Copy
        option [ ] menu.paste "Paste" chord=cmd+v -> Paste
        option [ ] menu.duplicate "Duplicate" chord=cmd+d -> Duplicate
      options list
        option [ ] menu.bring-forward "Bring forward" chord=cmd+] -> BringForward
        option [ ] menu.send-backward "Send backward" chord=cmd+[ -> SendBackward
        option [ ] menu.bring-to-front "Bring to front" chord=cmd+shift+] -> BringToFront
        option [ ] menu.send-to-back "Send to back" chord=cmd+shift+[ -> SendToBack
      options list
        option [ ] menu.annotate-selection "Annotate selection" -> AnnotateSelection
      options list
        option [ ] menu.delete "Delete" chord=backspace -> Delete
    "#);
}

#[test]
fn several_items_can_be_grouped() {
    let mut app = opened([shape("a", A), shape("b", B), shape("c", SECOND)]);
    app.select(&["a", "b"]);
    app.right_click((150.0, 150.0));
    assert!(app.selected_ids().contains(&"b"), "the selection stays");
    assert_menu_snapshot!(app, @r#"
    anchor point 150,150
    choices menu "Menu"
      options list
        option [ ] menu.cut "Cut" chord=cmd+x -> Cut
        option [ ] menu.copy "Copy" chord=cmd+c -> Copy
        option [ ] menu.paste "Paste" chord=cmd+v -> Paste
        option [ ] menu.duplicate "Duplicate" chord=cmd+d -> Duplicate
      options list
        option [ ] menu.bring-forward "Bring forward" chord=cmd+] -> BringForward
        option [ ] menu.send-backward "Send backward" chord=cmd+[ -> SendBackward
        option [ ] menu.bring-to-front "Bring to front" chord=cmd+shift+] -> BringToFront
        option [ ] menu.send-to-back "Send to back" chord=cmd+shift+[ -> SendToBack
      options list
        option [ ] menu.group "Group" chord=cmd+g -> Group
        option [ ] menu.annotate-selection "Annotate selection" -> AnnotateSelection
      options list
        option [ ] menu.delete "Delete" chord=backspace -> Delete
    "#);
}

#[test]
fn a_group_can_be_ungrouped() {
    let mut app = opened([
        group("g", Rect::new(100.0, 100.0, 400.0, 200.0)),
        inside("g", shape("m", Rect::new(120.0, 120.0, 100.0, 100.0))),
    ]);
    app.right_click((110.0, 280.0));
    assert_menu_snapshot!(app, @r#"
    anchor point 110,280
    choices menu "Menu"
      options list
        option [ ] menu.cut "Cut" chord=cmd+x -> Cut
        option [ ] menu.copy "Copy" chord=cmd+c -> Copy
        option [ ] menu.paste "Paste" chord=cmd+v -> Paste
        option [ ] menu.duplicate "Duplicate" chord=cmd+d -> Duplicate
      options list
        option [ ] menu.bring-forward "Bring forward" chord=cmd+] -> BringForward
        option [ ] menu.send-backward "Send backward" chord=cmd+[ -> SendBackward
        option [ ] menu.bring-to-front "Bring to front" chord=cmd+shift+] -> BringToFront
        option [ ] menu.send-to-back "Send to back" chord=cmd+shift+[ -> SendToBack
      options list
        option [ ] menu.ungroup "Ungroup" chord=cmd+shift+g -> Ungroup
        option [ ] menu.annotate-selection "Annotate selection" -> AnnotateSelection
      options list
        option [ ] menu.delete "Delete" chord=backspace -> Delete
    "#);
}

#[test]
fn an_edge_has_the_order_and_delete() {
    let mut app = TestApp::empty();
    app.with_panels();
    app.open(connected(
        document([shape("a", A), shape("b", B)]),
        "e",
        "a",
        "b",
    ));
    let curve = app.app().edge_curve(&"e".into());
    let at = curve.expect("the edge has a curve").point(0.5);
    app.right_click(at);
    assert_menu_snapshot!(app, @r#"
    anchor point 350,150
    choices menu "Menu"
      options list
        option [ ] menu.bring-forward "Bring forward" chord=cmd+] -> BringForward
        option [ ] menu.send-backward "Send backward" chord=cmd+[ -> SendBackward
        option [ ] menu.bring-to-front "Bring to front" chord=cmd+shift+] -> BringToFront
        option [ ] menu.send-to-back "Send to back" chord=cmd+shift+[ -> SendToBack
      options list
        option [ ] menu.delete "Delete" chord=backspace -> Delete
    "#);
}

#[test]
fn the_menu_is_none_once_its_selection_has_changed() {
    let mut app = opened([shape("a", A), shape("b", B)]);
    app.right_click((150.0, 150.0));
    let target = app
        .session()
        .panel
        .menu
        .clone()
        .expect("a menu is open")
        .target;
    assert!(context_menu(app.app(), &target, glam::Vec2::ZERO).is_some());
    app.select(&["b"]);
    assert!(context_menu(app.app(), &target, glam::Vec2::ZERO).is_none());
    assert!(matches!(target, MenuTarget::Selection(_)));
}
