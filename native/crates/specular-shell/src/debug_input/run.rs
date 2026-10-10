//! Doing one step of a script in the live window.

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use glam::Vec2;
use gpui_kit::{AnyWindowHandle, AsyncApp};
use specular_doc::{EdgeId, EntityId, ItemId};
use specular_interact::{Action, Event, SidebarAction};

use super::events::{self, DRAGGED, LEFT_DOWN, LEFT_UP, MOVED};
use super::steps::{self, Do, Place};
use super::{access, control, menu, window, window_facts};
use crate::view::{self, CanvasFocus};
use crate::{canvas, shell};

/// How long a named control is waited for before the step fails: a list
/// that a click just opened is laid out a frame or two later.
const CONTROL_WAIT: Duration = Duration::from_millis(1500);
const CONTROL_POLL: Duration = Duration::from_millis(40);

/// How often a `pan` scrolls: a 120 Hz display's refresh.
const REFRESH: Duration = Duration::from_micros(8333);

/// How far inside the content's bottom-right corner a `resize-drag` presses.
const RESIZE_INSET: f32 = 2.0;

thread_local! {
    /// The modifier flags `hold` set for the pointer steps after it.
    static HELD: Cell<usize> = const { Cell::new(0) };
    /// Where the pointer was last put.
    static POINTER: Cell<Vec2> = const { Cell::new(Vec2::ZERO) };
    /// Whether the left button is down.
    static DOWN: Cell<bool> = const { Cell::new(false) };
}

fn mouse(kind: usize, at: Vec2, clicks: isize) {
    POINTER.set(at);
    events::mouse(kind, f64::from(at.x), f64::from(at.y), clicks, HELD.get());
}

/// Moves the pointer to `at`: a drag while the button is down.
fn point(at: Vec2) {
    mouse(if DOWN.get() { DRAGGED } else { MOVED }, at, 0);
}

fn shot(path: &Path) -> Result<(), String> {
    let (number, _) = window_facts().ok_or("there is no window")?;
    let taken = Command::new("screencapture")
        .args(["-x", "-o", "-l", &number.to_string()])
        .arg(path)
        .status();
    match taken {
        Ok(status) if status.success() => Ok(()),
        other => Err(format!("screencapture failed: {other:?}")),
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|error| format!("writing {}: {error}", path.display()))
}

/// What a check reads back: the canvas showing, the camera, the selection,
/// what has the keys, the controls on screen, the window and every
/// entity's rect, one a line.
fn state() -> String {
    let mut out = canvas::with(|canvas| {
        let app = canvas.runtime.app();
        let session = app.session();
        let camera = session.camera;
        let mut out = String::new();
        let _ = writeln!(out, "canvas {}", app.space().active().name);
        let _ = writeln!(
            out,
            "camera {:.1} {:.1} {:.3}",
            camera.pan.x, camera.pan.y, camera.zoom
        );
        let _ = writeln!(out, "covered-left {}", app.covered_left());
        let _ = writeln!(out, "tool {:?}", session.tool);
        let _ = writeln!(out, "selection {:?}", session.selection.items());
        let _ = writeln!(out, "focus {:?}", session.focus);
        let _ = writeln!(out, "gesture {}", session.gesture.is_some());
        let _ = writeln!(out, "tool-defaults {:?}", app.tool_defaults());
        let _ = writeln!(
            out,
            "editing {:?}",
            app.text_edit().map(specular_interact::TextEdit::text)
        );
        let _ = writeln!(
            out,
            "can-undo {} can-redo {}",
            app.can_undo(),
            app.can_redo()
        );
        for (id, _, _) in app.pages() {
            if let Some(page) = app.page_state(id) {
                let scroll = page.scroll;
                let _ = writeln!(
                    out,
                    "page {id} scroll {:.0} {:.0} title {}",
                    scroll.x, scroll.y, page.title
                );
            }
        }
        for entity in app.document().entities() {
            let rect = entity.rect;
            let _ = writeln!(
                out,
                "entity {} {:.1} {:.1} {:.1} {:.1}",
                entity.id, rect.x, rect.y, rect.width, rect.height
            );
        }
        out
    })
    .unwrap_or_default();
    let names = |shown: Vec<(String, Vec2)>| {
        let names: Vec<String> = shown.into_iter().map(|(name, _)| name).collect();
        names.join(" ")
    };
    let _ = writeln!(out, "kit-controls {}", names(view::shown_controls()));
    let _ = writeln!(out, "canvas-controls {}", names(control::in_canvas_pass()));
    window::describe(&mut out);
    out
}

