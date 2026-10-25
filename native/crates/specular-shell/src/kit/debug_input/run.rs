//! Doing one step of a script in the live window.

mod pointer;

use self::pointer::{lift, locate, mouse, point, pointer};

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use glam::Vec2;
use gpui_kit::{AnyWindowHandle, AsyncApp};
use specular_doc::{Document, EdgeId, EntityId, ItemId};
use specular_interact::{Action, ControlId, Event, SidebarAction};
use specular_shell::script::{self, Checks, Expect, PanelStep};

use super::events::{self, LEFT_DOWN, LEFT_UP};
use super::steps::{self, Do, Place};
use super::{access, menu, window, window_facts};
use crate::view::{self, CanvasFocus};
use crate::{canvas, shell};

/// How long a named control is waited for before the step fails: a list
/// that a click just opened is laid out a frame or two later.
const CONTROL_WAIT: Duration = Duration::from_millis(1500);
const CONTROL_POLL: Duration = Duration::from_millis(40);

/// How long the pointer rests on a control before a click presses it.
const HOVER: Duration = Duration::from_millis(150);
/// How many times a control that keeps moving is followed before the click.
const HOVER_TRIES: usize = 4;

/// More undo steps than any script makes.
const UNDO_LIMIT: usize = 10_000;

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

/// What a check reads back: the `expect` facts, then what only the
/// window knows: the camera, what covers the canvas, the tool's defaults,
/// the pages' scroll, the controls on screen and the window's frame.
fn state(window: AnyWindowHandle, cx: &mut AsyncApp) -> String {
    let typed = typed_field(window, cx);
    let keys = canvas_has_keys(window, cx);
    let mut out = canvas::with(|canvas| {
        let app = canvas.runtime.app();
        let mut out = script::dump(app);
        let _ = writeln!(out, "# covered-left {}", app.covered_left());
        let _ = writeln!(out, "# gesture {}", app.session().gesture.is_some());
        let _ = writeln!(out, "# tool-defaults {:?}", app.tool_defaults());
        for (id, _) in app.pages() {
            if let Some(page) = app.page_state(id) {
                let scroll = page.scroll;
                let _ = writeln!(
                    out,
                    "# page {id} scroll {:.0} {:.0} title {}",
                    scroll.x, scroll.y, page.title
                );
            }
        }
        out
    })
    .unwrap_or_default();
    let names = |shown: Vec<(String, Vec2)>| {
        let names: Vec<String> = shown.into_iter().map(|(name, _)| name).collect();
        names.join(" ")
    };
    let shown = view::shown_controls();
    for (name, at) in &shown {
        let _ = writeln!(out, "# kit-control {name} at {:.0} {:.0}", at.x, at.y);
    }
    let _ = writeln!(out, "# kit-controls {}", names(shown));
    if let Some((id, text)) = typed {
        let _ = writeln!(out, "# kit-field {id} {text}");
    }
    let _ = writeln!(out, "# canvas-has-keys {keys}");
    window::describe(&mut out);
    out
}

/// Runs `check` on the app. With no canvas there is nothing to check.
fn checked(
    check: impl FnOnce(&specular_interact::App) -> anyhow::Result<()>,
) -> Result<(), String> {
    canvas::with(|canvas| check(canvas.runtime.app()))
        .ok_or("there is no canvas")?
        .map_err(|error| format!("{error:#}"))
}

/// Undoes until nothing is left, then checks the canvas is the start.
fn undo_to_start(checks: &Checks, except: &[String]) -> Result<(), String> {
    for _ in 0..UNDO_LIMIT {
        let more = canvas::with(|canvas| canvas.runtime.app().can_undo()).unwrap_or(false);
        if !more {
            break;
        }
        canvas::dispatch(Event::Action(Action::Undo));
    }
    checked(|app| checks.back_at_start(app, except))
}

/// Does a step that checks the app or remembers its canvas. `typed` is
/// the Kit field being typed in, if one is.
fn check_step(
    step: Do,
    checks: &mut Checks,
    typed: Option<(ControlId, String)>,
) -> Result<(), String> {
    match step {
        Do::Expect(Expect::Editing(wanted)) if let Some((_, typed)) = typed => {
            typed_is(&typed, wanted)
        }
        Do::Expect(expect) => checked(|app| checks.check(app, &expect)),
        Do::Mark(name) => canvas::with(|canvas| checks.mark(canvas.runtime.app(), &name))
            .ok_or("there is no canvas")?
            .map_err(|error| format!("{error:#}")),
        Do::Reopen => reopen(),
        Do::UndoToStart(except) => undo_to_start(checks, &except),
        // `step` hands over only the steps above.
        _ => Ok(()),
    }
}

/// Sends the event a panel step stands for, read against the app now.
fn panel(step: &PanelStep) -> Result<(), String> {
    let event = canvas::with(|canvas| step.event(canvas.runtime.app()))
        .ok_or("there is no canvas")?
        .map_err(|error| format!("{error:#}"))?;
    canvas::dispatch(event);
    Ok(())
}

/// Writes the canvas as a save would and opens what was written.
fn reopen() -> Result<(), String> {
    let text = canvas::with(|canvas| canvas.runtime.canvas_text())
        .ok_or("there is no canvas")?
        .map_err(|error| format!("{error:#}"))?;
    let document = Document::from_canvas_str(&text).map_err(|error| error.to_string())?;
    canvas::dispatch(Event::DocumentOpened(Box::new(document)));
    Ok(())
}

