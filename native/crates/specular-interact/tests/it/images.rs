//! Image files: `update` asks the shell for each one, and keeps what the
//! shell answers.

use std::sync::Arc;

use glam::Vec2;
use specular_core::Camera;
use specular_doc::{Document, Entity, FileRef, Kind, Rect};
use specular_interact::{Action, Effect, Event, ImageNotice, ImageState};
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
fn opening_another_document_drops_what_it_does_not_show_and_keeps_the_rest() {
    let mut app = opened(document([file("a", BOX), file("b", BOX)]));
    app.take_effects();
    app.open(document([file("b", BOX), file("c", BOX)]));
    // `a` (key 0) goes, `b` (key 1) is not loaded again, `c` is new.
    assert_eq!(image_effects(&app.take_effects()), [(0, ""), (2, "c.png")]);
    assert_eq!(state(&app, "a.png"), None);
}

/// Answers the load of `file` with `notice`.
fn answer(app: &mut TestApp, file: &str, notice: ImageNotice) {
    let key = app.app().image(file).map(|image| image.key);
    assert!(key.is_some(), "{file} was asked for");
    let Some(key) = key else { return };
    app.send(Event::Image { image: key, notice });
}

/// `(width, height)` of each svg raster asked for in `effects`.
fn rasters(effects: &[Effect]) -> Vec<(u32, u32)> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::RasterImage { size, .. } => Some((size.width, size.height)),
            _ => None,
        })
        .collect()
}

const START: u64 = 1_700_000_000_000;

#[test]
fn an_svg_is_rastered_at_its_drawn_size_and_again_only_once_the_zoom_leaves_the_band() {
    let mut app = opened(document([file_at("s", "art.svg")]));
    app.viewport((1000.0, 800.0)).tick(START);
    // The 120 by 80 svg is contained in the 240 by 160 box: twice its size.
    app.take_effects();
    answer(
        &mut app,
        "art.svg",
        ImageNotice::Ready {
            width: 120,
            height: 80,
        },
    );
    assert_eq!(rasters(&app.take_effects()), [(240, 160)]);
    // Drawn 1.4 times as wide: still inside the band, so the raster stays.
    app.zoom(1.4).tick(START + 20).tick(START + 30);
    assert_eq!(rasters(&app.take_effects()), []);
    // Drawn twice as wide is outside it, but not while the zoom is moving:
    // the first look sees a new zoom, the second sees it held.
    app.zoom(2.0).tick(START + 40);
    assert_eq!(rasters(&app.take_effects()), []);
    app.tick(START + 50);
    assert_eq!(rasters(&app.take_effects()), [(480, 320)]);
    // The old raster is still what is on screen until the new one is ready.
    assert!(matches!(
        state(&app, "art.svg"),
        Some(ImageState::Ready { .. })
    ));
}

#[test]
fn a_file_changed_on_disk_is_loaded_again_and_the_old_picture_stays_until_the_new_one_is_ready() {
    let mut app = opened(document([file_at("a", "assets/shot.png")]));
    let file = "assets/shot.png";
    answer(
        &mut app,
        file,
        ImageNotice::Ready {
            width: 10,
            height: 10,
        },
    );
    app.take_effects();
    answer(&mut app, file, ImageNotice::Changed);
    assert_eq!(image_effects(&app.take_effects()), [(0, file)]);
    let ready = |width, height| Some(ImageState::Ready { width, height });
    assert_eq!(state(&app, file), ready(10, 10));
    // Read while the file was half written: the picture is not lost.
    answer(&mut app, file, ImageNotice::Failed);
    assert_eq!(state(&app, file), ready(10, 10));
    answer(
        &mut app,
        file,
        ImageNotice::Ready {
            width: 20,
            height: 5,
        },
    );
    assert_eq!(state(&app, file), ready(20, 5));
}

#[test]
fn a_gif_shows_the_frame_the_clock_is_at_and_only_while_it_is_on_screen() {
    let mut app = opened(document([file_at("g", "loop.gif")]));
    app.viewport((1000.0, 800.0)).tick(START);
    let delays_ms: Arc<[u32]> = Arc::from([100, 200, 100]);
    answer(
        &mut app,
        "loop.gif",
        ImageNotice::Animated {
            width: 8,
            height: 8,
            delays_ms,
        },
    );
    let mut seen = Vec::new();
    for at in [50, 100, 250, 300, 399, 400, 520] {
        app.tick(START + at);
        seen.push(app.app().image_frame("loop.gif"));
    }
    assert_eq!(seen, [0, 1, 1, 2, 2, 0, 1]);
    // 500 ms in is the start of frame 1's second round: 200 ms to the next.
    app.tick(START + 500);
    assert_eq!(app.app().next_frame_in_ms(), Some(200));
    // Panned off screen it is not looked at, and nothing wakes for it.
    let epoch = app.app().animation_epoch();
    app.act(Action::SetCamera(Camera::new(Vec2::new(-5000.0, 0.0), 1.0)));
    app.tick(START + 700).tick(START + 900);
    assert_eq!(app.app().animation_epoch(), epoch);
    assert_eq!(app.app().next_frame_in_ms(), None);
}
