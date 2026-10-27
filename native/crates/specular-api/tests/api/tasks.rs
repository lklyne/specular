//! `POST /tasks/apply`: how the `breakpoints` verb makes a typed URL whole.
//! The verb itself runs end to end in `fixtures/scenarios/cli`.

use serde_json::json;
use specular_doc::{EntityId, Kind};

use crate::common::Scripted;

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
    let url = match document.entity(&EntityId::from(page)).map(|e| &e.kind) {
        Some(Kind::Page(page)) => page.url.clone(),
        _ => String::new(),
    };
    let title = document
        .entity(&EntityId::from(group))
        .and_then(|e| e.label.clone());
    Ok((url, title.unwrap_or_default()))
}

#[test]
#[expect(clippy::too_many_lines, reason = "one table of addresses")]
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
