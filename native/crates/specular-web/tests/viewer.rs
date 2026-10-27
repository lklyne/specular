//! The web viewer: what a viewer's input may do to a published canvas, and
//! what the bundle answers for its pages and comments.
#![expect(
    clippy::panic,
    reason = "a helper panics to fail the test it is called from"
)]

use glam::Vec2;
use specular_core::{Modifiers, PageId, PointerButton, PointerEventKind};
use specular_doc::{Annotation, AnnotationAnchor, AnnotationId, Document, Rect};
use specular_interact::NoteState;
use specular_testkit::{document, note, page, sticky};
use specular_web::{Access, Mode, Upload, Viewer, key_input};

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};

/// A page `p1` at (100, 100) and a sticky `s1` at (700, 100), published.
fn canvas() -> String {
    let document = document([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        sticky("s1", Rect::new(700.0, 100.0, 200.0, 200.0), "ship it"),
    ]);
    published(&document)
}

#[track_caller]
fn published(document: &Document) -> String {
    match document.to_canvas_string() {
        Ok(text) => text,
        Err(error) => panic!("the canvas could not be written: {error:?}"),
    }
}

#[track_caller]
fn open(canvas: &str, access: Access, seed: u64) -> Viewer {
    match Viewer::new(canvas, access, seed) {
        Ok(viewer) => viewer,
        Err(error) => panic!("{error}"),
    }
}

fn viewer(access: Access) -> Viewer {
    let mut viewer = open(&canvas(), access, 7);
    viewer.tick(1_761_500_000_000);
    viewer.resize(Vec2::new(1200.0, 800.0));
    viewer
}

/// Where the canvas point `world` is on screen now.
fn screen(viewer: &Viewer, world: (f32, f32)) -> Vec2 {
    viewer
        .app()
        .session()
        .camera
        .world_to_screen(Vec2::new(world.0, world.1))
}

fn press(viewer: &mut Viewer, at: Vec2) {
    let kind = PointerEventKind::Down {
        button: PointerButton::Left,
        click_count: 1,
    };
    viewer.pointer(kind, at, NONE);
}

fn release(viewer: &mut Viewer, at: Vec2) {
    let kind = PointerEventKind::Up {
        button: PointerButton::Left,
        click_count: 1,
    };
    viewer.pointer(kind, at, NONE);
}

fn drag(viewer: &mut Viewer, from: Vec2, to: Vec2) {
    press(viewer, from);
    viewer.pointer(PointerEventKind::Move, (from + to) / 2.0, NONE);
    viewer.pointer(PointerEventKind::Move, to, NONE);
    release(viewer, to);
}

fn click(viewer: &mut Viewer, at: Vec2) {
    viewer.pointer(PointerEventKind::Move, at, NONE);
    press(viewer, at);
    release(viewer, at);
}

fn key(viewer: &mut Viewer, code: &str, key: &str, modifiers: Modifiers) -> bool {
    let used = viewer.key(key_input(code, key, true, false, modifiers));
    viewer.key(key_input(code, key, false, false, modifiers));
    used
}

fn type_text(viewer: &mut Viewer, text: &str) {
    for character in text.chars() {
        let code = match character {
            ' ' => "Space".to_owned(),
            letter => format!("Key{}", letter.to_ascii_uppercase()),
        };
        key(viewer, &code, &character.to_string(), NONE);
    }
}

fn rect_of(viewer: &Viewer, id: &str) -> Option<Rect> {
    let entity = viewer
        .app()
        .document()
        .entity(&specular_doc::EntityId::new(id))?;
    Some(entity.rect)
}

/// A comment from the server on the canvas point `at`.
fn remote_comment(id: &str, at: (f64, f64), text: &str) -> Annotation {
    let mut comment = Annotation::new(
        AnnotationId::new(id),
        AnnotationAnchor::Canvas {
            canvas_x: at.0,
            canvas_y: at.1,
        },
        "2026-10-27T01:00:00.000Z".to_owned(),
    );
    text.clone_into(&mut comment.text);
    comment
}

#[test]
fn a_drag_in_pan_mode_moves_the_camera_and_leaves_the_canvas() {
    let mut viewer = viewer(Access::Comment);
    let before = viewer.app().session().camera;
    let on_sticky = screen(&viewer, (800.0, 200.0));
    drag(&mut viewer, on_sticky, on_sticky + Vec2::new(120.0, 40.0));
    let after = viewer.app().session().camera;
    assert_eq!(after.pan - before.pan, Vec2::new(120.0, 40.0));
    assert_eq!(
        rect_of(&viewer, "s1"),
        Some(Rect::new(700.0, 100.0, 200.0, 200.0))
    );
}

#[test]
fn keys_that_edit_do_nothing_and_zoom_keys_zoom() {
    let mut viewer = viewer(Access::Comment);
    let cmd = Modifiers { meta: true, ..NONE };
    let at = screen(&viewer, (800.0, 200.0));
    click(&mut viewer, at);
    for (code, text, modifiers) in [
        ("Backspace", "Backspace", NONE),
        ("Delete", "Delete", NONE),
        ("KeyX", "x", cmd),
        ("KeyD", "d", cmd),
    ] {
        key(&mut viewer, code, text, modifiers);
    }
    assert_eq!(viewer.app().document().entities().count(), 2);
    let zoom = viewer.app().session().camera.zoom;
    assert!(key(&mut viewer, "Equal", "=", cmd));
    assert!(viewer.app().session().camera.zoom > zoom);
}

