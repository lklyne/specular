//! `POST /annotations/fix`: one comment, or every open one on an origin,
//! sent to the agent on the origin's repo.
//!
//! The change that breaks it: sending a comment on an origin with no repo,
//! a resolved one, or one thread's comments as several runs.

use serde_json::{Value, json};
use specular_interact::Effect;
use specular_testkit::TestApp;

use crate::common::Scripted;

/// Two open comments and a resolved one on a page of the bound origin, and
/// one on a page of an origin bound to nothing. Returns their ids in that
/// order.
fn commented() -> (Scripted, [Value; 4]) {
    let mut app = TestApp::empty();
    app.viewport((1000.0, 800.0))
        .bind("https://bound.test", "/scratch/site", false);
    let mut session = Scripted::new(app);
    let made = session.apply(json!({ "entities": [
        { "kind": "page", "url": "https://bound.test/a", "canvasX": 0, "canvasY": 0 },
        { "kind": "page", "url": "https://loose.test/", "canvasX": 1400, "canvasY": 0 },
    ]}));
    let pages = [0, 0, 0, 1].map(|at| made["created"][at].clone());
    let ids = pages.map(|page| {
        let body = json!({ "text": "fix me", "anchor": { "type": "page", "pageId": page } });
        session.post("/annotations", body).body["id"].clone()
    });
    let resolved = session.post(
        &format!("/annotations/{}/resolve", ids[2].as_str().unwrap_or("")),
        json!({}),
    );
    assert_eq!(resolved.status, 200, "{}", resolved.body);
    (session, ids)
}

#[test]
fn fixing_sends_open_comments_on_a_bound_origin_one_run_a_thread() {
    // The body, given the comments' ids; the answer; the runs started.
    type Row = (&'static str, fn(&[Value; 4]) -> Value, Value, usize);
    let rows: [Row; 5] = [
        (
            "a bound comment",
            |ids| json!({ "annotationId": ids[0] }),
            json!({ "ok": true, "queued": 1 }),
            1,
        ),
        (
            "a resolved comment",
            |ids| json!({ "annotationId": ids[2] }),
            json!({ "ok": false, "queued": 0 }),
            0,
        ),
        (
            "a comment with no repo",
            |ids| json!({ "annotationId": ids[3] }),
            json!({ "ok": false, "queued": 0 }),
            0,
        ),
        (
            "the bound origin",
            |_| json!({ "origin": "https://bound.test" }),
            json!({ "ok": true, "queued": 2 }),
            1,
        ),
        (
            "an origin with no repo",
            |_| json!({ "origin": "https://loose.test" }),
            json!({ "ok": true, "queued": 0 }),
            0,
        ),
    ];
    for (name, body, answer, runs) in rows {
        let (mut session, ids) = commented();
        let response = session.post("/annotations/fix", body(&ids));
        assert_eq!((response.status, response.body), (200, answer), "{name}");
        let started: Vec<Option<String>> = (session.effects.iter())
            .filter_map(|effect| match effect {
                Effect::RunAgent(request) => Some(request.cwd.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            started,
            vec![Some("/scratch/site".to_owned()); runs],
            "{name}"
        );
    }
    let (mut session, _) = commented();
    assert_eq!(session.post("/annotations/fix", json!({})).status, 400);
}
