//! Scene snapshots of image files: the pixels once loaded, the card until
//! then and after a failure.

use specular_doc::{Entity, FileRef, Kind, ObjectFit, Rect};
use specular_interact::{Event, ImageNotice};
use specular_testkit::{TestApp, assert_scene_snapshot};

fn image_file(id: &str, x: f64, fit: Option<ObjectFit>) -> Entity {
    let file = FileRef {
        file: format!("assets/{id}.png"),
        object_fit: fit,
        ..FileRef::default()
    };
    Entity::new(id, Rect::new(x, 100.0, 200.0, 100.0), Kind::File(file))
}

fn answer(app: &mut TestApp, id: &str, notice: ImageNotice) {
    let file = format!("assets/{id}.png");
    let image = app.app().image(&file).map(|image| image.key);
    assert!(image.is_some(), "{file} was never asked for");
    if let Some(image) = image {
        app.send(Event::Image { image, notice });
    }
}

#[test]
fn a_loaded_image_is_its_pixels_fitted_to_the_rect() {
    let mut app = TestApp::with_entities([
        image_file("default", 100.0, None),
        image_file("contain", 400.0, Some(ObjectFit::Contain)),
        image_file("cover", 700.0, Some(ObjectFit::Cover)),
        image_file("fill", 1000.0, Some(ObjectFit::Fill)),
    ]);
    // Each is a 100 by 100 square in a 200 by 100 rect.
    let square = ImageNotice::Ready {
        width: 100,
        height: 100,
    };
    for id in ["default", "contain", "cover", "fill"] {
        answer(&mut app, id, square.clone());
    }
    assert_scene_snapshot!(app);
}

#[test]
fn an_image_that_is_loading_missing_or_failed_is_the_card() {
    let mut app = TestApp::with_entities([
        image_file("loading", 100.0, None),
        image_file("missing", 400.0, None),
        image_file("failed", 700.0, None),
    ]);
    answer(&mut app, "missing", ImageNotice::Missing);
    answer(&mut app, "failed", ImageNotice::Failed);
    assert_scene_snapshot!(app);
}
