//! The phase 0 numbers (ADR 0055): a scripted toolbar over `panels/toolbar`
//! renders only when a key it read moves, never while idle, and fast enough
//! for a frame. Run with `--release -- --nocapture` for the numbers the
//! phase gate reads; a debug build checks only the counts.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Entity, TestAppContext, VisualTestContext};
use serde_json::{Value, json};
use specular_mods::{Mod, ModHost, Reads, Revisions, RunError};

#[derive(Default)]
struct Recorder(RefCell<Vec<String>>);

impl ModHost for Recorder {
    fn run(&self, name: &str, args: &Value) -> Result<Value, RunError> {
        self.0.borrow_mut().push(format!("{name} {args}"));
        Ok(Value::Bool(true))
    }
    fn commands(&self) -> Value {
        json!([])
    }
    fn notify(&self, _: &str, _: &str) {}
}

fn toolbar(active: usize) -> Value {
    let names = [
        "select", "draw", "sticky", "shape", "page", "text", "document", "comment", "inspect",
    ];
    let groups: [&[usize]; 3] = [&[0], &[1, 2, 3, 4, 5, 6], &[7, 8]];
    let sections: Vec<Value> = (groups.iter())
        .map(|group| {
            json!({ "tools": group.iter().map(|&i| json!({
                "id": format!("tool.{}", names[i]),
                "label": names[i],
                "active": i == active,
            })).collect::<Vec<_>>() })
        })
        .collect();
    json!({ "sections": sections, "theme": { "id": "theme", "label": "Switch to dark theme" } })
}

struct Root(Entity<gpui_shell::ShellRoot>);

impl gpui::Render for Root {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

fn percentile(sorted: &[Duration], q: f64) -> Duration {
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

#[gpui::test]
fn scripted_toolbar_renders_on_change_only(cx: &mut TestAppContext) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/toolbar");
    let data = std::env::temp_dir().join(format!("specular-mods-test-{}", std::process::id()));
    let revisions = Rc::new(RefCell::new(Revisions::default()));
    let reads = Rc::new(RefCell::new(Reads::default()));
    revisions.borrow_mut().publish("panels/toolbar", toolbar(0));
    revisions
        .borrow_mut()
        .publish("mods/toolbar/props", json!({ "hidden": ["tool.inspect"] }));
    let host = Rc::new(Recorder::default());

    let started = Instant::now();
    let slot = Rc::new(RefCell::new(None));
    let window = {
        let (slot, revisions, reads, host) =
            (slot.clone(), revisions.clone(), reads.clone(), host.clone());
        cx.add_window(move |window, cx| {
            let loaded = Mod::load(
                "toolbar", &dir, &data, host, revisions, &reads, false, window, cx,
            )
            .expect("the mod loads");
            let root = Root(loaded.root().clone());
            *slot.borrow_mut() = Some(loaded);
            root
        })
    };
    let mut vcx = VisualTestContext::from_window(*window, cx);
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    let loaded = slot.borrow_mut().take().expect("loaded");
    println!("load and first render: {:?}", started.elapsed());
    assert_eq!(loaded.stats().renders, 1, "one render to start");
    assert_eq!(
        reads.borrow().read_by("toolbar"),
        ["mods/toolbar/props", "panels/toolbar"]
    );

    // Idle: frames with nothing changed never enter the script.
    for _ in 0..100 {
        vcx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
    assert_eq!(loaded.stats().renders, 1, "no script render while idle");

    // A key it did not read moves: no render.
    revisions
        .borrow_mut()
        .publish("panels/dock", json!({ "controls": [] }));
    let rendered = vcx.update(|_, cx| loaded.refresh_if_stale(&revisions.borrow(), &reads, cx));
    assert!(!rendered, "the dock moving leaves the toolbar alone");

    // A key it read moves: one render each time.
    let samples = 300;
    let mut times = Vec::with_capacity(samples);
    for i in 0..samples {
        revisions
            .borrow_mut()
            .publish("panels/toolbar", toolbar((i + 1) % 9));
        let before = loaded.stats();
        vcx.update(|window, cx| {
            assert!(loaded.refresh_if_stale(&revisions.borrow(), &reads, cx));
            window.draw(cx).clear(cx);
        });
        let after = loaded.stats();
        assert_eq!(after.renders, before.renders + 1);
        times.push(after.render_time.saturating_sub(before.render_time));
    }
    times.sort();
    let (median, p95, max) = (
        percentile(&times, 0.5),
        percentile(&times, 0.95),
        times[times.len() - 1],
    );
    println!("script render over {samples}: median {median:?}, p95 {p95:?}, max {max:?}");
    if !cfg!(debug_assertions) {
        assert!(
            median < Duration::from_millis(2),
            "median {median:?} is over 2 ms"
        );
        assert!(
            max < Duration::from_millis(8),
            "slowest {max:?} is over 8 ms"
        );
    }
}
