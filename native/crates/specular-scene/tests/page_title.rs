//! The title line above a page: its label, or its live title and address.

use specular_doc::Rect;
use specular_interact::PageNotice;
use specular_testkit::{TestApp, assert_scene_snapshot, page};

const BOX: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

/// The text runs of the scene, one per line.
fn text_lines(app: &TestApp) -> String {
    (app.scene_snapshot().lines())
        .filter(|line| line.contains("text"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn app() -> TestApp {
    TestApp::with_entities([page("p1", BOX)])
}

#[test]
fn a_page_that_has_said_nothing_shows_its_address() {
    assert_scene_snapshot!(app());
}

#[test]
fn a_page_with_a_title_shows_it_before_the_address() {
    let mut app = app();
    app.page_reports("p1", PageNotice::Title("Example Domain".to_owned()))
        .page_reports("p1", PageNotice::Url("https://example.com/".to_owned()));
    assert!(text_lines(&app).contains("Example Domain \u{2014} example.com"));
    assert_scene_snapshot!(app);
}

#[test]
fn a_loading_page_says_so() {
    let mut app = app();
    app.page_reports("p1", PageNotice::Title("Example Domain".to_owned()))
        .page_reports(
            "p1",
            PageNotice::Loading {
                loading: true,
                can_go_back: false,
                can_go_forward: false,
            },
        );
    assert!(text_lines(&app).contains("Loading\u{2026} Example Domain"));
    assert_scene_snapshot!(app);
}

#[test]
fn a_label_wins_over_the_live_title() {
    let mut named = page("p1", BOX);
    named.label = Some("Home".to_owned());
    let mut app = TestApp::with_entities([named]);
    app.page_reports("p1", PageNotice::Title("Example Domain".to_owned()));
    let text = text_lines(&app);
    assert!(
        text.contains("Home") && !text.contains("Example Domain"),
        "{text}"
    );
    assert_scene_snapshot!(app);
}
