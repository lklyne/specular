//! The page properties: viewport preset, orientation, custom size, device
//! frame and color scheme. Each is one undo step and resizes the page's host
//! when the size changes.

use specular_core::{CssSize, Modifiers};
use specular_doc::{ColorScheme, DeviceFrameStyle, Rect};
use specular_interact::property::read::device_frame as frame;
use specular_interact::{Action, Effect, Key, Orientation, PageStep, Property, property};
use specular_testkit::{CMD, TestApp, assert_doc_snapshot, document, page};

fn set(app: &mut TestApp, property: Property) {
    app.act(Action::SetProperty(property));
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
fn rotating_a_page_turns_its_size() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    app.take_effects();
    set(&mut app, Property::ViewportPreset(0));
    set(&mut app, Property::Orientation(Orientation::Landscape));
    // The page turns about its centre, a whole pixel at a time.
    assert_eq!(app.rect("p1"), Rect::new(-46.0, 246.0, 667.0, 375.0));
    assert_eq!(
        viewports(&mut app).last().map(|v| v.1),
        Some(CssSize::new(667, 375))
    );
    set(&mut app, Property::Orientation(Orientation::Portrait));
    assert_eq!(app.rect("p1"), Rect::new(100.0, 100.0, 375.0, 667.0));
    set(&mut app, Property::Orientation(Orientation::Landscape));
    // A preset chosen while turned keeps the turn: 393x852 lies down as
    // 852x393, from the same corner as any resize.
    set(&mut app, Property::ViewportPreset(1));
    assert_eq!(app.rect("p1"), Rect::new(-46.0, 246.0, 852.0, 393.0));
    set(&mut app, Property::Orientation(Orientation::Portrait));
    assert_eq!(app.rect("p1").width, 393.0);
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
    // A custom size is laid across by each turn, about its centre, and
    // stays custom.
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_eq!(app.rect("p1"), Rect::new(-46.0, 246.0, 667.0, 375.0));
    assert_eq!(property::read::viewport_preset(app.app()), None);
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_eq!(app.rect("p1").width, 667.0, "the same way again is no turn");
    set(&mut app, Property::Orientation(Orientation::Portrait));
    assert_eq!(app.rect("p1"), Rect::new(100.0, 100.0, 375.0, 667.0));
    // A preset after a custom size is a preset again, and turns.
    set(&mut app, Property::ViewportPreset(1));
    set(&mut app, Property::Orientation(Orientation::Landscape));
    assert_eq!(app.rect("p1").width, 852.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn the_device_frame_and_color_scheme_are_page_settings() {
    let mut app = phone_and_laptop();
    app.select(&["p1", "p2"]);
    app.take_effects();
    set(
        &mut app,
        Property::DeviceFrame(Some(DeviceFrameStyle::Clay)),
    );
    set(&mut app, Property::ColorScheme(Some(ColorScheme::Dark)));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":375,"height":667,"url":"https://example.com/p1","metadata":{"showDeviceFrame":true},"colorScheme":"dark"}
      {"id":"p2","type":"link","x":700,"y":100,"width":375,"height":667,"url":"https://example.com/p2","metadata":{"showDeviceFrame":true},"colorScheme":"dark"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert!(viewports(&mut app).is_empty(), "neither changes the size");
    assert_eq!(
        property::read::device_frame(app.app()),
        Some(Some(DeviceFrameStyle::Clay))
    );
    assert_eq!(
        property::read::color_scheme(app.app()),
        Some(Some(ColorScheme::Dark))
    );
    set(&mut app, Property::DeviceFrame(None));
    assert_eq!(property::read::device_frame(app.app()), Some(None));
    set(&mut app, Property::ColorScheme(None));
    assert_eq!(property::read::color_scheme(app.app()), Some(None));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_typed_width_or_height_makes_the_size_custom_and_keeps_the_other_side() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    set(&mut app, Property::ViewportPreset(1));
    app.take_effects();
    set(&mut app, Property::ViewportWidth(500.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"p1","type":"link","x":100,"y":100,"width":500,"height":852,"url":"https://example.com/p1","presetIndex":1,"metadata":{"customSize":{"width":500,"height":852},"pageSizeMode":"custom"}}
      {"id":"p2","type":"link","x":700,"y":100,"width":375,"height":667,"url":"https://example.com/p2"}
    edges:
    specular: {"entityOrder":["p1","p2"]}
    "#);
    assert_eq!(viewports(&mut app).len(), 1, "the page's host follows");
    assert_eq!(property::read::viewport_preset(app.app()), None);
    set(&mut app, Property::ViewportHeight(300.0));
    assert_eq!(app.rect("p1"), Rect::new(100.0, 100.0, 500.0, 300.0));
    set(&mut app, Property::ViewportHeight(300.0));
    app.undo();
    assert_eq!(
        app.rect("p1").height,
        852.0,
        "one step each, and none for a repeat"
    );
    app.undo();
    assert_eq!(app.rect("p1").width, 393.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_selected_pages_keys_step_its_frame_turn_and_color_scheme() {
    let mut app = phone_and_laptop();
    app.select(&["p1"]);
    set(&mut app, Property::ViewportPreset(1));
    let keys = Modifiers { alt: true, ..CMD };
    app.chord(keys, Key::Char('f'));
    assert_eq!(
        property::read::device_frame(app.app()),
        Some(Some(DeviceFrameStyle::Clay))
    );
    app.chord(keys, Key::Char('r'));
    assert_eq!(app.rect("p1").width, 852.0, "turned to landscape");
    app.chord(keys, Key::Char('l'));
    assert_eq!(
        property::read::color_scheme(app.app()),
        Some(Some(ColorScheme::Light))
    );
    app.assert_undo_returns_to_start();
}

/// An iPhone 14 Pro with a clay frame as a file before frame styles wrote
/// it, and a laptop with no frame. The file places the phone by its shell's
/// corner.
const FRAMED_PHONE_AND_LAPTOP: &str = r#"{"nodes":[
    {"id":"phone","type":"link","x":100,"y":100,"width":393,"height":852,
     "url":"https://example.com/phone","presetIndex":1,
     "metadata":{"showDeviceFrame":true,"deviceId":"iphone-14-pro"}},
    {"id":"laptop","type":"link","x":700,"y":100,"width":1280,"height":800,
     "url":"https://example.com/laptop","presetIndex":6,
     "metadata":{"deviceId":"laptop","deviceOrientation":"landscape"}}],"edges":[]}"#;

fn step_frame(app: &mut TestApp) {
    app.act(Action::StepPage(PageStep::Frame));
}

#[test]
fn the_frame_steps_through_none_clay_and_realistic_and_skips_what_cannot_be_drawn() {
    use DeviceFrameStyle::{Clay, Realistic};
    let mut app = TestApp::from_canvas(FRAMED_PHONE_AND_LAPTOP);
    // The old file's frame is clay, and the screen sits 22 in from it.
    app.select(&["phone"]);
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    assert_eq!(app.rect("phone"), Rect::new(122.0, 122.0, 393.0, 852.0));

    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(Some(Realistic)));
    // The drawing's margins are its own, and the shell's corner stays.
    assert_eq!(app.rect("phone"), Rect::new(119.0, 117.0, 393.0, 852.0));
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"phone","type":"link","x":100,"y":100,"width":393,"height":852,"url":"https://example.com/phone","presetIndex":1,"metadata":{"showDeviceFrame":true,"deviceId":"iphone-14-pro","deviceFrameStyle":"realistic"}}
      {"id":"laptop","type":"link","x":700,"y":100,"width":1280,"height":800,"url":"https://example.com/laptop","presetIndex":6,"metadata":{"deviceId":"laptop","deviceOrientation":"landscape"}}
    edges:
    specular: {"entityOrder":["phone","laptop"]}
    "#);
    // What was written loads back as it is.
    let saved = app.document().to_canvas_string().unwrap();
    let mut reloaded = TestApp::from_canvas(&saved);
    reloaded.select(&["phone"]);
    assert_eq!(reloaded.rect("phone"), app.rect("phone"));
    assert_eq!(frame(reloaded.app()), Some(Some(Realistic)));
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(None));
    assert_eq!(app.rect("phone"), Rect::new(100.0, 100.0, 393.0, 852.0));
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    app.undo();
    app.undo();
    assert_eq!(frame(app.app()), Some(Some(Realistic)));
    assert_eq!(app.rect("phone"), Rect::new(119.0, 117.0, 393.0, 852.0));

    // A laptop has no drawing, so each press changes what is shown.
    app.select(&["laptop"]);
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(None));

    // Together they go as far as either can, then both lose the frame.
    app.select(&["phone", "laptop"]);
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    step_frame(&mut app);
    assert_eq!(frame(app.app()), None, "realistic and clay");
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(None));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_realistic_frame_is_clay_at_a_custom_size_and_comes_back_with_a_preset() {
    use DeviceFrameStyle::{Clay, Realistic};
    let mut app = TestApp::from_canvas(FRAMED_PHONE_AND_LAPTOP);
    app.select(&["phone"]);
    set(&mut app, Property::DeviceFrame(Some(Realistic)));
    set(&mut app, Property::ViewportWidth(500.0));
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    step_frame(&mut app);
    assert_eq!(frame(app.app()), Some(None), "no press does nothing");
    app.undo();
    // Laptop has no drawing either, and iPhone SE has.
    set(&mut app, Property::ViewportPreset(6));
    assert_eq!(frame(app.app()), Some(Some(Clay)));
    set(&mut app, Property::ViewportPreset(0));
    assert_eq!(frame(app.app()), Some(Some(Realistic)));
    app.assert_undo_returns_to_start();
}
