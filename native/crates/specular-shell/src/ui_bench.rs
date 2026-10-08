//! The UI-busy benchmark: what the canvas's frames cost while the Kit's
//! own UI is at work.
//!
//! `--bench` measures the canvas with the Kit's UI still. This runs one
//! profile in the ordinary window, with the toolbar, the sidebar and the
//! right panel drawn, and prints one JSON line: the canvas's frame rate and
//! intervals, GPUI's frames and how long each held the main thread, and how
//! often each part of the UI was built (`ui_stats`).
//!
//! `SPECULAR_UI_BENCH=<profile>` names it, `SPECULAR_UI_BENCH_MS` is how
//! long it is measured for (8000) and `SPECULAR_UI_BENCH_WARMUP_MS` how
//! long the window is left to load first (3000). The profiles, each under
//! a pan made of real scroll events unless it says otherwise:
//!
//! | Profile | What the UI does |
//! |---|---|
//! | `quiet` | nothing: the right panel is closed and the sidebar hidden |
//! | `chat-open` | the right panel is open and still |
//! | `chat-run` | a run streams text and its bar's label shimmers (`SPECULAR_AGENT_SCRIPT`) |
//! | `sidebar-churn` | the sidebar is shown while items are made and the canvas renamed |
//! | `popup-dropdown` | a tool's popup hangs from the toolbar and the zoom list is open |
//! | `typing` | keys are typed into the right panel's field |
//! | `resize` | the window is resized every refresh, with no pan |
//!
//! `fixtures/bench/run-kit-ui.sh` runs them all on scratch copies of the
//! bench canvases. A run with the screen locked or the display asleep
//! prints `"invalid"` and the reason in place of numbers.

use std::time::{Duration, Instant};

use glam::Vec2;
use gpui_kit::{AnyWindowHandle, App, AsyncApp};
use specular_doc::ItemId;
use specular_interact::{Action, CanvasAction, ChatAction, Event, Tool};

use crate::{canvas, debug_input, screen, shell, ui_stats};

/// Where the pointer rests: over the canvas with the sidebar and the right
/// panel both shown, in a window of the default size.
const POINTER: Vec2 = Vec2::new(700.0, 560.0);
/// How far a pan step scrolls, and how long before the pan turns back, so
/// what is drawn stays what the canvas opened on.
const PAN_STEP: f32 = 4.0;
const PAN_LEG: Duration = Duration::from_millis(1000);
/// How long the window is given after a profile is set up.
const SETTLE: Duration = Duration::from_millis(1500);
/// The fewest canvas frames a second that count as the window being shown.
const SHOWN_FPS: f64 = 5.0;

const TYPED: &str = "the quick brown fox jumps over the lazy dog ";
const TYPE_EVERY: Duration = Duration::from_millis(80);
const DUPLICATE_EVERY: Duration = Duration::from_millis(200);
const RENAME_EVERY: Duration = Duration::from_millis(500);
/// The sizes a resize goes between, and how far a step goes.
const WIDE: Vec2 = Vec2::new(1600.0, 1000.0);
const NARROW: Vec2 = Vec2::new(1240.0, 775.0);
const RESIZE_STEPS: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Profile {
    Quiet,
    ChatOpen,
    ChatRun,
    SidebarChurn,
    PopupDropdown,
    Typing,
    Resize,
}

impl Profile {
    const ALL: [(&'static str, Self); 7] = [
        ("quiet", Self::Quiet),
        ("chat-open", Self::ChatOpen),
        ("chat-run", Self::ChatRun),
        ("sidebar-churn", Self::SidebarChurn),
        ("popup-dropdown", Self::PopupDropdown),
        ("typing", Self::Typing),
        ("resize", Self::Resize),
    ];

    fn named(name: &str) -> Option<Self> {
        (Self::ALL.iter()).find_map(|(known, profile)| (*known == name).then_some(*profile))
    }

    fn name(self) -> &'static str {
        (Self::ALL.iter())
            .find_map(|(name, profile)| (*profile == self).then_some(*name))
            .unwrap_or("")
    }
}

fn act(action: Action) {
    canvas::dispatch(Event::Action(action));
}

