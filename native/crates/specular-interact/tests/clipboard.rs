//! Copy, cut and paste: what goes on the clipboard, and what a paste makes
//! of what comes back.

#![expect(
    clippy::expect_used,
    reason = "a helper fails the test it is called from"
)]

use specular_doc::{Kind, Rect};
use specular_interact::{
    Action, AssetBytes, ClipboardContent, ClipboardImage, Effect, Event, Key, Paste,
};
use specular_testkit::{
    CMD, TestApp, assert_doc_snapshot, connected, document, group, inside, shape,
};

fn text(text: &str) -> ClipboardContent {
    ClipboardContent {
        text: Some(text.to_owned()),
        image: None,
    }
}

fn image() -> ClipboardImage {
    ClipboardImage {
        width: 640,
        height: 480,
        png: AssetBytes::from(vec![0x89, b'P', b'N', b'G']),
    }
}

/// Two shapes with an edge between them, and a third on its own.
fn three_shapes() -> TestApp {
    let entities = [
        shape("a", Rect::new(0.0, 0.0, 100.0, 100.0)),
        shape("b", Rect::new(200.0, 0.0, 100.0, 100.0)),
        shape("c", Rect::new(0.0, 300.0, 100.0, 100.0)),
    ];
    TestApp::from_document(connected(document(entities), "e1", "a", "b"))
}

/// What a copy put on the clipboard.
fn copied(app: &mut TestApp) -> String {
    let written = app
        .take_effects()
        .into_iter()
        .find_map(|effect| match effect {
            Effect::WriteClipboard(text) => Some(text),
            _ => None,
        });
    written.expect("the copy wrote to the clipboard")
}

fn paste(app: &mut TestApp, content: ClipboardContent) {
    app.take_effects();
    app.send(Event::Clipboard(content));
}

#[test]
fn copy_writes_the_selection_and_leaves_the_document_alone() {
    let mut app = three_shapes();
    app.select(&["a", "b"]).take_effects();
    app.chord(CMD, Key::Char('c'));
    let effects = app.take_effects();
    let [Effect::WriteClipboard(text)] = effects.as_slice() else {
        panic!("expected one clipboard write, got {effects:?}");
    };
    assert_eq!(
        text,
        concat!(
            r#"specular:canvas:{"nodes":["#,
            r#"{"id":"a","type":"shape","x":0,"y":0,"width":100,"height":100,"shapeKind":"rectangle","text":""},"#,
            r#"{"id":"b","type":"shape","x":200,"y":0,"width":100,"height":100,"shapeKind":"rectangle","text":""}],"#,
            r#""edges":[{"id":"e1","fromNode":"a","toNode":"b"}],"#,
            r#""specular":{"entityOrder":["a","b","e1"]}}"#,
        )
    );
    assert_eq!(app.document().entities().count(), 3);
}

#[test]
fn copy_with_nothing_selected_writes_nothing() {
    let mut app = three_shapes();
    app.act(Action::Copy);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn paste_asks_the_shell_for_the_clipboard() {
    let mut app = three_shapes();
    app.take_effects();
    app.chord(CMD, Key::Char('v'));
    assert_eq!(app.take_effects(), [Effect::ReadClipboard]);
}

#[test]
fn pasted_copies_land_at_the_pointer_with_their_edge_and_are_selected() {
    let mut app = three_shapes();
    app.select(&["a", "b"]).act(Action::Copy);
    let clipboard = copied(&mut app);
    app.pointer_move((505.0, 395.0));
    paste(&mut app, text(&clipboard));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"a","type":"shape","x":0,"y":0,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"b","type":"shape","x":200,"y":0,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"c","type":"shape","x":0,"y":300,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"e220a8397b1dcdaf","type":"shape","x":500,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"6e789e6aa1b965f4","type":"shape","x":700,"y":400,"width":100,"height":100,"shapeKind":"rectangle","text":""}
    edges:
      {"id":"e1","fromNode":"a","toNode":"b"}
      {"id":"06c45d188009454f","fromNode":"e220a8397b1dcdaf","toNode":"6e789e6aa1b965f4"}
    specular: {"entityOrder":["a","b","c","e1","e220a8397b1dcdaf","6e789e6aa1b965f4","06c45d188009454f"]}
    "#);
    assert_eq!(app.selected_ids().len(), 2);
    assert!(!app.selected_ids().contains(&"a"));
    assert!(app.take_effects().contains(&Effect::Save));
    app.assert_undo_returns_to_start();
}

#[test]
fn cut_copies_then_removes_in_one_undo_step() {
    let mut app = three_shapes();
    app.select(&["a"]).take_effects();
    app.chord(CMD, Key::Char('x'));
    assert!(copied(&mut app).starts_with("specular:canvas:"));
    // The edge that lost an end goes with it.
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"b","type":"shape","x":200,"y":0,"width":100,"height":100,"shapeKind":"rectangle","text":""}
      {"id":"c","type":"shape","x":0,"y":300,"width":100,"height":100,"shapeKind":"rectangle","text":""}
    edges:
    specular: {"entityOrder":["b","c"]}
    "#);
    app.undo();
    assert_eq!(app.document().entities().count(), 3);
    assert_eq!(app.document().edges().count(), 1);
}

