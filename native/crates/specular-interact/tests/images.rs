//! Image files: `update` asks the shell for each one, and keeps what the
//! shell answers.

use specular_doc::{Document, Entity, FileRef, Kind, Rect};
use specular_interact::{Effect, Event, ImageKey, ImageNotice, ImageState};
use specular_testkit::{TestApp, document, file, page};

const BOX: Rect = Rect::new(100.0, 100.0, 240.0, 160.0);

fn file_at(id: &str, path: &str) -> Entity {
    let file = FileRef {
        file: path.to_owned(),
        ..FileRef::default()
    };
    Entity::new(id, BOX, Kind::File(file))
}

fn opened(document: Document) -> TestApp {
    let mut app = TestApp::empty();
    app.open(document);
    app
}

/// The image effects in `effects`: `(key, file)` for a load, `(key, "")`
/// for a drop.
fn image_effects(effects: &[Effect]) -> Vec<(u64, &str)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::LoadImage { image, file } => Some((image.0, file.as_str())),
            Effect::DropImage(image) => Some((image.0, "")),
            _ => None,
        })
        .collect()
}

fn state(app: &TestApp, file: &str) -> Option<ImageState> {
    app.app().image(file).map(|image| image.state)
}

#[test]
fn opening_a_document_asks_for_each_image_file_once() {
    let mut app = opened(document([
        file_at("a", "assets/shot.png"),
        file_at("b", "assets/shot.png"),
        file_at("c", "/abs/photo.JPG"),
        file_at("d", "notes.md"),
        page("p1", BOX),
    ]));
    assert_eq!(
        image_effects(&app.take_effects()),
        [(0, "assets/shot.png"), (1, "/abs/photo.JPG")]
    );
    assert_eq!(state(&app, "assets/shot.png"), Some(ImageState::Loading));
    assert_eq!(state(&app, "notes.md"), None);
}

#[test]
fn the_shells_answer_is_kept_per_image() {
    let mut app = opened(document([
        file("ready", BOX),
        file("missing", BOX),
        file("failed", BOX),
        file("slow", BOX),
    ]));
    let key = |file: &str| app.app().image(file).map(|image| image.key).unwrap();
    let (ready, missing, failed) = (key("ready.png"), key("missing.png"), key("failed.png"));
    let notice = ImageNotice::Ready {
        width: 640,
        height: 480,
    };
    app.send(Event::Image {
        image: ready,
        notice,
    })
    .send(Event::Image {
        image: missing,
        notice: ImageNotice::Missing,
    })
    .send(Event::Image {
        image: failed,
        notice: ImageNotice::Failed,
    });
    assert_eq!(
        [
            state(&app, "ready.png"),
            state(&app, "missing.png"),
            state(&app, "failed.png"),
            state(&app, "slow.png"),
        ]
        .map(Option::unwrap),
        [
            ImageState::Ready {
                width: 640,
                height: 480
            },
            ImageState::Missing,
            ImageState::Failed,
            ImageState::Loading,
        ]
    );
}

#[test]
fn an_answer_for_an_image_no_longer_held_is_ignored() {
    let mut app = opened(document([file("a", BOX)]));
    let before = app.session().clone();
    app.send(Event::Image {
        image: ImageKey(99),
        notice: ImageNotice::Failed,
    });
    assert_eq!(app.session(), &before);
}

#[test]
fn opening_another_document_drops_what_it_does_not_show_and_keeps_the_rest() {
    let mut app = opened(document([file("a", BOX), file("b", BOX)]));
    app.take_effects();
    app.open(document([file("b", BOX), file("c", BOX)]));
    // `a` (key 0) goes, `b` (key 1) is not loaded again, `c` is new.
    assert_eq!(image_effects(&app.take_effects()), [(0, ""), (2, "c.png")]);
    assert_eq!(state(&app, "a.png"), None);
}
