//! Pointer steps: synthetic mouse events at the place a step names, and swipes.

use std::time::{Duration, Instant};

use glam::Vec2;
use gpui_kit::AsyncApp;

use super::super::control;
use super::super::events::{self, DRAGGED, LEFT_DOWN, LEFT_UP, MOVED};
use super::super::steps::{Do, Place};
use super::{CONTROL_POLL, CONTROL_WAIT, DOWN, HELD, POINTER, REFRESH, RESIZE_INSET};

pub(super) fn mouse(kind: usize, at: Vec2, clicks: isize) {
    POINTER.set(at);
    events::mouse(kind, f64::from(at.x), f64::from(at.y), clicks, HELD.get());
}

/// Moves the pointer to `at`: a drag while the button is down.
pub(super) fn point(at: Vec2) {
    mouse(if DOWN.get() { DRAGGED } else { MOVED }, at, 0);
}

/// Where `place` is now. A named control is waited for a moment.
pub(super) async fn locate(place: Place, cx: &AsyncApp) -> Result<Vec2, String> {
    let name = match place {
        Place::At(at) => return Ok(at),
        Place::Pointer => return Ok(POINTER.get()),
        Place::Control(name) => name,
    };
    let deadline = Instant::now() + CONTROL_WAIT;
    loop {
        match control::locate(&name) {
            Ok(at) => return Ok(at),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => cx.background_executor().timer(CONTROL_POLL).await,
        }
    }
}

/// Presses where the pointer is, drags by `by` over `ms` with a move every
/// refresh, and releases `late` ms after the last move.
async fn swipe(by: Vec2, ms: u64, late: u64, cx: &AsyncApp) {
    let from = POINTER.get();
    mouse(LEFT_DOWN, from, 1);
    DOWN.set(true);
    let span = Duration::from_millis(ms.max(1));
    let started = Instant::now();
    loop {
        cx.background_executor().timer(REFRESH).await;
        let done = (started.elapsed().as_secs_f32() / span.as_secs_f32()).min(1.0);
        mouse(DRAGGED, from + by * done, 0);
        if done >= 1.0 {
            break;
        }
    }
    if late > 0 {
        (cx.background_executor())
            .timer(Duration::from_millis(late))
            .await;
    }
    mouse(LEFT_UP, from + by, 1);
    DOWN.set(false);
}

/// How long the window is given to take a release before the next press.
const LIFTED: Duration = Duration::from_millis(100);

/// Lets go of a button a script left down: a drag Escape cancelled is
/// still held in the window, and a person lifts it before pressing again.
/// Headless, the next press starts afresh by itself.
pub(super) async fn lift(cx: &AsyncApp) {
    if DOWN.replace(false) {
        mouse(LEFT_UP, POINTER.get(), 1);
        cx.background_executor().timer(LIFTED).await;
    }
}

/// Does a step of the pointer, the wheel or the trackpad.
pub(super) async fn pointer(step: Do, cx: &AsyncApp) -> Result<(), String> {
    if matches!(
        step,
        Do::Press(_) | Do::Click(..) | Do::Drag(..) | Do::Swipe(..) | Do::ResizeDrag(..)
    ) {
        lift(cx).await;
    }
    match step {
        Do::Move(place) => point(locate(place, cx).await?),
        Do::Press(place) => {
            let at = locate(place, cx).await?;
            point(at);
            mouse(LEFT_DOWN, at, 1);
            DOWN.set(true);
        }
        Do::DragTo(at) => mouse(DRAGGED, at, 0),
        Do::Release(place) => {
            let at = locate(place, cx).await?;
            mouse(LEFT_UP, at, 1);
            DOWN.set(false);
        }
        Do::Click(place, count) => {
            let at = locate(place, cx).await?;
            point(at);
            for clicks in 1..=isize::from(count) {
                mouse(LEFT_DOWN, at, clicks);
                mouse(LEFT_UP, at, clicks);
            }
        }
        Do::RightClick(at) => {
            point(at);
            events::right_click(f64::from(at.x), f64::from(at.y));
        }
        Do::Drag(from, to) => {
            point(from);
            mouse(LEFT_DOWN, from, 1);
            mouse(DRAGGED, from.midpoint(to), 0);
            mouse(DRAGGED, to, 0);
            mouse(LEFT_UP, to, 1);
        }
        Do::Scroll(place, by) => {
            let at = locate(place, cx).await?;
            POINTER.set(at);
            let (x, y) = (f64::from(at.x), f64::from(at.y));
            events::scroll(x, y, by.x as i32, by.y as i32, HELD.get());
        }
        Do::Pan(by, ms) => {
            let at = POINTER.get();
            let (x, y) = (f64::from(at.x), f64::from(at.y));
            let until = Instant::now() + Duration::from_millis(ms);
            while Instant::now() < until {
                events::scroll(x, y, by.x as i32, by.y as i32, HELD.get());
                cx.background_executor().timer(REFRESH).await;
            }
        }
        Do::ResizeDrag(by, ms) => {
            let (width, height) = super::super::content_size().ok_or("there is no window")?;
            // Just inside the corner, where AppKit takes a press as the
            // start of a resize.
            let from = Vec2::new(width as f32, height as f32) - Vec2::splat(RESIZE_INSET);
            point(from);
            mouse(LEFT_DOWN, from, 1);
            DOWN.set(true);
            let span = Duration::from_millis(ms.max(1));
            let started = Instant::now();
            loop {
                let done = (started.elapsed().as_secs_f32() / span.as_secs_f32()).min(1.0);
                mouse(DRAGGED, from + by * done, 0);
                if done >= 1.0 {
                    break;
                }
                cx.background_executor().timer(REFRESH).await;
            }
            mouse(LEFT_UP, from + by, 1);
            DOWN.set(false);
        }
        Do::Swipe(by, ms, late) => swipe(by, ms, late, cx).await,
        Do::Pinch(delta) => {
            let at = POINTER.get();
            let (x, y) = (f64::from(at.x), f64::from(at.y));
            for (amount, phase) in [(0.0, 1), (f64::from(delta), 2), (0.0, 4)] {
                events::magnify(x, y, amount, phase);
            }
        }
        // `step` hands over only the steps above.
        _ => {}
    }
    Ok(())
}
