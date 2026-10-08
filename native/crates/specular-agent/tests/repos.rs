//! `repos.json`, the file the Electron app also keeps, and its one rule: an
//! origin writes to one repo at a time.

use specular_agent::{Repos, origin_of};

const FILE: &str = r#"{
  "repos": [
    {
      "id": "r1",
      "absolutePath": "/scratch/site",
      "label": "Site",
      "boundOrigins": [
        {
          "origin": "http://localhost:3000",
          "autoFix": true
        },
        {
          "origin": "https://example.com",
          "autoFix": false
        }
      ]
    },
    {
      "id": "r2",
      "absolutePath": "/scratch/docs",
      "label": "docs"
    }
  ]
}"#;

#[test]
fn repos_json_round_trips_byte_for_byte_and_drops_junk() {
    let repos = Repos::from_json(FILE);
    assert_eq!(repos.to_json(), FILE);
    assert_eq!(
        repos
            .binding("http://localhost:3000/anything")
            .map(|b| (b.repo_path, b.auto_fix)),
        Some(("/scratch/site", true))
    );

    let messy = r#"{"repos":[
        {"id":"","absolutePath":"/x"},
        {"id":"a","absolutePath":"/scratch/site/","boundOrigins":[
            {"origin":"HTTP://Localhost:80/path"},{"origin":7}]},
        "junk"]}"#;
    let repos = Repos::from_json(messy);
    assert_eq!(repos.all().len(), 1);
    assert_eq!(repos.all()[0].label, "site");
    assert_eq!(repos.all()[0].bound_origins.len(), 1);
    assert_eq!(repos.all()[0].bound_origins[0].origin, "http://localhost");
    assert!(
        !repos.all()[0].bound_origins[0].auto_fix,
        "auto-fix is off unless the file says so"
    );
    assert_eq!(Repos::from_json("not json"), Repos::default());
}

#[test]
fn an_origin_is_bound_to_one_repo_and_rebinding_moves_it_with_auto_off() {
    let mut repos = Repos::default();
    repos.bind("http://localhost:3000/a", "/scratch/site");
    assert!(repos.set_auto_fix("http://localhost:3000", true));
    repos.bind("http://localhost:3000", "/scratch/other");
    let bound: Vec<(&str, usize)> = repos
        .all()
        .iter()
        .map(|r| (r.absolute_path.as_str(), r.bound_origins.len()))
        .collect();
    assert_eq!(bound, [("/scratch/site", 0), ("/scratch/other", 1)]);
    let binding = repos
        .binding("http://localhost:3000")
        .map(|b| (b.repo_path, b.auto_fix));
    assert_eq!(binding, Some(("/scratch/other", false)));

    repos.bind("http://localhost:3000", "/scratch/other");
    assert_eq!(
        repos.all().len(),
        2,
        "binding to a connected folder adds no repo"
    );
    let id = repos.connect("/scratch/site");
    assert_eq!(
        id, "30613715f17b86a6",
        "the first 16 hex digits of the path's SHA-256"
    );
    repos.bind("http://localhost:4000", "/scratch/other");
    assert!(repos.set_auto_fix("http://localhost:3000", true));
    assert_eq!(
        repos.binding("http://localhost:4000").map(|b| b.auto_fix),
        Some(false),
        "auto-fix is per origin"
    );
    assert!(repos.unbind("http://localhost:3000"));
    assert!(repos.binding("http://localhost:3000").is_none());
    assert!(
        repos.binding("http://localhost:4000").is_some(),
        "unbinding one origin keeps the others"
    );
}

#[test]
fn origin_of_keeps_scheme_host_and_a_port_that_is_not_the_default() {
    let rows = [
        ("http://localhost:3000/a?b#c", Some("http://localhost:3000")),
        ("https://Example.com:443/x", Some("https://example.com")),
        ("http://example.com:80", Some("http://example.com")),
        (
            "https://user@example.com:8443/",
            Some("https://example.com:8443"),
        ),
        ("http://example.com#top", Some("http://example.com")),
        ("ws://example.com:80/s", Some("ws://example.com")),
        ("wss://example.com:443/s", Some("wss://example.com")),
        ("http://example.com:/x", Some("http://example.com")),
        ("  http://example.com/x  ", Some("http://example.com")),
        ("http:///path", None),
        ("data:text/html,hi", None),
        ("file:///a/b.html", None),
        ("about:blank", None),
        ("not a url", None),
    ];
    for (url, want) in rows {
        assert_eq!(origin_of(url).as_deref(), want, "{url}");
    }
}