/// Where `place` is now. A named control is waited for a moment.
async fn locate(place: Place, cx: &AsyncApp) -> Result<Vec2, String> {
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

/// Whether the canvas has the keys, and not a Kit text field.
fn canvas_has_keys(window: AnyWindowHandle, cx: &mut AsyncApp) -> bool {
    window
        .update(cx, |_, window, cx| {
            (cx.try_global::<CanvasFocus>()).is_some_and(|focus| focus.0.is_focused(window))
        })
        .unwrap_or(false)
}

fn select(ids: &[String]) {
    canvas::with(|canvas| {
        let document = canvas.runtime.app().document();
        let items = ids.iter().map(|id| {
            if document.edge(&EdgeId::from(id.as_str())).is_some() {
                ItemId::Edge(EdgeId::from(id.as_str()))
            } else {
                ItemId::Entity(EntityId::from(id.as_str()))
            }
        });
        let select = Action::Select(items.collect());
        canvas.dispatch(Event::Action(select));
    });
}

fn show_sidebar(shown: bool) {
    canvas::with(|canvas| {
        if canvas.runtime.app().session().sidebar.shown() != shown {
            canvas.dispatch(Event::Action(Action::Sidebar(SidebarAction::Toggle)));
        }
    });
}

/// Does a step of the pointer, the wheel or the trackpad.
async fn pointer(step: Do, cx: &AsyncApp) -> Result<(), String> {
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
            let (width, height) = super::content_size().ok_or("there is no window")?;
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

/// Does `step`. An error is why it could not be done.
pub(super) async fn step(
    step: Do,
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    if steps::is_clipboard_key(&step) && !canvas_has_keys(window, cx) {
        return Err(
            "a Kit text field has the keys, and its copy, cut and paste use the system \
             clipboard, which a script must not touch"
                .to_owned(),
        );
    }
    match step {
        Do::Wait(ms) => {
            (cx.background_executor())
                .timer(Duration::from_millis(ms))
                .await;
        }
        step @ (Do::Move(_)
        | Do::Press(_)
        | Do::DragTo(_)
        | Do::Release(_)
        | Do::Click(..)
        | Do::RightClick(_)
        | Do::Drag(..)
        | Do::Scroll(..)
        | Do::Pan(..)
        | Do::ResizeDrag(..)
        | Do::Pinch(_)) => pointer(step, cx).await?,
        Do::Hold(held) => HELD.set(held),
        Do::Key {
            code,
            flags,
            characters,
            plain,
        } => events::key(code, flags, &characters, &plain),
        Do::Type(text) => events::type_text(&text)?,
        Do::Compose(text) => window::set_marked_text(&text)?,
        Do::Commit(text) => window::insert_text(&text)?,
        Do::Clipboard(text) => {
            canvas::with(|canvas| canvas.runtime.script_clipboard(Some(text)));
        }
        Do::Act(action) => canvas::dispatch(Event::Action(action)),
        Do::Select(ids) => select(&ids),
        Do::Sidebar(shown) => show_sidebar(shown),
        Do::Drop(paths, Some(at)) => view::drop_files(paths, at),
        Do::Drop(paths, None) => {
            canvas::with(|canvas| {
                canvas.runtime.drop_files(paths, None);
                canvas.refresh_models();
            });
        }
        Do::PasteImage(path) => cx.update(|cx| {
            // The view is reached once this update is over, so a file it
            // does not take fails the run from there.
            shell::with_view(cx, move |view, _, cx| {
                if !view.paste_image_file(&path, cx) {
                    super::fail(&format!(
                        "script step `paste-image`: {} is not an image a thread takes",
                        path.display()
                    ));
                    shell::begin_exit(cx);
                }
            });
        }),
        Do::Choose(name) => {
            let action = canvas::models()
                .and_then(|models| models.onboarding)
                .and_then(|view| view.choices.into_iter().find(|choice| choice.name == name))
                .map(|choice| choice.action)
                .ok_or_else(|| format!("the first-run view has no choice `{name}`"))?;
            canvas::dispatch(Event::Action(action));
        }
        Do::Resize(size) => window::resize(f64::from(size.x), f64::from(size.y)),
        Do::MoveWindow(to) => window::move_to(f64::from(to.x), f64::from(to.y)),
        Do::FullScreen => window::toggle_full_screen(),
        Do::MenuDump(path) => write(&path, &menu::dump())?,
        Do::MenuChoose(item) => menu::choose(&item),
        Do::State(path) => write(&path, &state())?,
        Do::Accessibility(path) => write(&path, &access::dump())?,
        Do::Shot(path) => shot(&path)?,
        Do::Save(path) => {
            let text = canvas::with(|canvas| canvas.runtime.canvas_text())
                .ok_or("there is no canvas")?
                .map_err(|error| format!("{error:#}"))?;
            write(&path, &text)?;
        }
        Do::Quit => cx.update(shell::begin_exit),
    }
    Ok(())
}
