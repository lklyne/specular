//! Question 10 of the spike (step S7, ADR 0055): the toolbar as a Luau
//! script, on Rune's GPU-free `Runtime`. Each test prints what it measured;
//! run with `--nocapture` to read it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use glam::Vec2;
use rune::{Env, Runtime};
use rune_luau::{Data, Metrics, Script, ScriptOptions, Sources};
use rune_shell_spike::shell::Shell;
use rune_shell_spike::{Chrome, Options, app, toolbar_panel};
use serde_json::Value;
use specular_interact::{Action, Event, Tool, WheelInput};

const DT: f64 = 1.0 / 60.0;

fn runtime(chrome: Chrome) -> Runtime<Shell> {
    let canvas =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kitchen-sink.canvas");
    let options = Options {
        canvas: Some(canvas),
        still: true,
        chrome,
        ..Options::default()
    };
    let size = options.size;
    let mut rt = app(options).headless_runtime(size, Env::new(false, 1.0));
    rt.frame(0.0);
    rt
}

fn tap(rt: &mut Runtime<Shell>, key: &str) {
    let (x, y) = rt
        .find(key)
        .unwrap_or_else(|| panic!("no element keyed {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    for _ in 0..3 {
        rt.frame(DT);
    }
}

fn keyed(node: &Value, key: &str) -> Option<Value> {
    if node["key"] == key {
        return Some(node.clone());
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|child| keyed(child, key))
}

/// The accessibility subtree under the ribbon: every control's key, role,
/// name, states and actions.
fn toolbar_tree(rt: &Runtime<Shell>) -> Value {
    keyed(&rt.accessibility_json(), "toolbar").expect("a node keyed toolbar")
}

fn script(rt: &Runtime<Shell>) -> &Script {
    &rt.state.panel.as_ref().expect("a scripted panel").script
}

/// A copy of the shipped toolbar script in a folder a test may rewrite.
fn scratch_panel(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    std::fs::copy(toolbar_panel().join("init.luau"), dir.join("init.luau")).expect("the script");
    dir
}

/// Steps frames until `done`, giving the file watcher's thread time.
fn frames_until(rt: &mut Runtime<Shell>, done: impl Fn(&Runtime<Shell>) -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        rt.frame(DT);
        if done(rt) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn the_scripted_toolbar_has_the_rust_toolbars_tree_and_runs_its_actions() {
    let mut rust = runtime(Chrome::Rust);
    let mut luau = runtime(Chrome::Luau(toolbar_panel()));
    assert!(
        script(&luau).error().is_none(),
        "{:?}",
        script(&luau).error()
    );
    assert_eq!(toolbar_tree(&rust), toolbar_tree(&luau), "at rest");

    // A press in the script reaches `update` as the control's own `Action`.
    for rt in [&mut rust, &mut luau] {
        tap(rt, "tool.sticky");
        assert_eq!(rt.state.app.session().tool, Tool::AddSticky);
    }
    assert_eq!(toolbar_tree(&rust), toolbar_tree(&luau), "sticky armed");

    // The zoom list: open is the renderer's state on both sides.
    for rt in [&mut rust, &mut luau] {
        tap(rt, "zoom");
        assert!(rt.find("zoom.50").is_some(), "the list is open");
    }
    assert_eq!(toolbar_tree(&rust), toolbar_tree(&luau), "zoom list open");
    for rt in [&mut rust, &mut luau] {
        tap(rt, "zoom.50");
        assert!(rt.find("zoom.50").is_none(), "the list closed");
        assert_eq!(rt.state.app.session().camera.zoom, 0.5);
    }
    assert_eq!(toolbar_tree(&rust), toolbar_tree(&luau), "zoomed to 50%");
}

#[test]
fn a_saved_edit_reloads_the_panel_and_a_bad_one_keeps_the_last_good() {
    let dir = scratch_panel("reload");
    let file = dir.join("init.luau");
    let shipped = std::fs::read_to_string(&file).expect("the script");
    let mut rt = runtime(Chrome::Luau(dir.clone()));

    // App state and the script's own state (the open list) to carry over.
    tap(&mut rt, "tool.sticky");
    tap(&mut rt, "zoom");
    assert!(rt.find("zoom.list").is_some());
    let before = toolbar_tree(&rt);
    assert_eq!(before["name"], "Tools");

    // 1. A save, picked up by the watcher: no rebuild, no call from here.
    let edited = shipped
        .replace(r#"label = "Tools""#, r#"label = "Instruments""#)
        .replace(
            "table.insert(items, pressable(model.theme, \"theme\", nil))\n\ttable.insert(items, pressable(model.share, \"share\", nil))",
            "table.insert(items, pressable(model.share, \"share\", nil))\n\ttable.insert(items, pressable(model.theme, \"theme\", nil))",
        );
    assert_ne!(edited, shipped);
    let saved = Instant::now();
    std::fs::write(&file, &edited).expect("a save");
    assert!(
        frames_until(&mut rt, |rt| script(rt).metrics().reloads == 1),
        "the watcher reloaded the panel"
    );
    let waited = saved.elapsed();
    rt.frame(DT);
    let after = toolbar_tree(&rt);
    assert_eq!(after["name"], "Instruments", "the edit shows");
    let keys = |tree: &Value| -> Vec<String> {
        (tree["children"].as_array().expect("children").iter())
            .filter_map(|child| child["key"].as_str().map(str::to_owned))
            .collect()
    };
    let (was, is) = (keys(&before), keys(&after));
    assert_eq!(&was[was.len() - 2..], ["theme", "share.copy"]);
    assert_eq!(&is[is.len() - 2..], ["share.copy", "theme"], "reordered");
    assert_eq!(rt.state.app.session().tool, Tool::AddSticky, "app state");
    assert!(rt.find("zoom.list").is_some(), "the script's state");
    let reload = script(&rt).metrics().last_reload.expect("a reload");
    println!(
        "hot reload: {waited:?} from save to swapped; compile {:?}, swap {:?}",
        reload.compile, reload.swap
    );

    // 2. A save that does not compile: the last good version keeps running.
    let broken = edited.replace("local function ribbon()", "local function ribbon(");
    std::fs::write(&file, &broken).expect("a save");
    let errors = |rt: &Runtime<Shell>| {
        (script(rt).log().iter())
            .filter(|line| line.level == rune_luau::LogLevel::Error)
            .count()
    };
    assert!(
        frames_until(&mut rt, |rt| errors(rt) > 0),
        "the error is logged"
    );
    let message = script(&rt).log().last().expect("a line").message.clone();
    println!("a syntax error on save logs: {message}");
    assert_eq!(script(&rt).metrics().reloads, 1, "no swap");
    assert!(script(&rt).error().is_none(), "still running");
    assert_eq!(toolbar_tree(&rt), after, "still the last good panel");
    tap(&mut rt, "tool.shape");
    assert_eq!(rt.state.app.session().tool, Tool::AddShape, "and it works");

    // 3. The same through `reload`, which says why.
    let refused = script(&rt).reload(Sources::single(broken));
    println!("reload() of the same: {:?}", refused.as_ref().err());
    assert!(refused.is_err());

    // 4. A script that compiles and fails as it starts is refused too.
    let failing = format!("error(\"boom at start\")\n{shipped}");
    assert!(script(&rt).reload(Sources::single(failing)).is_err());
    assert!(script(&rt).error().is_none());

    // 5. A script that starts and fails in `render` is swapped in and shows
    //    Rune's error card in place of the panel. The app carries on.
    let throwing = shipped.replace(
        "local function ribbon()\n",
        "local function ribbon()\n\terror(\"boom in render\")\n",
    );
    assert_ne!(throwing, shipped);
    assert!(script(&rt).reload(Sources::single(throwing)).is_ok());
    // A reload called from outside a frame asks for no frame itself.
    rt.update(|_, _| {});
    rt.frame(DT);
    assert!(rt.find("tool.shape").is_none(), "the panel is gone");
    assert_eq!(rt.state.app.session().tool, Tool::AddShape);
    println!(
        "a render error leaves: {}",
        script(&rt)
            .log()
            .last()
            .expect("a line")
            .message
            .lines()
            .next()
            .unwrap_or_default()
    );

    // 6. The fix, saved: back to the shipped panel, list still open.
    std::fs::write(&file, &shipped).expect("a save");
    let reloads = script(&rt).metrics().reloads;
    assert!(frames_until(&mut rt, |rt| script(rt).metrics().reloads > reloads));
    rt.frame(DT);
    assert_eq!(toolbar_tree(&rt)["name"], "Tools");
    assert!(rt.find("tool.shape").is_some());
}

/// One row of the cost table: what `step` made the script do.
fn cost(
    rt: &mut Runtime<Shell>,
    what: &str,
    frames: u32,
    mut step: impl FnMut(&mut Runtime<Shell>),
) -> Metrics {
    let panel = rt.state.panel.as_ref().expect("a scripted panel");
    let (before, sends) = (panel.script.metrics(), panel.sends.get());
    let start = Instant::now();
    for _ in 0..frames {
        step(rt);
        rt.frame(DT);
    }
    let wall = start.elapsed();
    let panel = rt.state.panel.as_ref().expect("a scripted panel");
    let after = panel.script.metrics();
    let delta = Metrics {
        vm_calls: after.vm_calls - before.vm_calls,
        vm_time: after.vm_time - before.vm_time,
        renders: after.renders - before.renders,
        ..Metrics::default()
    };
    println!(
        "cost: {what}: {frames} frame(s), {} snapshot(s) sent, {} VM entries, {} render(s), {:?} in the VM, {:?} wall a frame",
        panel.sends.get() - sends,
        delta.vm_calls,
        delta.renders,
        delta.vm_time,
        wall / frames,
    );
    delta
}

fn pan(rt: &mut Runtime<Shell>) {
    rt.update(|shell, _| {
        shell.dispatch(Event::Wheel(WheelInput {
            delta: Vec2::new(7.0, 5.0),
            modifiers: Default::default(),
        }));
    });
}

#[test]
fn the_script_runs_on_a_snapshot_change_or_a_press_and_at_no_other_time() {
    let mut rt = runtime(Chrome::Luau(toolbar_panel()));
    let start = script(&rt).metrics();
    println!(
        "cost: start to first frame: {} VM entries, {} render(s), {:?} in the VM; heap {} bytes",
        start.vm_calls,
        start.renders,
        start.vm_time,
        script(&rt).memory()
    );

    let idle = cost(&mut rt, "idle", 100, |_| {});
    assert_eq!(idle.vm_calls, 0);

    // The view is built again, and the toolbar model has not changed.
    let rebuilt = cost(&mut rt, "view rebuilt, nothing changed", 100, |rt| {
        rt.update(|_, _| {})
    });
    assert_eq!(rebuilt.vm_calls, 0);

    // A pan moves the camera and nothing the toolbar shows.
    let at = rt.state.app.session().camera.pan;
    let panned = cost(&mut rt, "pan", 100, pan);
    assert_ne!(rt.state.app.session().camera.pan, at, "the camera moved");
    assert_eq!(panned.vm_calls, 0, "a pan never enters the script");

    // A selection of a sticky: the document's state, not the toolbar's.
    let sticky = (rt.state.app.document().entities())
        .find(|entity| entity.id.as_str().starts_with("sticky"))
        .map(|entity| entity.id.clone());
    if let Some(sticky) = sticky {
        let selected = cost(&mut rt, "select a sticky", 1, |rt| {
            let id = sticky.clone();
            rt.update(|shell, _| shell.act(Action::Select(vec![id.into()])));
        });
        println!("  (a selection sent {} VM entries)", selected.vm_calls);
    }

    // A tool change by a press: the handler, the snapshot, the render.
    let mut tools = ["tool.shape", "tool.draw"].into_iter().cycle();
    let pressed = cost(&mut rt, "tool change by a press", 20, |rt| {
        let (x, y) = rt.find(tools.next().unwrap_or_default()).expect("a tool");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    });
    assert_eq!(pressed.renders, 20, "one render a change");
    assert_eq!(pressed.vm_calls, 60, "handler + snapshot + render");

    // A tool change from elsewhere (a key, the API): snapshot and render.
    let mut next = [Tool::AddSticky, Tool::Select].into_iter().cycle();
    let acted = cost(&mut rt, "tool change by an action", 20, |rt| {
        let tool = next.next().unwrap_or(Tool::Select);
        rt.update(|shell, _| shell.act(Action::SetTool(tool)));
    });
    assert_eq!(acted.vm_calls, 40);

    // A zoom step changes the readout, which is in the snapshot: the one
    // way the camera reaches the script.
    let zoomed = cost(&mut rt, "zoom step", 10, |rt| {
        rt.update(|shell, _| shell.act(Action::ZoomIn));
    });
    println!(
        "  (10 zoom steps rendered the panel {} times)",
        zoomed.renders
    );

    // A pinch: the camera's zoom moves every frame, and the readout with
    // it whenever its whole percent does.
    let pinched = cost(&mut rt, "pinch", 100, |rt| {
        rt.update(|shell, _| {
            shell.dispatch(Event::Pinch { delta: 0.004 });
        });
    });
    println!(
        "  (100 frames of pinch rendered the panel {} times, zoom now {})",
        pinched.renders,
        rt.state.app.session().camera.zoom
    );

    let held = script(&rt).memory();
    script(&rt).collect_garbage();
    println!(
        "cost: heap {held} bytes, {} after a full collection",
        script(&rt).memory()
    );
}

#[test]
fn the_rust_toolbar_for_the_same_steps() {
    let mut rt = runtime(Chrome::Rust);
    let mut time = |what: &str, frames: u32, step: &mut dyn FnMut(&mut Runtime<Shell>)| {
        let start = Instant::now();
        for _ in 0..frames {
            step(&mut rt);
            rt.frame(DT);
        }
        println!(
            "cost (rust chrome): {what}: {:?} wall a frame",
            start.elapsed() / frames
        );
    };
    time("idle", 100, &mut |_| {});
    time("view rebuilt, nothing changed", 100, &mut |rt| {
        rt.update(|_, _| {})
    });
    time("pan", 100, &mut pan);
    let mut next = [Tool::AddSticky, Tool::Select].into_iter().cycle();
    time("tool change by an action", 20, &mut |rt| {
        let tool = next.next().unwrap_or(Tool::Select);
        rt.update(|shell, _| shell.act(Action::SetTool(tool)));
    });
}

const PROBE: &str = r#"
local function try(f, ...)
	local ok, why = pcall(f, ...)
	return if ok then "ok: " .. tostring(why) else "refused: " .. tostring(why)
end
local function names(t)
	local out = {}
	for name in t do
		table.insert(out, tostring(name))
	end
	table.sort(out)
	return table.concat(out, " ")
end
return {
	render = function() return nil end,
	reach = function()
		return {
			io = type(io),
			os = if os then names(os) else "nil",
			debug = if debug then names(debug) else "nil",
			loadstring = type(loadstring),
			dofile = type(dofile),
			loadfile = type(loadfile),
			package = type(package),
			getfenv = type(getfenv),
			specular = try(require, "@specular/app"),
			outside = try(require, "../outside"),
			sibling = try(require, "./sibling"),
			absolute = try(require, "/etc/passwd"),
			missing = try(require, "@self/missing"),
			setGlobal = try(function() _G.leak = 1; leak2 = 1 end),
			patchString = try(function() string.rep = nil end),
			patchUi = try(function() require("@rune/ui").row = nil end),
		}
	end,
	spin = function() while true do end end,
	hog = function()
		local t = {}
		for i = 1, 1e9 do
			t[i] = string.rep("x", 1024) .. i
		end
	end,
	recurse = function()
		local function f() return 1 + f() end
		return f()
	end,
	alive = function() return "still here" end,
}
"#;

#[test]
fn what_a_script_can_reach_by_default() {
    let script = Script::with_options("probe", Sources::single(PROBE), ScriptOptions::new());
    assert!(script.error().is_none(), "{:?}", script.error());
    let Data::Map(reach) = script.query("reach", []).expect("reach") else {
        panic!("reach returns a table");
    };
    for (name, value) in &reach {
        println!("sandbox: {name} = {}", value.as_str().unwrap_or("?"));
    }
    for absent in ["io", "dofile", "loadfile", "package"] {
        assert_eq!(reach[absent].as_str(), Some("nil"), "{absent}");
    }
    // `patchUi` is not here: a script can rewrite `@rune/ui` in its own VM.
    for refused in [
        "specular",
        "outside",
        "sibling",
        "absolute",
        "missing",
        "patchString",
        "setGlobal",
    ] {
        assert!(
            reach[refused]
                .as_str()
                .is_some_and(|said| said.starts_with("refused")),
            "{refused}: {:?}",
            reach[refused]
        );
    }

    for runaway in ["spin", "recurse", "hog"] {
        let start = Instant::now();
        let stopped = script.query(runaway, []);
        println!(
            "sandbox: {runaway}() stopped after {:?}: {}",
            start.elapsed(),
            stopped.as_ref().err().map_or("it returned", |error| error
                .message
                .lines()
                .next()
                .unwrap_or_default())
        );
        assert!(stopped.is_err(), "{runaway}");
        // The VM is still usable after each.
        assert_eq!(
            script.query("alive", []).expect("alive").as_str(),
            Some("still here")
        );
    }
    let full = script.memory();
    script.collect_garbage();
    println!(
        "sandbox: heap after the runaways {full} bytes, {} after a full collection",
        script.memory()
    );
}
