//! `repos.json`, the file the Electron app also keeps, and its one rule: an
//! origin writes to one repo at a time.

use specular_agent::Repos;

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
