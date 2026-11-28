//! The phase 3 gate (ADR 0055): the shipped toolbar mod over the keyed
//! state of a real app. It shows every control the native toolbar's model
//! lists, leaves out what its props hide, presses a tool through the host,
//! renders only when a key it read moves (never while idle, never for a
//! camera move), and fast enough for a frame. Run with `--release --
//! --nocapture` for the numbers the gate reads; a debug build checks only
//! the counts.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

mod parity;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use glam::Vec2;
use gpui::{AnyView, ParentElement as _, Styled as _, TestAppContext, VisualTestContext, div, px};
use serde_json::{Value, json};
use specular_interact::{Action, Event, Tool, WheelInput, publish_state, state};
use specular_mods::{Hosting, Mod, ModHost, RunError, shipped};
use specular_testkit::TestApp;

/// A host that remembers the commands it was asked to run.
#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

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

/// An app as the Kit window starts one: its own chrome, a right panel.
fn window_app() -> TestApp {
    let mut app = TestApp::with_pages(1);
    app.send(Event::ShellChrome);
    app.send(Event::ChatPanel(true));
    app
}

/// A folder of its own under the system's temporary folder.
fn scratch() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("specular-mods-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

/// The docked toolbar's box, as the window gives it to the mod: a strip
/// down the left edge.
struct Root(AnyView);

impl gpui::Render for Root {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        div()
            .w(px(specular_interact::TOOLS_DOCK_WIDTH))
            .h_full()
            .child(self.0.clone())
    }
}

/// The shipped toolbar, installed as the app installs it, loaded over
/// `app`'s state with `hidden` as its props, in a window of its own and
/// drawn once.
struct Loaded {
    toolbar: Mod,
    hosting: Hosting,
    host: Rc<Recorder>,
    vcx: VisualTestContext,
}

impl Loaded {
    fn new(app: &TestApp, hidden: &[&str], cx: &mut TestAppContext) -> Self {
        let mods = scratch();
        shipped::install(&mods).expect("the shipped mods are written");
        let dir = mods.join("toolbar");
        let data = mods.join(".data");
        let host = Rc::new(Recorder::default());
        let hosting = Hosting::new(host.clone());
        publish_state(&mut hosting.revisions.borrow_mut(), app.app());
        (hosting.revisions.borrow_mut()).publish("mods/toolbar/props", json!({ "hidden": hidden }));
        let slot = Rc::new(RefCell::new(None));
        let window = {
            let (slot, hosting) = (slot.clone(), hosting.clone());
            cx.add_window(move |window, cx| {
                let toolbar = Mod::load("toolbar", &dir, &data, &hosting, false, window, cx)
                    .expect("the shipped toolbar loads");
                let root = Root(toolbar.view(cx));
                *slot.borrow_mut() = Some(toolbar);
                root
            })
        };
        let mut vcx = VisualTestContext::from_window(*window, cx);
        vcx.simulate_resize(gpui::size(gpui::px(800.0), gpui::px(600.0)));
        vcx.update(|window, cx| window.draw(cx).clear(cx));
        let toolbar = slot.borrow_mut().take().expect("loaded");
        Self {
            toolbar,
            hosting,
            host,
            vcx,
        }
    }

    fn tree(&mut self) -> String {
        let toolbar = &self.toolbar;
        self.vcx
            .update(|_, cx| toolbar.tree(cx))
            .expect("the toolbar rendered")
    }

    /// Publishes `app`'s state and renders the mod again if a key it read
    /// moved, then draws a frame. Returns whether the script rendered.
    fn step(&mut self, app: &TestApp) -> bool {
        publish_state(&mut self.hosting.revisions.borrow_mut(), app.app());
        let (toolbar, hosting) = (&self.toolbar, &self.hosting);
        self.vcx.update(|window, cx| {
            let rendered = toolbar.refresh_if_stale(hosting, cx);
            window.draw(cx).clear(cx);
            rendered
        })
    }
}

fn percentile(sorted: &[Duration], q: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

#[gpui::test]
fn the_toolbar_renders_on_change_only_and_inside_a_frame(cx: &mut TestAppContext) {
    let mut app = window_app();
    let started = Instant::now();
    let mut loaded = Loaded::new(&app, &[], cx);
    println!("load and first render: {:?}", started.elapsed());
    assert_eq!(loaded.toolbar.stats().renders, 1, "one render to start");
    assert_eq!(
        loaded.hosting.reads.borrow().read_by("toolbar"),
        [
            "mods/toolbar/props",
            "panels/tool_options",
            "panels/toolbar",
            "theme"
        ]
    );

    // Idle: frames with nothing changed never enter the script.
    for _ in 0..100 {
        loaded.vcx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
    assert_eq!(
        loaded.toolbar.stats().renders,
        1,
        "no script render while idle"
    );

    // The camera is in no key: a pan and a zoom move nothing, and the
    // script is not entered.
    let before = state(app.app());
    app.send(Event::Wheel(WheelInput {
        delta: Vec2::new(40.0, 25.0),
        modifiers: specular_core::Modifiers::default(),
    }));
    app.zoom(1.6);
    assert_eq!(state(app.app()), before, "a camera move changes no key");
    assert!(!loaded.step(&app), "a camera move renders nothing");

    // The tool changes each step: one render each time.
    let tools: Vec<Tool> = [
        "select", "draw", "sticky", "shape", "page", "text", "document", "comment", "inspect",
    ]
    .into_iter()
    .map(|name| Tool::named(name).expect("a tool"))
    .collect();
    let samples = 300;
    let mut times = Vec::with_capacity(samples);
    let mut cpu = Vec::with_capacity(samples);
    for i in 0..samples {
        app.act(Action::SetTool(tools[(i + 1) % tools.len()]));
        let before = loaded.toolbar.stats();
        let cpu_before = thread_cpu();
        assert!(loaded.step(&app), "a tool change renders the toolbar");
        cpu.push(thread_cpu().saturating_sub(cpu_before));
        let after = loaded.toolbar.stats();
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
    println!("script render over {samples}: median {median:?}, p95 {p95:?}, max {max:?}");
    println!("renders over 2 ms (step, script wall, step thread CPU): {slow:?}");
    if !cfg!(debug_assertions) {
        assert!(
            median < Duration::from_millis(2),
            "median {median:?} is over 2 ms"
        );
        assert!(
            worked < Duration::from_millis(8),
            "slowest {worked:?} of work (wall {max:?}) is over 8 ms"
        );
    }
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
