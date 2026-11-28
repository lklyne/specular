//! What every panel's gate test shares (ADR 0055, phase 4): a shipped mod
//! installed as the app installs it, loaded over a real app's keyed state
//! in a headless window of the size its region gives it, and the three
//! measures the gate reads: renders while idle, renders on a camera move,
//! and the time of a script render when its state moves.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use glam::Vec2;
use gpui::{
    AnyView, Modifiers, ParentElement as _, Pixels, Point, Styled as _, TestAppContext,
    VisualTestContext, div, point, px,
};
use serde_json::{Value, json};
use specular_interact::{Event, WheelInput, publish_state, state};
use specular_mods::{Hosting, Mod, ModHost, RunError, shipped};
use specular_testkit::TestApp;

/// A host that remembers the commands it was asked to run.
#[derive(Default)]
pub(crate) struct Recorder(pub(crate) RefCell<Vec<String>>);

impl ModHost for Recorder {
    fn run(&self, name: &str, args: &Value) -> Result<Value, RunError> {
        self.0.borrow_mut().push(format!("{name} {args}"));
        Ok(json!({ "ok": true, "name": name }))
    }
    fn commands(&self) -> Value {
        json!([])
    }
    fn notify(&self, _: &str, _: &str) {}
}

/// What makes the app a case of a table starts from.
pub(crate) type Fixture = fn() -> TestApp;

/// An app as the Kit window starts one: its own chrome, a right panel.
pub(crate) fn window_app() -> TestApp {
    let mut app = TestApp::with_pages(1);
    app.send(Event::ShellChrome);
    app.send(Event::ChatPanel(true));
    app
}

/// A folder of its own under the system's temporary folder.
pub(crate) fn scratch() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("specular-panels-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

/// The box a region gives its mod.
struct Root {
    view: AnyView,
    size: (f32, f32),
}

impl gpui::Render for Root {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        div()
            .w(px(self.size.0))
            .h(px(self.size.1))
            .child(self.view.clone())
    }
}

/// A shipped mod over an app's state, drawn once in a window of its own.
pub(crate) struct Loaded {
    pub(crate) panel: Mod,
    pub(crate) hosting: Hosting,
    pub(crate) host: Rc<Recorder>,
    pub(crate) vcx: VisualTestContext,
}

impl Loaded {
    /// The shipped mod `name`, with `props`, in a box `size` points across,
    /// over `app`'s state.
    pub(crate) fn new(
        app: &TestApp,
        name: &'static str,
        props: Value,
        size: (f32, f32),
        cx: &mut TestAppContext,
    ) -> Self {
        // The base layer's globals, which the window has from its start.
        cx.update(gpui_shell::init);
        let mods = scratch();
        shipped::install(&mods).expect("the shipped mods are written");
        let dir = mods.join(name);
        let data = mods.join(".data");
        let host = Rc::new(Recorder::default());
        let hosting = Hosting::new(host.clone());
        publish_state(&mut hosting.revisions.borrow_mut(), app.app());
        (hosting.revisions.borrow_mut()).publish(&format!("mods/{name}/props"), props);
        let slot = Rc::new(RefCell::new(None));
        let window = {
            let (slot, hosting) = (slot.clone(), hosting.clone());
            cx.add_window(move |window, cx| {
                let panel = Mod::load(name, &dir, &data, &hosting, window, cx)
                    .expect("the shipped mod loads");
                let root = Root {
                    view: panel.view(cx),
                    size,
                };
                *slot.borrow_mut() = Some(panel);
                root
            })
        };
        let mut vcx = VisualTestContext::from_window(*window, cx);
        vcx.simulate_resize(gpui::size(px(1280.0), px(800.0)));
        vcx.update(|window, cx| window.draw(cx).clear(cx));
        let panel = slot.borrow_mut().take().expect("loaded");
        Self {
            panel,
            hosting,
            host,
            vcx,
        }
    }

    /// The element tree the mod's last render described.
    pub(crate) fn tree(&mut self) -> String {
        let panel = &self.panel;
        self.vcx
            .update(|_, cx| panel.tree(cx))
            .expect("the mod rendered")
    }

    /// The keys the mod read.
    pub(crate) fn reads(&self, name: &str) -> Vec<String> {
        self.hosting.reads.borrow().read_by(name)
    }

    /// Publishes `app`'s state and renders the mod again if a key it read
    /// moved, then draws a frame. Returns whether the script rendered.
    pub(crate) fn step(&mut self, app: &TestApp) -> bool {
        publish_state(&mut self.hosting.revisions.borrow_mut(), app.app());
        let (panel, hosting) = (&self.panel, &self.hosting);
        self.vcx.update(|window, cx| {
            let rendered = panel.refresh_if_stale(hosting, cx);
            window.draw(cx).clear(cx);
            rendered
        })
    }

    /// Script renders over 100 frames with nothing changed.
    pub(crate) fn idle_renders(&mut self) -> u64 {
        let before = self.panel.stats().renders;
        for _ in 0..100 {
            self.vcx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }
        self.panel.stats().renders - before
    }

