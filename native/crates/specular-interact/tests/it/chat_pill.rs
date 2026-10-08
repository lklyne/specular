//! The pill over the composer: what this turn is about, read from the app.

use specular_doc::{AnnotationAnchor, Rect};
use specular_interact::{Action, PageNotice, PillKind, Tool};
use specular_testkit::{
    TestApp, comment, document, file, group, page, plain_text, shape, with_comment,
};

const AT: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

fn pill(app: &TestApp) -> (PillKind, String) {
    let pill = app.chat().composer.pill;
    (pill.kind, pill.label)
}

#[test]
fn a_single_selection_is_named_by_its_kind_and_several_are_counted() {
    let long = "Make   the headline bigger and bolder, please do it now";
    let doc = document([
        page("p1", AT),
        file("f1", AT),
        group("g1", AT),
        plain_text("t1", AT, long),
        shape("s1", AT),
    ]);
    let mut app = TestApp::from_document(doc);
    app.with_chat_panel();
    let table: [(&[&str], &str); 6] = [
        (&["p1"], "page"),
        (&["f1"], "f1.png"),
        (&["g1"], "group"),
        (&["t1"], "Make the headline bigger and bolder, pl\u{2026}"),
        (&["s1"], "shape"),
        (&["p1", "f1", "s1"], "3 items"),
    ];
    for (ids, label) in table {
        app.select(ids);
        assert_eq!(
            pill(&app),
            (PillKind::Selection, label.to_owned()),
            "{ids:?}"
        );
    }
    app.page_reports("p1", PageNotice::Title("Home page".into()))
        .select(&["p1"]);
    assert_eq!(
        pill(&app).1,
        "Home page",
        "a page is named by its live title"
    );
    app.select(&[]);
    assert_eq!(pill(&app), (PillKind::Canvas, "Canvas 1".to_owned()));
}

#[test]
fn a_focused_comment_comes_before_the_selection() {
    let anchor = AnnotationAnchor::Canvas {
        canvas_x: 50.0,
        canvas_y: 50.0,
    };
    let doc = with_comment(
        document([shape("s1", AT)]),
        comment("c1", anchor, "fix the header"),
    );
    let mut app = TestApp::from_document(doc);
    app.with_chat_panel().select(&["s1"]);
    assert_eq!(pill(&app).0, PillKind::Selection);
    app.act(Action::FocusComment(Some("c1".into())));
    assert_eq!(pill(&app), (PillKind::Comment, "fix the header".to_owned()));
}

#[test]
fn a_picked_node_comes_before_a_focused_comment_and_the_selection() {
    let anchor = AnnotationAnchor::Canvas {
        canvas_x: 50.0,
        canvas_y: 50.0,
    };
    let doc = with_comment(
        document([
            page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
            shape("s1", AT),
        ]),
        comment("c1", anchor, "fix the header"),
    );
    let mut app = TestApp::from_document(doc);
    app.with_chat_panel().select(&["s1"]);
    app.act(Action::FocusComment(Some("c1".into())));
    assert_eq!(pill(&app).0, PillKind::Comment);
    app.tool(Tool::Inspect)
        .click((200.0, 150.0))
        .answer_inspect();
    assert_eq!(pill(&app), (PillKind::Dom, "div \"Cell 0,1\"".to_owned()));
}
