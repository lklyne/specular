//! Files dropped on the canvas.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{Kind, Rect};
use specular_interact::{DroppedFile, Effect, Event};
use specular_testkit::{TestApp, assert_doc_snapshot};

fn outside(path: &str, image_size: Option<(u32, u32)>) -> DroppedFile {
    DroppedFile {
        path: path.to_owned(),
        space_path: None,
        image_size,
    }
}

fn drop_at(app: &mut TestApp, at: (f32, f32), files: Vec<DroppedFile>) {
    app.take_effects();
    app.send(Event::FilesDropped {
        files,
        screen: Some(at.into()),
    });
}

fn file_of(app: &TestApp, id: &str) -> String {
    match &app.entity(id).kind {
        Kind::File(file) => file.file.clone(),
        other => panic!("{id} is not a file: {other:?}"),
    }
}

#[test]
fn an_image_from_outside_the_space_is_copied_into_assets_and_shown_at_its_size() {
    let mut app = TestApp::empty();
    drop_at(
        &mut app,
        (207.0, 93.0),
        vec![outside("/Users/me/Desktop/Shot.PNG", Some((800, 600)))],
    );
    let id = app
        .selected()
        .expect("the dropped file is selected")
        .to_owned();
    let file = format!("assets/{id}.png");
    assert_eq!(file_of(&app, &id), file);
    assert_eq!(app.rect(&id), Rect::new(200.0, 100.0, 800.0, 600.0));
    let effects = app.take_effects();
    let copied = effects.iter().position(|effect| {
        *effect
            == Effect::CopyAsset {
                from: "/Users/me/Desktop/Shot.PNG".to_owned(),
                file: file.clone(),
            }
    });
    let loaded = effects
        .iter()
        .position(|effect| matches!(effect, Effect::LoadImage { file: from, .. } if *from == file));
    assert!(copied.is_some() && copied < loaded, "{effects:?}");
    app.assert_undo_returns_to_start();
}

#[test]
fn a_file_inside_the_space_is_shown_where_it_is() {
    let mut app = TestApp::empty();
    let inside = DroppedFile {
        path: "/Users/me/Space/notes/Plan.md".to_owned(),
        space_path: Some("notes/Plan.md".to_owned()),
        image_size: None,
    };
    drop_at(&mut app, (0.0, 0.0), vec![inside]);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"e220a8397b1dcdaf","type":"file","x":0,"y":0,"width":400,"height":400,"file":"notes/Plan.md"}
    edges:
    specular: {"entityOrder":["e220a8397b1dcdaf"]}
    "#);
    let effects = app.take_effects();
    assert!(
        !(effects.iter()).any(|effect| matches!(effect, Effect::CopyAsset { .. })),
        "{effects:?}"
    );
    assert!(effects.contains(&Effect::LoadNote {
        file: "notes/Plan.md".to_owned()
    }));
}

#[test]
fn several_files_cascade_from_the_drop_point_as_one_undo_step() {
    let mut app = TestApp::empty();
    drop_at(
        &mut app,
        (100.0, 100.0),
        vec![
            outside("/tmp/a.jpg", Some((100, 50))),
            outside("/tmp/archive.zip", None),
            outside("/tmp/b.md", None),
            outside("/tmp/c.webp", None),
        ],
    );
    let rects: Vec<Rect> = (app.document().entities())
        .map(|entity| entity.rect)
        .collect();
    assert_eq!(
        rects,
        [
            Rect::new(100.0, 100.0, 100.0, 50.0),
            Rect::new(120.0, 120.0, 400.0, 400.0),
            // An image whose size could not be read.
            Rect::new(140.0, 140.0, 300.0, 300.0),
        ]
    );
    assert_eq!(app.selected_ids().len(), 3);
    app.undo();
    assert_eq!(app.document().entities().count(), 0);
}

#[test]
fn files_the_canvas_cannot_show_are_ignored() {
    let mut app = TestApp::empty();
    drop_at(
        &mut app,
        (0.0, 0.0),
        vec![outside("/tmp/clip.mp4", None), outside("/tmp/README", None)],
    );
    assert_eq!(app.document().entities().count(), 0);
    assert_eq!(app.take_effects(), []);
}
