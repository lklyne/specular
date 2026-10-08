//! The page properties: viewport preset, orientation, custom size, device
//! frame and color scheme. Each is one undo step and resizes the page's host
//! when the size changes.

use specular_core::CssSize;
use specular_doc::{ColorScheme, Rect};
use specular_interact::{Action, Effect, Orientation, Property, property};
use specular_testkit::{TestApp, assert_doc_snapshot, document, page};

fn set(app: &mut TestApp, property: Property) {
    app.act(Action::SetProperty(property));
}

fn steps(app: &TestApp) -> bool {
    app.app().can_undo()
}

fn viewports(app: &mut TestApp) -> Vec<(String, CssSize)> {
    app.take_effects()
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::SetPageViewport { page, viewport } => Some((page.to_string(), viewport)),
            _ => None,
        })
        .collect()
}

fn phone_and_laptop() -> TestApp {
    let doc = document([
        page("p1", Rect::new(100.0, 100.0, 375.0, 667.0)),
        page("p2", Rect::new(700.0, 100.0, 375.0, 667.0)),
    ]);
    TestApp::from_document(doc)
}

#[test]
fn a_viewport_preset_resizes_the_page_and_names_its_device() {
    let mut app = phone_and_laptop();
    app.select(&["p1", "p2"]);
    app.take_effects();
    set(&mut app, Property::ViewportPreset(1));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":393,"height":852,"url":"https://example.com/p1","presetIndex":1,"metadata":{"deviceId":"iphone-14-pro"}}
      {"id":"p2","type":"link","x":700,"y":100,"width":393,"height":852,"url":"https://example.com/p2","presetIndex":1,"metadata":{"deviceId":"iphone-14-pro"}}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert_eq!(viewports(&mut app).len(), 2, "each page's host is resized");
    assert_eq!(property::read::viewport_preset(app.app()), Some(1));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_landscape_preset_on_a_portrait_page_keeps_the_orientation_it_has() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    set(&mut app, Property::ViewportPreset(6));
    assert_eq!(app.rect("p1"), Rect::new(100.0, 100.0, 800.0, 1280.0));
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":1280,"height":800,"url":"https://example.com/p1","presetIndex":6,"metadata":{"deviceId":"laptop","deviceOrientation":"landscape"}}
      {"id":"p2","type":"link","x":700,"y":100,"width":375,"height":667,"url":"https://example.com/p2"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert_eq!(
        property::read::orientation(app.app()),
        Some(Orientation::Landscape)
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn rotating_a_page_turns_its_size() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    app.take_effects();
    set(&mut app, Property::ViewportPreset(0));
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_eq!(app.rect("p1").width, 667.0);
    assert_eq!(
        viewports(&mut app).last().map(|v| v.1),
        Some(CssSize::new(667, 375))
    );
    set(&mut app, Property::Orientation(Orientation::Portrait));
    assert_eq!(app.rect("p1").width, 375.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_custom_viewport_keeps_the_size_and_drops_the_device() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    set(&mut app, Property::ViewportPreset(0));
    set(&mut app, Property::CustomViewport);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":375,"height":667,"url":"https://example.com/p1","presetIndex":0,"metadata":{"customSize":{"width":375,"height":667},"pageSizeMode":"custom"}}
      {"id":"p2","type":"link","x":700,"y":100,"width":375,"height":667,"url":"https://example.com/p2"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert_eq!(property::read::viewport_preset(app.app()), None);
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_eq!(app.rect("p1").width, 375.0, "a custom size is not turned");
    app.assert_undo_returns_to_start();
}

#[test]
fn the_device_frame_and_color_scheme_are_page_settings() {
    let mut app = phone_and_laptop();
    app.select(&["p1", "p2"]);
    app.take_effects();
    set(&mut app, Property::DeviceFrame(true));
    set(&mut app, Property::ColorScheme(Some(ColorScheme::Dark)));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":375,"height":667,"url":"https://example.com/p1","metadata":{"showDeviceFrame":true},"colorScheme":"dark"}
      {"id":"p2","type":"link","x":700,"y":100,"width":375,"height":667,"url":"https://example.com/p2","metadata":{"showDeviceFrame":true},"colorScheme":"dark"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert!(viewports(&mut app).is_empty(), "neither changes the size");
    assert_eq!(property::read::device_frame(app.app()), Some(true));
    assert_eq!(
        property::read::color_scheme(app.app()),
        Some(Some(ColorScheme::Dark))
    );
    set(&mut app, Property::ColorScheme(None));
    assert_eq!(property::read::color_scheme(app.app()), Some(None));
    app.assert_undo_returns_to_start();
}

#[test]
fn an_unknown_preset_applies_to_nothing() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    assert!(!Property::ViewportPreset(99).applies_to(app.app()));
    set(&mut app, Property::ViewportPreset(99));
    assert!(!steps(&app));
}
