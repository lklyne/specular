//! `add note` and `add file`: a text create that is really a Document, and
//! a file create that points into the space folder.

use serde_json::{Value, json};
use specular_interact::{DroppedFile, Effect};

use crate::common::Scripted;

fn writes(session: &Scripted) -> Vec<(String, String)> {
    (session.effects.iter())
        .filter_map(|effect| match effect {
            Effect::WriteNote { file, text } => Some((file.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

fn copies(session: &Scripted) -> Vec<(String, String)> {
    (session.effects.iter())
        .filter_map(|effect| match effect {
            Effect::CopyAsset { from, file } => Some((from.clone(), file.clone())),
            _ => None,
        })
        .collect()
}

fn size(node: &Value) -> (Value, Value) {
    (node["width"].clone(), node["height"].clone())
}

#[test]
fn a_heading_makes_a_document_named_after_it_with_the_text_written() {
    let mut session = Scripted::empty();
    let text = "# Launch plan: v2\n\nship it\n";
    let made = session.apply(json!({ "entities": [
        { "kind": "text", "text": text, "canvasX": 40, "canvasY": 80 },
    ]}));
    let node = session.node(&made["created"][0]);
    assert_eq!(node["type"], "file");
    assert_eq!(node["file"], "Launch plan- v2.md");
    assert_eq!(size(&node), (json!(400), json!(400)));
    assert_eq!(
        (node["x"].clone(), node["y"].clone()),
        (json!(40), json!(80))
    );
    assert_eq!(
        writes(&session),
        [("Launch plan- v2.md".to_owned(), text.to_owned())]
    );
    session.app.assert_undo_returns_to_start();
}

#[test]
fn long_text_is_named_from_its_first_line_cut_to_sixty_characters() {
    let mut session = Scripted::empty();
    let first = "abcdefghij".repeat(8);
    let text = format!("{first}\n{}", "more ".repeat(60));
    let made = session.apply(json!({ "entities": [
        { "kind": "note", "text": text, "width": 300, "height": 500 },
    ]}));
    let node = session.node(&made["created"][0]);
    assert_eq!(node["file"], json!(format!("{}.md", &first[..60])));
    assert_eq!(size(&node), (json!(300), json!(500)));
    session.app.assert_undo_returns_to_start();
}

#[test]
fn short_plain_text_stays_a_sticky_and_writes_nothing() {
    let mut session = Scripted::empty();
    let made = session.apply(json!({ "entities": [{ "kind": "text", "text": "remember milk" }] }));
    assert_eq!(session.node(&made["created"][0])["type"], "text");
    assert_eq!(writes(&session), Vec::<(String, String)>::new());
    let pinned = session.apply(json!({ "entities": [
        { "kind": "text", "text": "# not a note", "forceKind": true },
    ]}));
    assert_eq!(session.node(&pinned["created"][0])["type"], "text");
    let forced = session.apply(json!({ "entities": [
        { "kind": "text", "text": "tiny", "_forceFile": true },
    ]}));
    assert_eq!(session.node(&forced["created"][0])["file"], "tiny.md");
}

#[test]
fn a_name_that_is_taken_gets_a_number_even_within_one_patch() {
    let mut session = Scripted::empty();
    session.disk.entries = vec!["plan.MD".to_owned(), "Plan 3.md".to_owned()];
    let made = session.apply(json!({ "entities": [
        { "kind": "text", "text": "# Plan\none" },
        { "kind": "text", "text": "# Plan\ntwo" },
    ]}));
    let ids = made["created"].as_array().cloned().unwrap_or_default();
    let files: Vec<_> = ids
        .iter()
        .map(|id| session.node(id)["file"].clone())
        .collect();
    assert_eq!(files, [json!("Plan 2.md"), json!("Plan 4.md")]);
    assert_eq!(writes(&session).len(), 2);
    session.app.assert_undo_returns_to_start();
}

fn dropped(path: &str, inside: Option<&str>, image: Option<(u32, u32)>) -> DroppedFile {
    DroppedFile {
        path: path.to_owned(),
        space_path: inside.map(str::to_owned),
        image_size: image,
    }
}

#[test]
fn a_file_inside_the_space_keeps_its_relative_path_and_is_not_copied() {
    let mut session = Scripted::empty();
    let file = dropped("/space/shots/a.png", Some("shots/a.png"), Some((640, 480)));
    session
        .disk
        .files
        .insert("/space/shots/a.png".to_owned(), file);
    let made =
        session.apply(json!({ "entities": [{ "kind": "file", "file": "/space/shots/a.png" }] }));
    let node = session.node(&made["created"][0]);
    assert_eq!(node["file"], "shots/a.png");
    assert_eq!(size(&node), (json!(640), json!(480)));
    assert_eq!(copies(&session), Vec::<(String, String)>::new());
    session.app.assert_undo_returns_to_start();
}

#[test]
fn a_file_outside_the_space_is_copied_into_assets_and_sized_from_the_image() {
    let mut session = Scripted::empty();
    let files = &mut session.disk.files;
    files.insert(
        "/tmp/away.PNG".to_owned(),
        dropped("/tmp/away.PNG", None, Some((120, 80))),
    );
    files.insert(
        "/tmp/notes.txt".to_owned(),
        dropped("/tmp/notes.txt", None, None),
    );
    let made = session.apply(json!({ "entities": [
        { "kind": "file", "file": "/tmp/away.PNG" },
        { "kind": "file", "file": "/tmp/notes.txt", "width": 100, "height": 50 },
    ]}));
    let image = session.node(&made["created"][0]);
    let text = session.node(&made["created"][1]);
    let id = made["created"][0].as_str().unwrap_or_default();
    assert_eq!(image["file"], json!(format!("assets/{id}.png")));
    assert_eq!(size(&image), (json!(120), json!(80)));
    assert_eq!(size(&text), (json!(100), json!(50)));
    let from_to = |from: &str, node: &Value| {
        (
            from.to_owned(),
            node["file"].as_str().unwrap_or_default().to_owned(),
        )
    };
    assert_eq!(
        copies(&session),
        [
            from_to("/tmp/away.PNG", &image),
            from_to("/tmp/notes.txt", &text)
        ]
    );
    session.app.assert_undo_returns_to_start();
}

#[test]
fn a_file_that_does_not_exist_is_a_400_naming_the_path() {
    let mut session = Scripted::empty();
    let response = session.post(
        "/canvas/apply",
        json!({ "entities": [{ "kind": "file", "file": "nowhere/ghost.png" }] }),
    );
    assert_eq!(response.status, 400);
    assert!(
        response.body.to_string().contains("nowhere/ghost.png"),
        "{}",
        response.body
    );
    assert!(
        session.canvas()["nodes"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
}

#[test]
fn what_makes_text_a_document_and_how_the_file_is_named() {
    let long = |count: usize| "a".repeat(count);
    let rows = [
        ("exactly 300 characters stays a sticky", long(300), false),
        ("301 characters is a document", long(301), true),
        ("counted in characters, not bytes", "é".repeat(160), false),
        ("a code fence", "```\nx\n```".to_owned(), true),
        ("a table row", "| a | b |".to_owned(), true),
        ("a lone pipe is no table", "|abc".to_owned(), false),
        ("two pipes in a row are no table", "||abc".to_owned(), false),
        ("a six-level heading", "###### six".to_owned(), true),
        (
            "seven hashes are no heading",
            "####### seven".to_owned(),
            false,
        ),
        (
            "a hash with no space is no heading",
            "#tag".to_owned(),
            false,
        ),
    ];
    for (row, text, document) in rows {
        let mut session = Scripted::empty();
        let made = session.apply(json!({ "entities": [{ "kind": "text", "text": text }] }));
        let kind = session.node(&made["created"][0])["type"].clone();
        assert_eq!(kind, if document { "file" } else { "text" }, "{row}");
    }

    // An existing text stays text whatever it says now.
    let mut session = Scripted::empty();
    let made = session.apply(json!({ "entities": [{ "kind": "text", "text": "plain" }] }));
    let id = made["created"][0].clone();
    session.apply(json!({ "entities": [{ "id": id, "kind": "text", "text": "# now a heading" }] }));
    assert_eq!(session.node(&id)["type"], "text");

    let names = [
        ("# Title\nbody", "Title.md"),
        ("intro line\n## Deeper\nbody", "Deeper.md"),
        ("#\n# \nfirst line is used\n```", "#.md"),
        ("...\n```", "Note.md"),
        ("*?\"<>|\\/\n```", "--------.md"),
        ("  ..dotted..  \n```", "dotted.md"),
    ];
    for (text, file) in names {
        let mut session = Scripted::empty();
        let made = session.apply(json!({ "entities": [{ "kind": "text", "text": text }] }));
        assert_eq!(session.node(&made["created"][0])["file"], file, "{text}");
    }
    // A stray `file` on a non-file, and a `file` on an update, are not looked up on disk.
    let mut session = Scripted::empty();
    session.disk.files.insert(
        "known.md".to_owned(),
        dropped("known.md", Some("known.md"), None),
    );
    let made = session.apply(json!({ "entities": [
        { "kind": "text", "text": "odd", "file": "ghost.png" },
        { "kind": "file", "file": "known.md" },
    ]}));
    assert_eq!(made["created"].as_array().map(Vec::len), Some(2));
    let known = made["created"][1].clone();
    session.apply(json!({ "entities": [{ "id": known, "file": "other.md" }] }));
    assert_eq!(session.node(&known)["file"], "other.md");
}
