//! The pill over the composer: what this turn is about, read from the app.

use specular_doc::{AnnotationAnchor, Entity, FileRef, ItemId, Kind, Rect};
use specular_interact::{Action, PageNotice, PillKind, Tool};
use specular_testkit::{
    TestApp, comment, connected, document, file, group, page, plain_text, shape, with_comment,
};

const AT: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

fn pill(app: &TestApp) -> (PillKind, String) {
    let pill = app.chat().composer.pill;
    (pill.kind, pill.label)
}

#[test]
fn a_single_selection_is_named_by_its_kind_and_several_are_counted() {
    let long = "Make   the headline bigger and bolder, please do it now";
    let nested = Entity {
        kind: Kind::File(FileRef {
            file: "assets/shot.png".to_owned(),
            ..FileRef::default()
        }),
        ..file("f2", AT)
    };
    let padded = Entity {
        label: Some("  Moodboard ".to_owned()),
        ..group("g2", AT)
    };
    let exact = "abcdefghij".repeat(4);
    let doc = document([
        page("p1", AT),
        page("p2", AT),
        file("f1", AT),
        nested,
        group("g1", AT),
        padded,
        plain_text("t1", AT, long),
        plain_text("t2", AT, &exact),
        shape("s1", AT),
    ]);
    let doc = connected(doc, "e1", "s1", "t2");
    let mut app = TestApp::from_document(doc);
    app.with_chat_panel();
    let table: [(&[&str], &str); 10] = [
        (&["p1"], "page"),
        (&["f1"], "f1.png"),
        (&["f2"], "shot.png"),
        (&["g1"], "group"),
        (&["g2"], "Moodboard"),
        (&["t1"], "Make the headline bigger and bolder, pl\u{2026}"),
        (&["t2"], exact.as_str()),
        (&["s1"], "shape"),
        (&["p1", "f1", "s1"], "3 items"),
        (&["p2"], "page"),
    ];
    for (ids, label) in table {
        app.select(ids);
        assert_eq!(
            pill(&app),
            (PillKind::Selection, label.to_owned()),
            "{ids:?}"
        );
    }
    app.page_reports("p1", PageNotice::Title(" Home page ".into()))
        .select(&["p1"]);
    assert_eq!(
        pill(&app).1,
        "Home page",
        "a page is named by its live title"
    );
    app.page_reports("p2", PageNotice::Title("  ".into()))
        .select(&["p2"]);
    assert_eq!(pill(&app).1, "page", "a blank title names nothing");
    app.act(Action::Select(vec![ItemId::Edge("e1".into())]));
    assert_eq!(pill(&app), (PillKind::Selection, "edge".to_owned()));
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