#[test]
fn a_comment_written_in_comment_mode_goes_out_once() {
    let mut viewer = viewer(Access::Comment);
    assert!(viewer.set_mode(Mode::Comment));
    let at = screen(&viewer, (600.0, 600.0));
    click(&mut viewer, at);
    type_text(&mut viewer, "tighten this");
    key(&mut viewer, "Enter", "Enter", NONE);
    let sent = viewer.take_new_comments();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].text, "tighten this");
    assert_eq!(viewer.take_new_comments(), Vec::new());
    // The server lists it back: it is shown once, not twice.
    viewer.set_comments(sent.clone());
    assert_eq!(viewer.app().document().annotations(), sent.as_slice());
}

#[test]
fn a_view_link_cannot_comment() {
    let mut viewer = viewer(Access::View);
    assert!(!viewer.set_mode(Mode::Comment));
    assert_eq!(viewer.mode(), Mode::Pan);
}

#[test]
fn the_servers_comments_show_and_a_click_on_one_focuses_it() {
    let mut viewer = viewer(Access::Comment);
    viewer.set_comments(vec![remote_comment("a1", (600.0, 600.0), "why blue?")]);
    assert_eq!(viewer.app().document().annotations().len(), 1);
    let mark = viewer.app().comment_marks();
    let specular_interact::MarkShape::Badge(pill) = mark[0].shape else {
        panic!("a point comment is a pill");
    };
    click(&mut viewer, pill.min + pill.size / 2.0);
    assert_eq!(
        viewer.focused_comment().map(AnnotationId::as_str),
        Some("a1")
    );
    // Keys that would resolve or delete it are not passed on.
    key(&mut viewer, "Backspace", "Backspace", NONE);
    assert_eq!(viewer.app().document().annotations().len(), 1);
}

#[test]
fn the_servers_comments_wait_while_one_is_being_written() {
    let mut viewer = viewer(Access::Comment);
    viewer.set_mode(Mode::Comment);
    let at = screen(&viewer, (600.0, 600.0));
    click(&mut viewer, at);
    type_text(&mut viewer, "half");
    viewer.set_comments(vec![remote_comment("a1", (300.0, 600.0), "from Dana")]);
    assert!(viewer.app().comment_draft().is_some(), "the draft is kept");
    assert_eq!(viewer.app().document().annotations(), &[]);
    key(&mut viewer, "Escape", "Escape", NONE);
    viewer.tick(1_761_500_000_100);
    assert_eq!(viewer.app().document().annotations().len(), 1);
}

#[test]
fn a_page_shows_its_snapshot_once_it_arrives() {
    let mut viewer = viewer(Access::Comment);
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(8, 6, image::Rgba([255, 0, 0, 128]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    viewer.add_snapshot("p1", png);
    let page = viewer.page_of(&specular_doc::EntityId::new("p1"));
    assert_eq!(page, Some(PageId(1)));
    let uploads = viewer.take_uploads();
    let frames: Vec<_> = (uploads.iter())
        .filter_map(|upload| match upload {
            Upload::PageFrame {
                page,
                frame,
                viewport,
            } => Some((*page, frame, *viewport)),
            _ => None,
        })
        .collect();
    assert_eq!(frames.len(), 1);
    let (shown, frame, viewport) = frames[0];
    assert_eq!(shown, PageId(1));
    assert_eq!(
        (frame.size.width, frame.size.height, frame.stride),
        (8, 6, 32)
    );
    assert_eq!(&frame.bgra[..4], &[0, 0, 128, 128], "premultiplied BGRA");
    assert_eq!((viewport.width, viewport.height), (400, 300));
}

#[test]
fn a_published_document_canvas_reads_its_markdown() {
    let document: Document =
        document([note("n1", Rect::new(100.0, 500.0, 300.0, 200.0), "plan.md")]);
    let mut viewer = open(&published(&document), Access::Comment, 1);
    viewer.add_file("plan.md", b"# Plan\n\nShip on Friday.".to_vec());
    let ready = matches!(
        viewer.app().note("plan.md"),
        Some(NoteState::Ready(text)) if **text == *"# Plan\n\nShip on Friday."
    );
    assert!(ready, "{:?}", viewer.app().note("plan.md"));
}

#[test]
fn the_canvas_keeps_the_comments_it_was_published_with() {
    let mut document = document([page("p1", Rect::new(100.0, 100.0, 400.0, 300.0))]);
    let mine = remote_comment("d1", (50.0, 50.0), "from the desktop");
    let _ = document.apply(specular_doc::Command::InsertAnnotation {
        annotation: Box::new(mine.clone()),
        at: 0,
    });
    let mut viewer = open(&published(&document), Access::Comment, 1);
    let theirs = remote_comment("a1", (600.0, 600.0), "from Dana");
    viewer.set_comments(vec![theirs.clone()]);
    assert_eq!(
        viewer.app().document().annotations(),
        &[mine.clone(), theirs.clone()]
    );
    // The server's copy of a published comment (resolved there) wins.
    let resolved = Annotation {
        status: specular_doc::AnnotationStatus::Resolved,
        ..mine
    };
    viewer.set_comments(vec![resolved.clone(), theirs.clone()]);
    assert_eq!(viewer.app().document().annotations(), &[resolved, theirs]);
}