/// Clicks the control `name` where the window draws it. The settings
/// dialog's switches, and a control the Kit does not draw (a sidebar row
/// scrolled out of view, a choice in a menu that is not open), are pressed
/// from the models, as a headless run presses them; the run says so.
async fn press_control(name: &str, cx: &AsyncApp) -> Result<(), String> {
    let drawn = if name.starts_with("settings.") {
        None
    } else {
        locate(Place::Control(name.to_owned()), cx).await.ok()
    };
    if let Some(mut at) = drawn {
        lift(cx).await;
        // A person's pointer is over a control before they press it, and
        // what shows on hover (a list row's fields, a tab's close button)
        // is drawn by then. It arrives by moving: GPUI works out the hover
        // again on a move, not when what is under a still pointer changes.
        // A control that moved meanwhile (a tab row closing up) is followed.
        for _ in 0..HOVER_TRIES {
            point(at + Vec2::ONE);
            point(at);
            cx.background_executor().timer(HOVER).await;
            let now = locate(Place::Control(name.to_owned()), cx).await?;
            if now == at {
                break;
            }
            at = now;
        }
        mouse(LEFT_DOWN, at, 1);
        mouse(LEFT_UP, at, 1);
        return Ok(());
    }
    let control =
        canvas::with(|canvas| specular_interact::control_named(canvas.runtime.app(), name))
            .ok_or("there is no canvas")?
            .map_err(|error| error.to_string())?;
    if !name.starts_with("settings.") {
        eprintln!("specular: `{name}` is not drawn; pressed from the models");
    }
    canvas::dispatch(Event::Control(control, specular_core::Modifiers::default()));
    Ok(())
}

/// Moves onto or presses a named control. One that is not drawn has
/// nowhere to point at; as headless, its name must still be a control's.
async fn hover_or_hold(step: Do, cx: &AsyncApp) -> Result<(), String> {
    let (Do::Move(Place::Control(name)) | Do::Press(Place::Control(name))) = &step else {
        return pointer(step, cx).await;
    };
    if locate(Place::Control(name.clone()), cx).await.is_ok() {
        return pointer(step, cx).await;
    }
    canvas::with(|canvas| specular_interact::control_named(canvas.runtime.app(), name))
        .ok_or("there is no canvas")?
        .map_err(|error| error.to_string())?;
    eprintln!("specular: `{name}` is not drawn; nothing to point at");
    Ok(())
}

/// What is being typed in a Kit field, checked against `expect editing`.
/// The app hands a field's keys to the Kit and edits nothing itself, so
/// the window's field is what a headless run's edit is.
fn typed_is(typed: &str, wanted: Option<String>) -> Result<(), String> {
    if wanted.as_deref() == Some(typed) {
        return Ok(());
    }
    let wanted = wanted.unwrap_or_else(|| "none".to_owned());
    Err(format!(
        "editing is {typed} in a Kit field, expected {wanted}"
    ))
}

/// The Kit field being typed in, and its text.
fn typed_field(window: AnyWindowHandle, cx: &mut AsyncApp) -> Option<(ControlId, String)> {
    window
        .update(cx, |_, window, cx| view::typed_field(window, cx))
        .ok()
        .flatten()
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

/// Does `step`. An error is why it could not be done.
pub(super) async fn step(
    step: Do,
    checks: &mut Checks,
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
        Do::Click(Place::Control(name), 1) => press_control(&name, cx).await?,
        step @ (Do::Move(Place::Control(_)) | Do::Press(Place::Control(_))) => {
            hover_or_hold(step, cx).await?;
        }
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
        | Do::Swipe(..)
        | Do::Pinch(_)) => pointer(step, cx).await?,
        Do::Hold(held) => HELD.set(held),
        Do::Key {
            code,
            flags,
            characters,
            plain,
            // Modifiers a `hold` keeps down are down for keys too, as they
            // are headless.
        } => events::key(code, flags | HELD.get(), &characters, &plain),
        Do::Type(text) => events::type_text(&text)?,
        Do::Compose(text) => window::set_marked_text(&text)?,
        Do::Commit(text) => window::insert_text(&text)?,
        Do::Clipboard(text) => {
            canvas::with(|canvas| canvas.runtime.script_clipboard(Some(text)));
        }
        Do::Act(action) => canvas::dispatch(Event::Action(action)),
        Do::Send(event) => canvas::dispatch(event),
        Do::Panel(step) => panel(&step)?,
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
        Do::State(path) => write(&path, &state(window, cx))?,
        Do::Accessibility(path) => write(&path, &access::dump())?,
        // A picture is for a person to read, and a locked screen or a
        // covered window has none to take: the checks go on without it.
        Do::Shot(path) => {
            if let Err(error) = shot(&path) {
                eprintln!("specular: no snapshot {}: {error}", path.display());
            }
        }
        Do::Save(path) => {
            let text = canvas::with(|canvas| canvas.runtime.canvas_text())
                .ok_or("there is no canvas")?
                .map_err(|error| format!("{error:#}"))?;
            write(&path, &text)?;
        }
        step @ (Do::Expect(_) | Do::Mark(_) | Do::Reopen | Do::UndoToStart(_)) => {
            check_step(step, checks, typed_field(window, cx))?;
        }
        Do::Quit => cx.update(shell::begin_exit),
    }
    Ok(())
}