fn millis_from_env(name: &str, default: u64) -> Duration {
    let read = std::env::var(name).ok().and_then(|text| text.parse().ok());
    Duration::from_millis(read.unwrap_or(default))
}

/// Whether the open thread's run is in flight.
fn running() -> bool {
    canvas::models().is_some_and(|models| {
        let transcript = models.chat.transcript.as_ref();
        transcript.is_some_and(|transcript| transcript.run.is_some())
    })
}

/// Puts the window in the state `profile` measures.
async fn set_up(
    profile: Profile,
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    debug_input::step(&format!("move {} {}", POINTER.x, POINTER.y), window, cx).await?;
    let chat = matches!(
        profile,
        Profile::ChatOpen | Profile::ChatRun | Profile::Typing
    );
    let models = canvas::models().ok_or("there is no canvas")?;
    if chat != models.chat.visible {
        act(Action::Chat(ChatAction::Toggle));
    }
    match profile {
        Profile::Quiet | Profile::ChatOpen | Profile::Resize => {}
        Profile::ChatRun => {
            act(Action::Chat(ChatAction::NewThread));
            act(Action::Chat(ChatAction::Send {
                text: "Walk the canvas and say what you see.".to_owned(),
                images: Vec::new(),
            }));
        }
        Profile::Typing => cx.update(|cx| {
            shell::with_view(cx, |view, window, cx| view.focus_composer(window, cx));
        }),
        Profile::SidebarChurn => {
            debug_input::step("sidebar on", window, cx).await?;
            let first = canvas::with(|canvas| {
                let document = canvas.runtime.app().document();
                document.entities().next().map(|entity| entity.id.clone())
            });
            let first = first.flatten().ok_or("the canvas has nothing to copy")?;
            act(Action::Select(vec![ItemId::Entity(first)]));
        }
        Profile::PopupDropdown => {
            act(Action::SetTool(Tool::AddShape));
            cx.background_executor().timer(SETTLE).await;
            debug_input::step("control zoom", window, cx).await?;
            debug_input::step(&format!("move {} {}", POINTER.x, POINTER.y), window, cx).await?;
        }
    }
    cx.background_executor().timer(SETTLE).await;
    match profile {
        Profile::ChatRun if !running() => {
            Err("no run started: SPECULAR_AGENT_SCRIPT must name a script".to_owned())
        }
        Profile::PopupDropdown if canvas::models().is_none_or(|models| models.popup.is_none()) => {
            Err("the shape tool hung no popup from the toolbar".to_owned())
        }
        _ => Ok(()),
    }
}

/// What a profile does as it is measured, a step at a time.
struct Busy {
    profile: Profile,
    started: Instant,
    typed: usize,
    typed_at: Instant,
    duplicated_at: Instant,
    renamed: u32,
    renamed_at: Instant,
    resized: u32,
}

impl Busy {
    fn new(profile: Profile, now: Instant) -> Self {
        Self {
            profile,
            started: now,
            typed: 0,
            typed_at: now,
            duplicated_at: now,
            renamed: 0,
            renamed_at: now,
            resized: 0,
        }
    }

