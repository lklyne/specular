//! The notices mod against `notices`.

use gpui::TestAppContext;
use serde_json::json;
use specular_doc::EntityId;
use specular_interact::{Event, PageNotice, state};
use specular_testkit::TestApp;

use crate::support::{Loaded, grid, timed_load, window_app};

const SIZE: (f32, f32) = (360.0, 400.0);

fn tell(app: &mut TestApp, page: &str, notice: PageNotice) {
    app.send(Event::Page {
        page: EntityId::from(page),
        notice,
    });
}

/// An app whose two pages each have a notice: one stopped, one answered
/// an HTTP error.
fn app_with_notices() -> TestApp {
    let mut app = TestApp::with_pages(2);
    app.send(Event::ShellChrome);
    tell(
        &mut app,
        "p1",
        PageNotice::Url("https://example.com/".to_owned()),
    );
    tell(
        &mut app,
        "p1",
        PageNotice::Crashed {
            reason: "oom".to_owned(),
        },
    );
    tell(&mut app, "p2", PageNotice::Loaded { http_status: 404 });
    app
}

#[gpui::test]
fn the_toast_shows_every_notice_the_model_lists_but_those_hidden(cx: &mut TestAppContext) {
    let app = app_with_notices();
    let notices = state(app.app())["notices"].clone();
    let ids: Vec<String> = (notices.as_array().into_iter().flatten())
        .filter_map(|notice| notice["id"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(ids, ["page.p1.crashed", "page.p2.http"], "{notices}");
    for hidden in [&[][..], &["page.p2.http"]] {
        let tree = Loaded::new(&app, "toast", json!({ "hidden": hidden }), SIZE, cx).tree();
        for id in &ids {
            let hide = hidden.contains(&id.as_str());
            let shown = tree.contains(&format!("Button \"{id}.dismiss\""));
            assert_eq!(shown, !hide, "`{id}` with {hidden:?} hidden:\n{tree}");
        }
        assert_eq!(
            tree.contains("Button \"page.p1.crashed.action\""),
            !hidden.contains(&"page.p1.crashed"),
            "the stopped page offers Reload:\n{tree}"
        );
    }
    let mut quiet = window_app();
    let mut loaded = Loaded::new(&quiet, "toast", json!({}), SIZE, cx);
    assert!(!loaded.tree().contains("Button"), "no notices, no cards");
    loaded.assert_camera_moves_nothing(&mut quiet);
}

#[gpui::test]
fn reload_and_dismiss_reach_the_host(cx: &mut TestAppContext) {
    let app = app_with_notices();
    let mut loaded = Loaded::new(&app, "toast", json!({}), SIZE, cx);
    let reload = r#"page.navigate {"page":"p1","url":"https://example.com/"}"#;
    let dismiss = r#"notice.dismiss {"id":"page.p2.http"}"#;
    loaded.click_until(grid((340, 300), 5), reload);
    let runs = loaded.click_until(grid((340, 300), 5), dismiss);
    assert!(
        runs.contains(&reload.to_owned()) && runs.contains(&dismiss.to_owned()),
        "{runs:?}"
    );
}

#[gpui::test]
fn a_dismissed_notice_goes_and_comes_back_when_the_page_fails_again(cx: &mut TestAppContext) {
    let mut app = app_with_notices();
    let mut loaded = timed_load(&app, "toast", json!({}), SIZE, cx);
    assert_eq!(
        loaded.reads("toast"),
        ["mods/toast/props", "notices", "theme"]
    );
    assert_eq!(loaded.idle_renders(), 0, "no script render while idle");
    app.act(specular_interact::Action::DismissNotice(
        "page.p1.crashed".to_owned(),
    ));
    assert!(loaded.step(&app));
    assert!(!loaded.tree().contains("page.p1.crashed"), "dismissed");
    // A page that fails, then loads again, then fails: one render a step.
    crate::support::assert_renders_inside_a_frame(&mut loaded, &mut app, "toast", 300, |app, i| {
        if i % 2 == 0 {
            tell(
                app,
                "p1",
                PageNotice::Crashed {
                    reason: format!("crash {i}"),
                },
            );
        } else {
            tell(
                app,
                "p1",
                PageNotice::Loading {
                    loading: true,
                    can_go_back: false,
                    can_go_forward: false,
                },
            );
        }
    });
}
