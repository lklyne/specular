//! Loads and saves every `.canvas` file checked into the repository and
//! compares JSON values: the integration-test snapshots, the starter space,
//! the benchmark fixtures and this crate's own fixtures.
#![expect(
    clippy::unwrap_used,
    reason = "shared test helpers panic to fail the test on an unreadable file"
)]

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specular_doc::Document;

/// Directories searched, relative to the repository root.
const ROOTS: [&str; 4] = [
    "tests/integration",
    "resources/starter-space",
    "native/fixtures",
    "native/crates",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "canvas") {
            found.push(path);
        }
    }
}

fn canvases() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in ROOTS {
        collect(&repo_root().join(root), &mut found);
    }
    found.sort();
    found
}

/// What a save writes for `file`: the file itself, plus the stack order
/// when it has none (nodes back-to-front, then edges).
fn expected(mut file: Value) -> Value {
    let ids = file["nodes"].as_array().unwrap().iter();
    let ids = ids.chain(file["edges"].as_array().unwrap());
    let order: Vec<Value> = ids.map(|item| item["id"].clone()).collect();
    if file["specular"]["entityOrder"].is_null() && !order.is_empty() {
        file["specular"] = json!({ "entityOrder": order });
    }
    file
}

#[test]
fn every_canvas_in_the_repo_loads_and_saves_to_the_same_json_value() {
    let files = canvases();
    for name in [
        "rich-workspace.canvas",
        "Welcome.canvas",
        "static-9.canvas",
        "kitchen-sink.canvas",
    ] {
        assert!(
            files.iter().any(|path| path.ends_with(name)),
            "{name} was not found"
        );
    }
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        let file: Value = serde_json::from_str(&text).unwrap();
        let document = Document::from_canvas_str(&text).unwrap();

        let nodes = file["nodes"].as_array().unwrap().len();
        assert_eq!(document.entities().count(), nodes, "{}", path.display());
        assert!(
            !document.extra().contains_key("nodes"),
            "{}",
            path.display()
        );

        let first = document.to_canvas_string().unwrap();
        let saved: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(saved, expected(file), "{}", path.display());

        let second = Document::from_canvas_str(&first).unwrap();
        assert_eq!(second, document, "{}", path.display());
        assert_eq!(
            second.to_canvas_string().unwrap(),
            first,
            "{}",
            path.display()
        );
    }
}

/// Files already in the writer's canonical form: every key in the Electron
/// writer's order. The Electron app wrote the first three. `Welcome.canvas`
/// has `"syncId": null` nodes, which must stay in the `syncId` slot.
/// `kitchen-sink.canvas` has every kind in every style, annotations included.
const CANONICAL: [&str; 4] = [
    "resources/starter-space/Welcome.canvas",
    "tests/integration/__snapshots__/rich-workspace.canvas",
    "native/crates/specular-doc/tests/fixtures/rich-workspace.canvas",
    "native/fixtures/kitchen-sink.canvas",
];

#[test]
fn a_canonical_file_loads_and_saves_to_the_same_bytes() {
    for name in CANONICAL {
        let text = std::fs::read_to_string(repo_root().join(name)).unwrap();
        let document = Document::from_canvas_str(&text).unwrap();
        assert_eq!(document.to_canvas_string().unwrap(), text, "{name}");
    }
}
