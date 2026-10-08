//! What the context menu holds for each thing under the pointer.

use specular_doc::Rect;
use specular_testkit::{TestApp, connected, document, group, inside, page, shape, sticky};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
const B: Rect = Rect::new(400.0, 100.0, 200.0, 100.0);
const SECOND: Rect = Rect::new(700.0, 100.0, 200.0, 100.0);

fn opened(entities: impl IntoIterator<Item = specular_doc::Entity>) -> TestApp {
    let mut app = TestApp::with_entities(entities);
    app.with_panels();
    app
}

fn menu_items(app: &TestApp) -> Vec<String> {
    (app.panel_layout().controls())
        .map(|id| id.as_str().to_owned())
        .filter(|id| id.starts_with("menu."))
        .collect()
}

#[test]
fn the_menu_holds_what_the_thing_under_the_pointer_can_do() {
    let clipboard = ["menu.cut", "menu.copy", "menu.paste", "menu.duplicate"];
    let order = [
        "menu.bring-forward",
        "menu.send-backward",
        "menu.bring-to-front",
        "menu.send-to-back",
    ];
    let with = |first: &[&str], more: &[&str]| -> Vec<String> {
        let mut items: Vec<&str> = first.to_vec();
        items.extend(clipboard);
        items.extend(order);
        items.extend(more);
        items.push("menu.annotate-selection");
        items.push("menu.delete");
        items.into_iter().map(str::to_owned).collect()
    };

    let mut empty = opened([shape("a", A)]);
    empty.right_click((900.0, 600.0));
    assert_eq!(
        menu_items(&empty),
        ["menu.paste", "menu.select-all"],
        "empty canvas"
    );

    let mut sticky_app = opened([sticky("n", A, "note")]);
    sticky_app.right_click((150.0, 150.0));
    assert_eq!(menu_items(&sticky_app), with(&[], &[]), "a sticky");

    let mut page_app = opened([page("p", Rect::new(100.0, 100.0, 375.0, 400.0))]);
    page_app.right_click((200.0, 300.0));
    assert_eq!(
        menu_items(&page_app),
        with(&["menu.back", "menu.forward", "menu.reload"], &[]),
        "a page has its history first"
    );

    let mut several = opened([shape("a", A), shape("b", B), shape("c", SECOND)]);
    several.select(&["a", "b"]);
    several.right_click((150.0, 150.0));
    assert!(several.selected_ids().contains(&"b"), "the selection stays");
    assert_eq!(
        menu_items(&several),
        with(&[], &["menu.group"]),
        "several items"
    );

    let mut grouped = opened([
        group("g", Rect::new(100.0, 100.0, 400.0, 200.0)),
        inside("g", shape("m", Rect::new(120.0, 120.0, 100.0, 100.0))),
    ]);
    grouped.right_click((110.0, 280.0));
    assert_eq!(
        menu_items(&grouped),
        with(&[], &["menu.ungroup"]),
        "a group"
    );

    let mut edge = TestApp::empty();
    edge.with_panels();
    edge.open(connected(
        document([shape("a", A), shape("b", B)]),
        "e",
        "a",
        "b",
    ));
    let at = edge
        .app()
        .edge_curve(&"e".into())
        .expect("a curve")
        .point(0.5);
    edge.right_click(at);
    let mut expected: Vec<String> = order.iter().map(|id| (*id).to_owned()).collect();
    expected.push("menu.delete".to_owned());
    assert_eq!(menu_items(&edge), expected, "an edge");
}
