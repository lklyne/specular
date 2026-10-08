//! `POST /tasks/apply`: the `breakpoints` verb.

use serde_json::{Value, json};
use specular_doc::{EntityId, Kind, VIEWPORT_PRESETS};

use crate::common::Scripted;

const URL: &str = "https://example.com/shop";

#[test]
fn breakpoints_lay_one_page_per_preset_in_a_row_inside_a_group_as_one_undo_step() {
    let mut session = Scripted::empty();
    let response = session.post(
        "/tasks/apply",
        json!({ "taskKind": "breakpoint_map", "input": { "url": URL } }),
    );
    assert_eq!(response.status, 200, "{}", response.body);
    let reply = response.body;

    assert_eq!(
        reply["resolvedPresets"],
        json!(["iPhone 14 Pro", "iPad Mini", "Desktop"])
    );
    assert_eq!(reply["taskKind"], "breakpoint_map");
    assert_eq!(reply["warnings"], json!([]));
    assert_eq!(reply["edgeIds"].as_array().map(Vec::len), Some(2));
    let group = reply["groupId"].as_str().unwrap_or_default();
    let pages: Vec<&str> = (reply["pageIds"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(pages.len(), 3);

    let document = session.app.document();
    let label = document
        .entity(&EntityId::from(group))
        .and_then(|e| e.label.clone());
    assert_eq!(label.as_deref(), Some("Breakpoints: example.com/shop"));
    let mut sync_ids = Vec::new();
    let mut next_x = reply["placement"]["canvasX"].as_f64().unwrap_or_default();
    for (id, name) in pages.iter().zip(["iPhone 14 Pro", "iPad Mini", "Desktop"]) {
        let preset = VIEWPORT_PRESETS
            .iter()
            .find(|p| p.label == name)
            .expect("preset");
        let entity = document.entity(&EntityId::from(*id)).expect("page exists");
        assert_eq!(entity.parent.as_ref().map(EntityId::as_str), Some(group));
        assert_eq!(
            (
                entity.rect.x,
                entity.rect.y,
                entity.rect.width,
                entity.rect.height
            ),
            (
                next_x,
                reply["placement"]["canvasY"].as_f64().unwrap_or_default(),
                preset.width,
                preset.height
            )
        );
        next_x += preset.width + 80.0;
        let Kind::Page(page) = &entity.kind else {
            panic!("{id} is not a page");
        };
        assert_eq!(page.url, URL);
        sync_ids.push(page.sync_id.clone());
    }
    assert!(
        sync_ids[0]
            .as_deref()
            .is_some_and(|id| id.starts_with("sync_"))
    );
    assert!(sync_ids.iter().all(|id| *id == sync_ids[0]));

    session.undo();
    session.app.assert_undo_returns_to_start();
    assert_eq!(session.canvas()["nodes"], json!([]));
    assert_eq!(session.canvas()["edges"], json!([]));
}

#[test]
fn breakpoints_refuse_what_electron_refuses() {
    let cases = [
        (
            json!({ "taskKind": "component_states", "input": { "url": URL } }),
            "Unsupported task kind: component_states",
        ),
        (
            json!({ "taskKind": "breakpoint_map", "input": { "url": URL, "presets": ["Nokia 3310"] } }),
            "Unknown preset label: Nokia 3310",
        ),
        (
            json!({ "taskKind": "breakpoint_map", "input": { "url": URL, "presets": ["Desktop", "Desktop"] } }),
            "Duplicate preset labels are not allowed",
        ),
    ];
    for (body, error) in cases {
        let mut session = Scripted::empty();
        let response = session.post("/tasks/apply", body);
        assert_eq!(response.status, 400);
        assert_eq!(response.body, json!({ "error": error }));
        assert_eq!(session.canvas()["nodes"], json!([]));
    }
}

/// The page address and the group's title a URL typed as `input` comes to.
fn address_of(input: &str) -> Result<(String, String), String> {
    let mut session = Scripted::empty();
    let response = session.post(
        "/tasks/apply",
        json!({ "taskKind": "breakpoint_map", "input": { "url": input, "presets": ["Desktop"] } }),
    );
    if response.status != 200 {
        return Err(response.body["error"]
            .as_str()
            .unwrap_or_default()
            .to_owned());
    }
    let page = response.body["pageIds"][0].as_str().unwrap_or_default();
    let group = response.body["groupId"].as_str().unwrap_or_default();
    let document = session.app.document();
    let Some(Kind::Page(page)) = document
        .entity(&EntityId::from(page))
        .map(|e| e.kind.clone())
    else {
        panic!("not a page");
    };
    let title = document
        .entity(&EntityId::from(group))
        .and_then(|e| e.label.clone());
    Ok((page.url, title.unwrap_or_default()))
}

#[test]
fn breakpoints_make_the_url_whole_the_way_electron_does() {
    let rows = [
        (
            "example.com",
            "https://example.com/",
            "Breakpoints: example.com",
        ),
        (
            "  example.com/a/b  ",
            "https://example.com/a/b",
            "Breakpoints: example.com/a/b",
        ),
        (
            "HTTP://Example.COM/Path?q=1#h",
            "http://example.com/Path?q=1#h",
            "Breakpoints: example.com/Path",
        ),
        (
            "example.com?x=1",
            "https://example.com/?x=1",
            "Breakpoints: example.com",
        ),
        (
            "//cdn.example.com/x",
            "https://cdn.example.com/x",
            "Breakpoints: cdn.example.com/x",
        ),
        (
            "user:pw@example.com:8443/x",
            "https://user:pw@example.com:8443/x",
            "Breakpoints: example.com/x",
        ),
        (
            "localhost:3000/a",
            "http://localhost:3000/a",
            "Breakpoints: localhost/a",
        ),
        (
            "LocalHost:3000",
            "http://localhost:3000/",
            "Breakpoints: localhost",
        ),
        (
            "10.0.0.1.x",
            "https://10.0.0.1.x/",
            "Breakpoints: 10.0.0.1.x",
        ),
        (
            "app.localhost",
            "https://app.localhost/",
            "Breakpoints: app.localhost",
        ),
        (
            "a+b.c-d://Host/x",
            "a+b.c-d://host/x",
            "Breakpoints: host/x",
        ),
        ("[::1]:3000", "http://[::1]:3000/", "Breakpoints: [::1]"),
        (
            "127.0.0.1:80",
            "http://127.0.0.1:80/",
            "Breakpoints: 127.0.0.1",
        ),
        ("10.1.2.3", "http://10.1.2.3/", "Breakpoints: 10.1.2.3"),
        (
            "192.168.0.9",
            "http://192.168.0.9/",
            "Breakpoints: 192.168.0.9",
        ),
        (
            "192.169.0.9",
            "https://192.169.0.9/",
            "Breakpoints: 192.169.0.9",
        ),
        (
            "172.16.0.1",
            "http://172.16.0.1/",
            "Breakpoints: 172.16.0.1",
        ),
        (
            "172.31.0.1",
            "http://172.31.0.1/",
            "Breakpoints: 172.31.0.1",
        ),
        (
            "172.32.0.1",
            "https://172.32.0.1/",
            "Breakpoints: 172.32.0.1",
        ),
        (
            "172.15.0.1",
            "https://172.15.0.1/",
            "Breakpoints: 172.15.0.1",
        ),
        (
            "169.254.1.1",
            "http://169.254.1.1/",
            "Breakpoints: 169.254.1.1",
        ),
        (
            "169.253.1.1",
            "https://169.253.1.1/",
            "Breakpoints: 169.253.1.1",
        ),
        ("1.2.3", "https://1.2.3/", "Breakpoints: 1.2.3"),
        (
            "10.0.0.1.5",
            "https://10.0.0.1.5/",
            "Breakpoints: 10.0.0.1.5",
        ),
        (
            "my.box.local",
            "http://my.box.local/",
            "Breakpoints: my.box.local",
        ),
        (
            "my.local.com",
            "https://my.local.com/",
            "Breakpoints: my.local.com",
        ),
        ("data:text/html,hi", "data:text/html,hi", "Breakpoints: "),
        ("BLOB:abc", "BLOB:abc", "Breakpoints: "),
        (
            "file:///tmp/a.html",
            "file:///tmp/a.html",
            "Breakpoints: /tmp/a.html",
        ),
        ("ftp://Host/x", "ftp://host/x", "Breakpoints: host/x"),
    ];
    for (input, url, title) in rows {
        assert_eq!(
            address_of(input),
            Ok((url.to_owned(), title.to_owned())),
            "{input}"
        );
    }
    for (input, error) in [
        ("   ", "URL cannot be empty"),
        ("https://", "Invalid URL: https://"),
    ] {
        assert_eq!(address_of(input), Err(error.to_owned()), "{input}");
    }
}

#[test]
fn breakpoints_follow_their_options_and_describe_the_cluster() {
    let mut session = Scripted::empty();
    let post = |session: &mut Scripted, body: Value| {
        let response = session.post(
            "/tasks/apply",
            json!({ "taskKind": "breakpoint_map", "input": body }),
        );
        assert_eq!(response.status, 200, "{}", response.body);
        response.body
    };
    // A label, trimmed, names the group; a blank one falls back to the address.
    let named = post(&mut session, json!({ "url": URL, "label": "  Shop  " }));
    let titled = |session: &mut Scripted, reply: &Value| {
        session
            .app
            .document()
            .entity(&EntityId::from(
                reply["groupId"].as_str().unwrap_or_default(),
            ))
            .and_then(|e| e.label.clone())
    };
    assert_eq!(titled(&mut session, &named).as_deref(), Some("Shop"));
    assert_eq!(named["warnings"], json!([]));
    let blank = post(&mut session, json!({ "url": URL, "label": "   " }));
    assert_eq!(
        titled(&mut session, &blank).as_deref(),
        Some("Breakpoints: example.com/shop")
    );
    assert_eq!(
        blank["warnings"],
        json!(["A breakpoint cluster for this URL already exists"]),
        "the same address and presets again"
    );
    let other = post(
        &mut session,
        json!({ "url": URL, "presets": ["Desktop", "iPad Mini"] }),
    );
    assert_eq!(
        other["warnings"],
        json!([]),
        "other presets are another cluster"
    );

    let different = post(&mut session, json!({ "url": "https://example.com/other" }));
    assert_eq!(
        different["warnings"],
        json!([]),
        "another address is another cluster"
    );

    // The group and what is in it say where they came from.
    let group = session
        .app
        .document()
        .entity(&EntityId::from(
            named["groupId"].as_str().unwrap_or_default(),
        ))
        .cloned()
        .expect("group");
    let Kind::Group(details) = &group.kind else {
        panic!("not a group")
    };
    assert_eq!(
        (
            details.managed_layout,
            details.layout_gap,
            details.source_task_id.as_deref()
        ),
        (Some(true), Some(80.0), named["taskId"].as_str())
    );
    let meta = details
        .metadata
        .clone()
        .map(Value::Object)
        .unwrap_or_default();
    assert_eq!(
        meta,
        json!({ "taskKind": "breakpoint_map", "url": URL, "presets": ["iPhone 14 Pro", "iPad Mini", "Desktop"] })
    );
    let first_page = session.node(&named["pageIds"][0]);
    let (x, y) = (
        first_page["x"].as_f64().unwrap_or(0.0),
        first_page["y"].as_f64().unwrap_or(0.0),
    );
    let row = ["iPhone 14 Pro", "iPad Mini", "Desktop"].map(|name| {
        VIEWPORT_PRESETS
            .iter()
            .find(|p| p.label == name)
            .map_or((0.0, 0.0), |p| (p.width, p.height))
    });
    let (width, height) = (
        row.iter().map(|s| s.0).sum::<f64>() + 160.0,
        row.iter().map(|s| s.1).fold(0.0, f64::max),
    );
    assert_eq!(
        group.rect,
        specular_doc::Rect::new(x - 24.0, y - 24.0, width + 48.0, height + 48.0)
    );
    let edge = session.canvas()["edges"][0].clone();
    assert_eq!(edge["edgeKind"], "breakpoint_variant");
    assert_eq!(
        edge["edgeMetadata"],
        json!({ "taskKind": "breakpoint_map", "url": URL })
    );
    let first = session
        .app
        .document()
        .entity(&EntityId::from(
            named["pageIds"][0].as_str().unwrap_or_default(),
        ))
        .cloned()
        .expect("page");
    let Kind::Page(page) = &first.kind else {
        panic!("not a page")
    };
    let row = VIEWPORT_PRESETS
        .iter()
        .position(|p| p.label == "iPhone 14 Pro");
    assert_eq!(
        page.preset_index,
        row.and_then(|row| u32::try_from(row).ok())
    );
    assert_eq!(page.source, Some(specular_doc::PageSource::Generated));

    // One preset is a lone page: no sync id, no edges. Null or empty presets mean the defaults.
    let lone = post(&mut session, json!({ "url": URL, "presets": ["Desktop"] }));
    assert_eq!(
        (
            lone["edgeIds"].clone(),
            lone["pageIds"].as_array().map(Vec::len)
        ),
        (json!([]), Some(1))
    );
    let page = session
        .app
        .document()
        .entity(&EntityId::from(
            lone["pageIds"][0].as_str().unwrap_or_default(),
        ))
        .cloned()
        .expect("page");
    let Kind::Page(page) = &page.kind else {
        panic!("not a page")
    };
    assert_eq!(page.sync_id, None);
    for presets in [json!(null), json!([])] {
        let filled = post(&mut session, json!({ "url": URL, "presets": presets }));
        assert_eq!(filled["resolvedPresets"].as_array().map(Vec::len), Some(3));
    }

    // Beside the selection unless told to look for an empty region.
    for (options, reason) in [
        (json!({}), "selection_anchor"),
        (json!({ "anchor": "empty_region" }), "scan_fit"),
    ] {
        let mut fresh = Scripted::empty();
        let marker =
            fresh.apply(json!({ "entities": [{ "kind": "text", "text": "marker" }] }))["created"]
                [0]
            .clone();
        fresh.app.select(&[marker.as_str().unwrap_or_default()]);
        let response = fresh.post(
            "/tasks/apply",
            json!({ "taskKind": "breakpoint_map", "input": { "url": URL }, "options": options }),
        );
        assert_eq!(response.body["placement"]["reason"], reason, "{options}");
    }

    // Focus selects the new group unless it is told not to.
    let focused = post(&mut session, json!({ "url": "https://focus.example/" }));
    assert_eq!(
        session.app.selected_ids(),
        [focused["groupId"].as_str().unwrap_or_default()]
    );
    session.app.select(&[]);
    let response = session.post(
        "/tasks/apply",
        json!({ "taskKind": "breakpoint_map", "input": { "url": "https://quiet.example/" }, "options": { "focus": false } }),
    );
    assert_eq!(response.status, 200);
    assert!(session.app.selected_ids().is_empty());

    for (body, error) in [
        (json!({ "input": {} }), "input.url is required"),
        (
            json!({ "input": { "url": URL, "presets": "Desktop" } }),
            "input.presets: expected an array of labels",
        ),
        (
            json!({ "input": { "url": URL, "presets": ["iPad"] } }),
            "Unknown preset label: iPad",
        ),
        (
            json!({ "input": { "url": URL, "presets": [4] } }),
            "input.presets: expected an array of labels",
        ),
    ] {
        let mut body = body;
        body["taskKind"] = json!("breakpoint_map");
        let refused = session.post("/tasks/apply", body.clone());
        assert_eq!(refused.body, json!({ "error": error }), "{body}");
    }
}
