//! A page in a device frame and the safe area of its device's screen: a
//! document is laid out inside it until it says it covers the screen, and
//! then it is told the insets.

use glam::DVec2;
use specular_core::{CssSize, PageSafeArea};
use specular_doc::{EntityId, Rect};
use specular_interact::{Effect, PageNotice};
use specular_testkit::TestApp;

/// One page with `metadata`, its screen the size given.
fn framed(size: (f64, f64), metadata: &str) -> TestApp {
    TestApp::from_canvas(&format!(
        r#"{{"nodes":[{{"id":"p1","type":"link","x":100,"y":100,"width":{},"height":{},
            "url":"https://example.com/","metadata":{metadata}}}],"edges":[]}}"#,
        size.0, size.1
    ))
}

fn fit(covers: bool) -> PageNotice {
    PageNotice::ViewportFit {
        covers,
        color: None,
    }
}

#[test]
fn a_framed_page_is_laid_out_inside_its_devices_safe_area() {
    let phone = r#""showDeviceFrame":true,"deviceId":"iphone-17""#;
    // The metadata, the screen, then the document in it: how far in from
    // the screen's corner it sits, and its size.
    let rows = [
        // The island above and the home indicator below.
        (
            format!("{{{phone}}}"),
            (402.0, 874.0),
            Rect::new(0.0, 62.0, 402.0, 778.0),
        ),
        // On its side the island's inset is on both short edges.
        (
            format!(r#"{{{phone},"deviceOrientation":"landscape"}}"#),
            (874.0, 402.0),
            Rect::new(62.0, 0.0, 750.0, 381.0),
        ),
        // The clay frame is the same device.
        (
            format!(r#"{{{phone},"deviceFrameStyle":"clay"}}"#),
            (402.0, 874.0),
            Rect::new(0.0, 62.0, 402.0, 778.0),
        ),
        // No frame, no device over the page.
        (
            r#"{"showDeviceFrame":false,"deviceId":"iphone-17"}"#.to_owned(),
            (402.0, 874.0),
            Rect::new(0.0, 0.0, 402.0, 874.0),
        ),
        // A screen with nothing over it.
        (
            r#"{"showDeviceFrame":true,"deviceId":"iphone-se"}"#.to_owned(),
            (375.0, 667.0),
            Rect::new(0.0, 0.0, 375.0, 667.0),
        ),
        (
            r#"{"showDeviceFrame":true,"deviceId":"ipad-mini"}"#.to_owned(),
            (744.0, 1133.0),
            Rect::new(0.0, 0.0, 744.0, 1133.0),
        ),
    ];
    let id = EntityId::new("p1");
    for (metadata, size, document) in rows {
        let app = framed(size, &metadata);
        let placement = app.app().page_placement(&id).unwrap();
        // The entity is still the screen.
        let screen = app.rect("p1");
        assert_eq!((screen.width, screen.height), size, "{metadata}");
        let document = document.translated(screen.x, screen.y);
        assert_eq!(placement.rect, document, "{metadata}");
        let viewport = CssSize::new(document.width as u32, document.height as u32);
        assert_eq!(placement.viewport, viewport, "{metadata}");
        // A canvas point maps into the document from where it sits.
        let corner = DVec2::new(document.x + 10.0, document.y + 20.0);
        assert_eq!(placement.page_local(corner), DVec2::new(10.0, 20.0));
    }
}

#[test]
fn a_document_that_covers_its_screen_gets_all_of_it_and_is_told_the_insets() {
    let mut app = framed(
        (402.0, 874.0),
        r#"{"showDeviceFrame":true,"deviceId":"iphone-17"}"#,
    );
    let id = EntityId::new("p1");
    app.take_effects();
    // A document that does not cover changes nothing.
    app.page_reports("p1", fit(false));
    assert_eq!(app.take_effects(), Vec::new());

    app.page_reports("p1", fit(true));
    let effects = app.take_effects();
    let told: Vec<PageSafeArea> = (effects.iter())
        .filter_map(|effect| match effect {
            Effect::SetPageDevice { device, .. } => Some(device.safe_area),
            _ => None,
        })
        .collect();
    let island = PageSafeArea {
        top: 62,
        right: 0,
        bottom: 34,
        left: 0,
    };
    assert_eq!(told, [island]);
    let viewport = Effect::SetPageViewport {
        page: id.clone(),
        viewport: CssSize::new(402, 874),
    };
    assert!(effects.contains(&viewport), "{effects:?}");
    let placement = app.app().page_placement(&id).unwrap();
    assert_eq!(placement.rect, app.rect("p1"));

    // The next document does not cover: back inside, with nothing to clear.
    app.page_reports("p1", fit(false));
    let effects = app.take_effects();
    let back = Effect::SetPageViewport {
        page: id,
        viewport: CssSize::new(402, 778),
    };
    assert!(effects.contains(&back), "{effects:?}");
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::SetPageDevice { device, .. } if device.safe_area == PageSafeArea::default()
    )));
}