    /// One refresh's worth.
    fn step(&mut self, now: Instant) -> Result<(), String> {
        if self.profile == Profile::Resize {
            // Out to the narrow size and back, a step a refresh.
            self.resized += 1;
            let leg = self.resized % (RESIZE_STEPS * 2);
            let along = leg.min(RESIZE_STEPS * 2 - leg) as f32 / RESIZE_STEPS as f32;
            debug_input::resize(WIDE.lerp(NARROW, along).round());
            return Ok(());
        }
        let legs = now.duration_since(self.started).as_millis() / PAN_LEG.as_millis();
        let direction = if legs.is_multiple_of(2) { 1.0 } else { -1.0 };
        debug_input::scroll(POINTER, Vec2::new(PAN_STEP * direction, 0.0));
        match self.profile {
            Profile::Typing if now.duration_since(self.typed_at) >= TYPE_EVERY => {
                self.typed_at = now;
                let at = self.typed % TYPED.len();
                self.typed += 1;
                debug_input::type_text(&TYPED[at..=at])?;
            }
            Profile::SidebarChurn => {
                if now.duration_since(self.duplicated_at) >= DUPLICATE_EVERY {
                    self.duplicated_at = now;
                    act(Action::Duplicate);
                }
                if now.duration_since(self.renamed_at) >= RENAME_EVERY {
                    self.renamed_at = now;
                    self.renamed += 1;
                    act(Action::Canvas(CanvasAction::Rename {
                        canvas: None,
                        name: format!("bench {}", self.renamed),
                    }));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// What the line says was measured.
fn subject(profile: Profile) -> String {
    let (name, source) = canvas::with(|canvas| {
        let name = canvas.runtime.app().space().active().name.clone();
        (name, canvas.runtime.source_name())
    })
    .unwrap_or_default();
    format!(
        "\"uiBench\":\"{}\",\"canvas\":\"{name}\",\"source\":\"{source}\"",
        profile.name()
    )
}

/// Measures `profile`. An error is why its numbers would mean nothing.
async fn measure(
    profile: Profile,
    window: AnyWindowHandle,
    cx: &mut AsyncApp,
) -> Result<String, String> {
    if let Some(why) = screen::why_nothing_shows() {
        return Err(why.to_owned());
    }
    // Named before the profile renames anything.
    let subject = subject(profile);
    set_up(profile, window, cx).await?;
    let refresh = canvas::with(|canvas| canvas.refresh()).ok_or("there is no canvas")?;
    let length = millis_from_env("SPECULAR_UI_BENCH_MS", 8000);
    let started = Instant::now();
    let mut busy = Busy::new(profile, started);
    ui_stats::start();
    while started.elapsed() < length {
        busy.step(Instant::now())?;
        cx.background_executor().timer(refresh).await;
    }
    let report = ui_stats::finish(refresh).ok_or("nothing was recorded")?;
    if let Some(why) = screen::why_nothing_shows() {
        return Err(why.to_owned());
    }
    let active = window.update(cx, |_, window, _| window.is_window_active());
    if !active.unwrap_or(false) {
        // GPUI draws a window that is not the active one 30 times a second.
        return Err("the window was not the active one".to_owned());
    }
    let canvas_fps = report.canvas_frames as f64 / report.seconds;
    if canvas_fps < SHOWN_FPS {
        return Err(format!(
            "the canvas presented {} frames in {:.1} s: the window is covered or not shown",
            report.canvas_frames, report.seconds
        ));
    }
    if profile == Profile::ChatRun && !running() {
        return Err("the run ended before the profile did".to_owned());
    }
    let hz = 1.0 / refresh.as_secs_f64();
    Ok(format!(
        "{{{subject},\"refreshHz\":{hz:.0},{}}}",
        report.json_fields()
    ))
}

/// Runs the profile the environment names, prints its line and quits.
pub(crate) fn run_from_env(window: AnyWindowHandle, cx: &mut App) {
    let Ok(name) = std::env::var("SPECULAR_UI_BENCH") else {
        return;
    };
    cx.spawn(async move |cx| {
        let Some(profile) = Profile::named(&name) else {
            let known: Vec<&str> = Profile::ALL.iter().map(|(name, _)| *name).collect();
            eprintln!(
                "specular: no UI bench profile `{name}` (one of {})",
                known.join(", ")
            );
            cx.update(shell::begin_exit);
            return;
        };
        let warmup = millis_from_env("SPECULAR_UI_BENCH_WARMUP_MS", 3000);
        cx.background_executor().timer(warmup).await;
        match measure(profile, window, cx).await {
            Ok(line) => println!("{line}"),
            Err(why) => {
                let why = why.replace('"', "'");
                println!(
                    "{{\"uiBench\":\"{}\",\"invalid\":\"{why}\"}}",
                    profile.name()
                );
            }
        }
        if profile == Profile::ChatRun {
            act(Action::Chat(ChatAction::Stop));
        }
        cx.update(shell::begin_exit);
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_profile_is_found_by_the_name_it_reports() {
        for (name, profile) in Profile::ALL {
            assert_eq!(Profile::named(name), Some(profile));
            assert_eq!(profile.name(), name);
        }
        assert_eq!(Profile::named("pan"), None);
    }
}