    /// Pans and zooms `app`, and checks that no key moved and the script
    /// was not entered.
    pub(crate) fn assert_camera_moves_nothing(&mut self, app: &mut TestApp) {
        let before = state(app.app());
        app.send(Event::Wheel(WheelInput {
            delta: Vec2::new(40.0, 25.0),
            modifiers: specular_core::Modifiers::default(),
        }));
        app.zoom(1.6);
        assert_eq!(state(app.app()), before, "a camera move changes no key");
        assert!(!self.step(app), "a camera move renders nothing");
    }

    /// The commands the host was asked to run, in order.
    pub(crate) fn runs(&self) -> Vec<String> {
        self.host.0.borrow().clone()
    }

    /// Clicks at each of `points` in turn until the host has been asked to
    /// run `wanted`. Returns every command it was asked to run.
    pub(crate) fn click_until(
        &mut self,
        points: impl IntoIterator<Item = Point<Pixels>>,
        wanted: &str,
    ) -> Vec<String> {
        self.click_until_run(points, |run| run == wanted)
    }

    /// Clicks at each of `points` in turn until the host has been asked to
    /// run a command `wanted` accepts. Returns every command it was asked
    /// to run.
    pub(crate) fn click_until_run(
        &mut self,
        points: impl IntoIterator<Item = Point<Pixels>>,
        wanted: impl Fn(&str) -> bool,
    ) -> Vec<String> {
        for at in points {
            self.vcx.simulate_click(at, Modifiers::default());
            self.vcx.run_until_parked();
            if self.runs().iter().any(|run| wanted(run)) {
                break;
            }
        }
        self.runs()
    }
}

/// Points across a row `y` down, every `step` points from `from` to `to`.
pub(crate) fn along(y: f32, (from, to): (u32, u32), step: usize) -> Vec<Point<Pixels>> {
    (from..to)
        .step_by(step)
        .map(|x| point(px(x as f32), px(y)))
        .collect()
}

/// Points in a grid over `(width, height)`, every `step` points.
pub(crate) fn grid((width, height): (u32, u32), step: usize) -> Vec<Point<Pixels>> {
    (2..height)
        .step_by(step)
        .flat_map(|y| along(y as f32, (2, width), step))
        .collect()
}

fn percentile(sorted: &[Duration], q: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// Changes `app` with `change` at each of `samples` steps, checks that the
/// mod renders once each time, and prints the times. In a release build
/// the median must be under 2 ms and none over 8 ms, the phase 3 gate.
pub(crate) fn assert_renders_inside_a_frame(
    loaded: &mut Loaded,
    app: &mut TestApp,
    label: &str,
    samples: usize,
    mut change: impl FnMut(&mut TestApp, usize),
) {
    let mut times = Vec::with_capacity(samples);
    let mut cpu = Vec::with_capacity(samples);
    for i in 0..samples {
        change(app, i);
        let before = loaded.panel.stats();
        let cpu_before = thread_cpu();
        assert!(loaded.step(app), "{label}: step {i} renders the mod");
        cpu.push(thread_cpu().saturating_sub(cpu_before));
        let after = loaded.panel.stats();
        assert_eq!(after.renders, before.renders + 1);
        times.push(after.render_time.saturating_sub(before.render_time));
    }
    // Each slow render with the CPU its whole step took on this thread.
    let slow: Vec<(usize, Duration, Duration)> = (times.iter().copied().enumerate())
        .filter(|(_, time)| *time > Duration::from_millis(2))
        .map(|(step, time)| (step, time, cpu[step]))
        .collect();
    // The slowest render the script itself worked through: a render the
    // runner paused takes wall time the thread did not spend, so each is
    // held to the CPU its whole step took when that is less.
    let worked = (times.iter().zip(&cpu))
        .map(|(wall, cpu)| (*wall).min(*cpu))
        .max()
        .unwrap_or_default();
    times.sort();
    let (median, p95, max) = (
        percentile(&times, 0.5),
        percentile(&times, 0.95),
        times[times.len() - 1],
    );
    println!("{label}: script render over {samples}: median {median:?}, p95 {p95:?}, max {max:?}");
    println!("{label}: renders over 2 ms (step, script wall, step thread CPU): {slow:?}");
    if !cfg!(debug_assertions) {
        assert!(
            median < Duration::from_millis(2),
            "{label}: median {median:?} is over 2 ms"
        );
        assert!(
            worked < Duration::from_millis(8),
            "{label}: slowest {worked:?} of work (wall {max:?}) is over 8 ms"
        );
    }
}

/// Loads the mod and prints how long the load and first render took.
pub(crate) fn timed_load(
    app: &TestApp,
    name: &'static str,
    props: Value,
    size: (f32, f32),
    cx: &mut TestAppContext,
) -> Loaded {
    let started = Instant::now();
    let loaded = Loaded::new(app, name, props, size, cx);
    println!("{name}: load and first render: {:?}", started.elapsed());
    assert_eq!(
        loaded.panel.stats().renders,
        1,
        "{name}: one render to start"
    );
    loaded
}

/// CPU time this thread has spent. Beside a wall-clock reading it tells a
/// render that did a lot of work from one whose thread was preempted, as a
/// shared CI runner does to it now and then.
fn thread_cpu() -> Duration {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid, writable timespec for the call's duration.
    let ok = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &raw mut now) } == 0;
    if !ok {
        return Duration::ZERO;
    }
    Duration::new(now.tv_sec as u64, now.tv_nsec as u32)
}
