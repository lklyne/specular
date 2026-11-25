//! Input over the canvas reaches `update` the way the window sends it: Rune
//! pointer, wheel, pinch and key input at window points, on the GPU-free
//! `Runtime`. Each row starts from a fresh window on the kitchen sink.

use std::path::PathBuf;

use glam::Vec2;
use rune::{Env, Key, KeyEvent, MagnifyEvent, Mods, PhysicalInput, PhysicalKeyCode, Runtime};
use rune_shell_spike::shell::Shell;
use rune_shell_spike::{Options, app};
use specular_core::Camera;
use specular_doc::{EntityId, Rect};

/// 660,520 200x200 in the canvas: at this camera, 330,360 to 430,460 on
/// screen, clear of the toolbar.
const STICKY: &str = "sticky-color-3";
const CAMERA: Camera = Camera {
    pan: Vec2::new(0.0, 100.0),
    zoom: 0.5,
};
const CENTRE: (f32, f32) = (380.0, 410.0);

fn window() -> Runtime<Shell> {
    let options = Options {
        canvas: Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kitchen-sink.canvas"),
        ),
        camera: Some(CAMERA),
        still: true,
        ..Options::default()
    };
    let size = options.size;
    let mut rt = app(options).headless_runtime(size, Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

fn step(rt: &mut Runtime<Shell>) {
    for _ in 0..2 {
        rt.frame(1.0 / 60.0);
    }
}

fn rect(rt: &Runtime<Shell>) -> Option<Rect> {
    let id = EntityId::new(STICKY);
    rt.state
        .app
        .document()
        .entity(&id)
        .map(|entity| entity.rect)
}

fn chord(rt: &mut Runtime<Shell>, key: PhysicalKeyCode, logical: Key, mods: Mods) {
    rt.physical_input(PhysicalInput::Modifiers(mods));
    rt.key_with(KeyEvent { key: logical, mods }, key, None);
    step(rt);
    rt.physical_input(PhysicalInput::Modifiers(Mods::default()));
}

fn undo(rt: &mut Runtime<Shell>) {
    let cmd = Mods {
        cmd: true,
        ..Mods::default()
    };
    chord(rt, PhysicalKeyCode::KeyZ, Key::Command('z'), cmd);
}

fn drag_sticky(rt: &mut Runtime<Shell>) {
    let (x, y) = CENTRE;
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    for i in 1..=5 {
        rt.pointer_move(x + 20.0 * i as f32, y + 12.0 * i as f32);
        rt.frame(1.0 / 60.0);
    }
    rt.pointer_up(x + 100.0, y + 60.0);
    step(rt);
}

fn click_sticky_then_delete(rt: &mut Runtime<Shell>) {
    let (x, y) = CENTRE;
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    step(rt);
    chord(
        rt,
        PhysicalKeyCode::Backspace,
        Key::Backspace,
        Mods::default(),
    );
}

fn drag_from_the_toolbar(rt: &mut Runtime<Shell>) {
    let (x, y) = rt.find("tool.select").expect("the select tool is drawn");
    rt.pointer_down(x, y);
    rt.pointer_move(x, y + 300.0);
    rt.pointer_up(x, y + 300.0);
    step(rt);
}

fn wheel(rt: &mut Runtime<Shell>) {
    let (x, y) = CENTRE;
    rt.pointer_move(x, y);
    rt.wheel_event(rune::WheelEvent::new(30.0, -40.0).at(x, y).precise(true));
    step(rt);
}

fn pinch(rt: &mut Runtime<Shell>) {
    let (x, y) = CENTRE;
    rt.pointer_move(x, y);
    rt.magnify_event(MagnifyEvent::new(0.25).at(x, y));
    step(rt);
}

/// What a row expects after its input.
enum Then {
    /// The sticky moved by this many canvas units, and Cmd+Z puts it back.
    StickyMoved(Vec2),
    /// The sticky is gone, and Cmd+Z brings it back.
    StickyDeleted,
    /// Nothing on the canvas changed: no move, no selection.
    Untouched,
    /// The camera panned by this many screen points.
    Panned(Vec2),
    /// The camera zoomed by this factor.
    Zoomed(f32),
}

#[test]
fn canvas_input_reaches_update_and_undoes() {
    let rows: [(&str, fn(&mut Runtime<Shell>), Then); 5] = [
        (
            "a drag moves the sticky",
            drag_sticky,
            Then::StickyMoved(Vec2::new(200.0, 120.0)),
        ),
        (
            "a click selects, Backspace deletes",
            click_sticky_then_delete,
            Then::StickyDeleted,
        ),
        (
            "a drag that starts on the toolbar is the toolbar's",
            drag_from_the_toolbar,
            Then::Untouched,
        ),
        (
            "the wheel pans",
            wheel,
            Then::Panned(Vec2::new(30.0, -40.0)),
        ),
        ("a pinch zooms", pinch, Then::Zoomed(1.25)),
    ];
    for (name, act, then) in rows {
        let mut rt = window();
        let before = rect(&rt).expect("the sticky is in the kitchen sink");
        let camera = rt.state.app.session().camera;
        act(&mut rt);
        let after = rect(&rt);
        match then {
            Then::StickyMoved(by) => {
                let after = after.expect("still there");
                let moved = Vec2::new((after.x - before.x) as f32, (after.y - before.y) as f32);
                assert!((moved - by).length() < 0.01, "{name}: moved {moved}");
                undo(&mut rt);
                assert_eq!(rect(&rt), Some(before), "{name}: undone");
            }
            Then::StickyDeleted => {
                assert_eq!(after, None, "{name}");
                undo(&mut rt);
                assert_eq!(rect(&rt), Some(before), "{name}: undone");
            }
            Then::Untouched => {
                assert_eq!(after, Some(before), "{name}");
                let selection = &rt.state.app.session().selection;
                assert!(selection.items().is_empty(), "{name}: {selection:?}");
                assert_eq!(rt.state.app.session().camera, camera, "{name}");
            }
            Then::Panned(by) => {
                let pan = rt.state.app.session().camera.pan - camera.pan;
                assert!((pan - by).length() < 0.01, "{name}: panned {pan}");
            }
            Then::Zoomed(factor) => {
                let zoom = rt.state.app.session().camera.zoom / camera.zoom;
                assert!((zoom - factor).abs() < 0.001, "{name}: zoomed {zoom}");
            }
        }
    }
}