#[test]
fn a_copied_group_is_pasted_with_its_members_inside_the_copy() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 300.0, 300.0)),
        inside("g", shape("a", Rect::new(20.0, 20.0, 100.0, 100.0))),
    ]);
    app.select(&["g"]).act(Action::Copy);
    let clipboard = copied(&mut app);
    app.pointer_move((600.0, 0.0));
    paste(&mut app, text(&clipboard));
    let [pasted_group] = app.selected_ids()[..] else {
        panic!("only the group's copy is selected");
    };
    let pasted_group = pasted_group.to_owned();
    let member = (app.document().entities())
        .find(|entity| entity.id.as_str() != "a" && entity.parent.is_some())
        .expect("the member was pasted");
    assert_eq!(
        member.parent.as_ref().map(specular_doc::EntityId::as_str),
        Some(pasted_group.as_str())
    );
    assert_eq!(member.rect, Rect::new(620.0, 20.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_member_copied_without_its_group_is_pasted_outside_it() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 300.0, 300.0)),
        inside("g", shape("a", Rect::new(20.0, 20.0, 100.0, 100.0))),
    ]);
    app.select(&["a"]).act(Action::Copy);
    let clipboard = copied(&mut app);
    paste(&mut app, text(&clipboard));
    let [pasted] = app.selected_ids()[..] else {
        panic!("the copy is selected");
    };
    assert_eq!(app.entity(pasted).parent, None);
}

#[test]
fn a_pasted_url_makes_a_desktop_page_and_hosts_it() {
    let mut app = TestApp::empty();
    app.pointer_move((95.0, 215.0));
    paste(&mut app, text(" example.com/docs \n"));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"e220a8397b1dcdaf","type":"link","x":100,"y":220,"width":1440,"height":900,"url":"https://example.com/docs","presetIndex":7,"source":"manual","metadata":{"createdFrom":"paste_url","deviceOrientation":"landscape","showDeviceFrame":true,"deviceId":"desktop"}}
    edges:
    specular: {"entityOrder":["e220a8397b1dcdaf"]}
    "#);
    let effects = app.take_effects();
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::CreatePage { url, .. } if url == "https://example.com/docs"
        )),
        "{effects:?}"
    );
    assert_eq!(app.selected_ids().len(), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn other_pasted_text_makes_a_sticky_with_the_sticky_defaults() {
    let mut app = TestApp::empty();
    app.pointer_move((95.0, 215.0));
    paste(&mut app, text("ship it\non Friday"));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"e220a8397b1dcdaf","type":"text","x":100,"y":220,"width":200,"height":200,"text":"ship it\non Friday","color":"3","specular":{"textStyle":"sticky","widthMode":"fixed","textSize":14,"textFont":"sans"}}
    edges:
    specular: {"entityOrder":["e220a8397b1dcdaf"]}
    "#);
    assert_eq!(app.session().editing, None);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_pasted_image_is_written_to_assets_before_it_is_loaded() {
    let mut app = TestApp::empty();
    app.pointer_move((95.0, 215.0));
    paste(
        &mut app,
        ClipboardContent {
            text: None,
            image: Some(image()),
        },
    );
    let id = app.selected().expect("the image is selected").to_owned();
    let file = format!("assets/{id}.png");
    let entity = app.entity(&id);
    assert_eq!(entity.rect, Rect::new(100.0, 220.0, 640.0, 480.0));
    assert!(matches!(&entity.kind, Kind::File(shown) if shown.file == file));
    let effects = app.take_effects();
    let written = effects.iter().position(
        |effect| matches!(effect, Effect::WriteAsset { file: to, bytes } if *to == file && bytes.as_slice().len() == 4),
    );
    let loaded = effects
        .iter()
        .position(|effect| matches!(effect, Effect::LoadImage { file: from, .. } if *from == file));
    assert!(written.is_some() && written < loaded, "{effects:?}");
    app.assert_undo_returns_to_start();
}

#[test]
fn an_empty_clipboard_pastes_nothing() {
    let mut app = TestApp::empty();
    paste(&mut app, ClipboardContent::default());
    paste(&mut app, text("  \n "));
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn a_paste_decides_items_then_image_then_url_then_text() {
    let mut app = three_shapes();
    app.select(&["c"]).act(Action::Copy);
    let items = copied(&mut app);
    let both = ClipboardContent {
        text: Some(items),
        image: Some(image()),
    };
    assert!(matches!(Paste::of(both), Paste::Items(_)));
    let image_and_url = ClipboardContent {
        text: Some("example.com".to_owned()),
        image: Some(image()),
    };
    assert_eq!(Paste::of(image_and_url), Paste::Image(image()));
    assert_eq!(
        Paste::of(text("localhost:4321/garden")),
        Paste::Page("http://localhost:4321/garden".to_owned())
    );
    // A URL on one of several lines is prose.
    assert_eq!(
        Paste::of(text("see\nexample.com")),
        Paste::Text("see\nexample.com".to_owned())
    );
    // The prefix with something else after it is text like any other.
    assert_eq!(
        Paste::of(text("specular:canvas:nope")),
        Paste::Text("specular:canvas:nope".to_owned())
    );
    assert_eq!(Paste::of(ClipboardContent::default()), Paste::Nothing);
}
