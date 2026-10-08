//! The scripted scenarios. Each drives the window with synthesized input,
//! captures it with `screencapture`, and writes a report next to the shots.

use std::process::{Child, Command};
use std::time::{Duration, Instant};

use gpui_kit::*;

use crate::Options;
use crate::canvas;
use crate::log::{self, Pacing};
use crate::native::Synth;
use crate::shell::Shell;

/// Toolbar geometry, in points from the content view's top-left.
const DIALOG_BUTTON: (f64, f64) = (350.0, 22.0);
const MENU_BUTTON: (f64, f64) = (468.0, 22.0);
const POPOVER_BUTTON: (f64, f64) = (586.0, 22.0);
const SPLIT_HANDLE: (f64, f64) = (260.0, 400.0);
const CANVAS_POINT: (f64, f64) = (760.0, 420.0);

struct Run {
    options: Options,
    window: AnyWindowHandle,
    shell: Entity<Shell>,
    report: Vec<String>,
    captures: Vec<Child>,
}

async fn sleep(cx: &mut AsyncApp, ms: u64) {
    cx.background_executor()
        .timer(Duration::from_millis(ms))
        .await;
}

fn synth(act: impl FnOnce(Synth<'_>)) {
    // Copied out first: a synthesized call can re-enter the canvas.
    if let Some(native) = canvas::with(|canvas| canvas.native) {
        act(Synth(&native));
    }
}

impl Run {
    fn path(&self, name: &str) -> String {
        self.options
            .out
            .join(format!("{}-{name}.png", self.options.tag))
            .to_string_lossy()
            .into_owned()
    }

    fn capture_command(&self, name: &str) -> Option<Command> {
        let number = canvas::with(|canvas| canvas.native.window_number())?;
        let mut command = Command::new("screencapture");
        command.args(["-x", "-o", "-l", &number.to_string(), &self.path(name)]);
        Some(command)
    }

    /// Captures the window as the window server composites it.
    fn shot(&mut self, name: &str) {
        if let Some(mut command) = self.capture_command(name) {
            match command.status() {
                Ok(status) if status.success() => self.report.push(format!("shot {name}")),
                other => self.report.push(format!("shot {name} failed: {other:?}")),
            }
        }
    }

    /// Starts a capture without waiting for it, for use mid-gesture.
    fn shot_async(&mut self, name: &str) {
        if let Some(mut command) = self.capture_command(name)
            && let Ok(child) = command.spawn()
        {
            self.captures.push(child);
        }
    }

    fn note(&mut self, text: impl Into<String>) {
        self.report.push(text.into());
    }

    fn drain_log(&mut self) {
        self.report.extend(log::take());
    }

    async fn overlays(&mut self, cx: &mut AsyncApp) {
        self.shot("base");
        synth(|s| s.click(MENU_BUTTON.0, MENU_BUTTON.1));
        sleep(cx, 500).await;
        self.shot("dropdown");
        // A click on the canvas while the menu is open: who receives it?
        self.note("-- click on the canvas with the dropdown open");
        synth(|s| s.click(CANVAS_POINT.0, CANVAS_POINT.1));
        sleep(cx, 400).await;
        self.drain_log();
        self.shot("dropdown-dismissed");

        synth(|s| s.click(POPOVER_BUTTON.0, POPOVER_BUTTON.1));
        sleep(cx, 500).await;
        self.shot("popover");
        synth(|s| s.key(true, 53, 0, "\u{1b}", "\u{1b}", false));
        synth(|s| s.key(false, 53, 0, "\u{1b}", "\u{1b}", false));
        sleep(cx, 300).await;

        self.note("-- right click on the canvas");
        synth(|s| s.right_click(CANVAS_POINT.0, CANVAS_POINT.1));
        sleep(cx, 500).await;
        self.drain_log();
        self.shot("context-menu");
        synth(|s| s.key(true, 53, 0, "\u{1b}", "\u{1b}", false));
        synth(|s| s.key(false, 53, 0, "\u{1b}", "\u{1b}", false));
        sleep(cx, 300).await;

        synth(|s| s.click(DIALOG_BUTTON.0, DIALOG_BUTTON.1));
        sleep(cx, 700).await;
        self.shot("dialog");
        self.note("-- click on the canvas with the dialog open");
        synth(|s| s.click(CANVAS_POINT.0, CANVAS_POINT.1));
        sleep(cx, 500).await;
        self.drain_log();
        self.shot("dialog-after-click");
    }

    async fn pacing(&mut self, cx: &mut AsyncApp) {
        sleep(cx, 1500).await;
        canvas::with(|canvas| {
            canvas.pacing.clear();
            canvas.render_time = Duration::ZERO;
            canvas.renders = 0;
            canvas.ticks = 0;
            canvas.skipped = 0;
        });
        let _ = self.shell.update(cx, |shell, _| shell.pacing.clear());
        sleep(cx, (self.options.seconds * 1000.0) as u64).await;
        if let Some(line) = canvas::with(|canvas| {
            format!(
                "{}\ncanvas time in frame(), drawable wait included: {:.2} ms over {} frames; page frames {} ({} zero-copy); display-link ticks {}, frames without a drawable {}",
                canvas.pacing.summary("canvas (wgpu child view)", 120.0),
                canvas.render_time.as_secs_f64() * 1000.0 / f64::from(canvas.renders.max(1)),
                canvas.renders,
                canvas.page_frames,
                canvas.gpu_shared_frames,
                canvas.ticks,
                canvas.skipped,
            )
        }) {
            self.note(line);
        }
        let line = self.shell.update(cx, |shell, _| {
            shell.pacing.summary("gpui (kit shell)", 120.0)
        });
        self.note(line);
        self.shot("pacing");
    }

    /// Drags the kit's resize handle and resizes the window, capturing
    /// throughout, so a frame where the two renderers disagree is on film.
    async fn resize(&mut self, cx: &mut AsyncApp) {
        self.shot("before");
        let (mut x, y) = SPLIT_HANDLE;
        if std::env::var_os("SPIKE_GRAB_GPUI_SIDE").is_some() {
            // The handle's band straddles the divider. In the `above`
            // layering the canvas view owns the half on its side.
            x -= 3.0;
        }
        synth(|s| {
            s.mouse(5, x, y, 0);
            s.mouse(1, x, y, 1);
        });
        sleep(cx, 50).await;
        let steps = 90;
        for step in 0..=steps {
            let phase = f64::from(step) / f64::from(steps);
            // Out 240 points and back, 6 points a frame at most.
            let offset = 240.0 * (phase * std::f64::consts::PI).sin();
            synth(|s| s.mouse(6, x + offset, y, 0));
            if step % 6 == 3 {
                self.shot_async(&format!("drag-{step:02}"));
            }
            sleep(cx, 16).await;
        }
        synth(|s| s.mouse(2, x, y, 1));
        sleep(cx, 300).await;
        self.shot("after-drag");

        for step in 0..=60 {
            let phase = f64::from(step) / 60.0;
            let grow = (phase * std::f64::consts::PI).sin();
            canvas::with(|canvas| {
                canvas
                    .native
                    .set_window_content_size(1200.0 + 260.0 * grow, 760.0 + 140.0 * grow)
            });
            if step % 5 == 2 {
                self.shot_async(&format!("window-{step:02}"));
            }
            sleep(cx, 16).await;
        }
        sleep(cx, 300).await;
        self.shot("after-window");
        if let Some(line) = canvas::with(|canvas| {
            format!(
                "surface reconfigured {} times, {:.2} ms each on average",
                canvas.reconfigures,
                canvas.reconfigure_time.as_secs_f64() * 1000.0
                    / f64::from(canvas.reconfigures.max(1))
            )
        }) {
            self.note(line);
        }
    }

    async fn input(&mut self, cx: &mut AsyncApp) {
        const SHIFT: usize = 1 << 17;
        const OPTION: usize = 1 << 19;
        const COMMAND: usize = 1 << 20;
        let (x, y) = CANVAS_POINT;
        let step = |name: &str| log::line(format!("-- {name}"));

        step("click on the canvas (takes focus)");
        synth(|s| s.click(x, y));
        sleep(cx, 200).await;
        step("drag on the canvas");
        synth(|s| {
            s.mouse(1, x, y, 1);
            s.mouse(6, x + 20.0, y + 10.0, 0);
            s.mouse(6, x + 40.0, y + 20.0, 0);
            s.mouse(2, x + 40.0, y + 20.0, 1);
        });
        sleep(cx, 200).await;
        step("precise scroll, dx=3 dy=-12");
        synth(|s| s.scroll(x, y, 3, -12));
        sleep(cx, 200).await;
        step("magnify +0.2");
        synth(|s| {
            s.magnify(x, y, 0.0, 1);
            s.magnify(x, y, 0.2, 2);
            s.magnify(x, y, 0.0, 4);
        });
        sleep(cx, 200).await;
        step("key a (keyCode 0): down, two repeats, up");
        synth(|s| {
            s.key(true, 0, 0, "a", "a", false);
            s.key(true, 0, 0, "a", "a", true);
            s.key(true, 0, 0, "a", "a", true);
            s.key(false, 0, 0, "a", "a", false);
        });
        sleep(cx, 200).await;
        step("shift+a");
        synth(|s| {
            s.key(true, 0, SHIFT, "A", "a", false);
            s.key(false, 0, SHIFT, "A", "a", false);
        });
        sleep(cx, 200).await;
        step("cmd+z (keyCode 6)");
        synth(|s| {
            s.key(true, 6, COMMAND, "z", "z", false);
            s.key(false, 6, COMMAND, "z", "z", false);
        });
        sleep(cx, 200).await;
        step("left arrow (keyCode 123)");
        synth(|s| {
            s.key(true, 123, 0, "\u{f702}", "\u{f702}", false);
            s.key(false, 123, 0, "\u{f702}", "\u{f702}", false);
        });
        sleep(cx, 200).await;
        step("numpad 1 (keyCode 83) against digit 1 (keyCode 18)");
        synth(|s| {
            s.key(true, 83, 1 << 21, "1", "1", false);
            s.key(false, 83, 1 << 21, "1", "1", false);
            s.key(true, 18, 0, "1", "1", false);
            s.key(false, 18, 0, "1", "1", false);
        });
        sleep(cx, 200).await;
        step("dead key: option+e then e, through the system input context");
        synth(|s| {
            s.key(true, 14, OPTION, "", "e", false);
            s.key(false, 14, OPTION, "", "e", false);
        });
        sleep(cx, 200).await;
        synth(|s| {
            s.key(true, 14, 0, "e", "e", false);
            s.key(false, 14, 0, "e", "e", false);
        });
        sleep(cx, 200).await;
        step("input method: setMarkedText then insertText on GPUI's NSTextInputClient");
        synth(|s| s.set_marked_text("に", 1));
        sleep(cx, 100).await;
        synth(|s| s.set_marked_text("にほ", 2));
        sleep(cx, 100).await;
        synth(|s| s.insert_text("日本"));
        sleep(cx, 200).await;
        self.drain_log();
        self.shot("input");
    }

    /// Real pages: pacing with CEF painting, then keys and an input-method
    /// composition forwarded into a page's text field.
    async fn cef(&mut self, cx: &mut AsyncApp) {
        sleep(cx, 3000).await;
        self.pacing(cx).await;
        let focused = canvas::with(|canvas| canvas.focus_page("page-live")).unwrap_or(false);
        self.note(format!("-- page focused: {focused}"));
        // A click inside the page's text field, in the page's own CSS
        // pixels, as the shell forwards a pointer press on an entered page.
        canvas::with(|canvas| {
            use specular_core::{
                InputEvent, Modifiers, PointerButton, PointerEvent, PointerEventKind,
            };
            let at = glam::Vec2::new(420.0, 100.0);
            for kind in [
                PointerEventKind::Move,
                PointerEventKind::Down {
                    button: PointerButton::Left,
                    click_count: 1,
                },
                PointerEventKind::Up {
                    button: PointerButton::Left,
                    click_count: 1,
                },
            ] {
                canvas.send_to_page(&InputEvent::Pointer(PointerEvent {
                    kind,
                    position: at,
                    modifiers: Modifiers::default(),
                }));
            }
        });
        sleep(cx, 200).await;
        // Focus the canvas slot, so GPUI installs its input handler.
        synth(|s| s.click(CANVAS_POINT.0, CANVAS_POINT.1));
        sleep(cx, 300).await;
        crate::native::FORWARD_KEYS.store(true, std::sync::atomic::Ordering::Relaxed);
        for (code, letter) in [(4u16, "h"), (34, "i"), (49, " ")] {
            synth(|s| {
                s.key(true, code, 0, letter, letter, false);
                s.key(false, code, 0, letter, letter, false);
            });
            sleep(cx, 80).await;
        }
        crate::native::FORWARD_KEYS.store(false, std::sync::atomic::Ordering::Relaxed);
        crate::shell::FORWARD_IME.store(true, std::sync::atomic::Ordering::Relaxed);
        synth(|s| s.set_marked_text("にほ", 2));
        sleep(cx, 400).await;
        self.shot("composing");
        synth(|s| s.insert_text("日本"));
        sleep(cx, 500).await;
        crate::shell::FORWARD_IME.store(false, std::sync::atomic::Ordering::Relaxed);
        self.drain_log();
        self.shot("typed");
    }

    /// GPUI Kit controls built from the app's own models, and what the
    /// canvas draws in step with them.
    async fn model(&mut self, cx: &mut AsyncApp) {
        canvas::with(|canvas| canvas.animate = false);
        self.shot("start");
        let row = self.shell.update(cx, |shell, cx| shell.press_first_row(cx));
        self.note(format!(
            "pressed sidebar row {row:?}: its Action selects the entity"
        ));
        sleep(cx, 400).await;
        self.shot("row-selected");
        let tool = self.shell.update(cx, |shell, cx| shell.press_tool(2, cx));
        self.note(format!("pressed tool {tool:?}: its Action arms the tool"));
        sleep(cx, 400).await;
        self.shot("tool-armed");
    }

    /// How steadily GPUI's foreground executor can run a 4 ms pump.
    async fn timer(&mut self, cx: &mut AsyncApp) {
        let mut pacing = Pacing::default();
        let until = Instant::now() + Duration::from_secs_f64(self.options.seconds);
        while Instant::now() < until {
            cx.background_executor()
                .timer(Duration::from_millis(4))
                .await;
            pacing.mark();
        }
        self.note(pacing.summary("gpui foreground timer, 4 ms asked", 250.0));
    }
}

pub async fn run(
    options: Options,
    window: AnyWindowHandle,
    shell: Entity<Shell>,
    cx: &mut AsyncApp,
) {
    let _ = std::fs::create_dir_all(&options.out);
    let mut run = Run {
        options,
        window,
        shell,
        report: Vec::new(),
        captures: Vec::new(),
    };
    run.note(format!("{:?}", run.options));
    run.note(format!(
        "window number {:?}",
        canvas::with(|canvas| canvas.native.window_number())
    ));
    if run.options.pump_from_gpui {
        // CEF's external message pump on GPUI's main-thread executor.
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(4))
                    .await;
                if canvas::with(|canvas| canvas.source.pump()).is_none() {
                    break;
                }
            }
        })
        .detach();
    }
    sleep(cx, 1200).await;
    run.drain_log();
    match run.options.scenario.clone().as_str() {
        "overlays" => run.overlays(cx).await,
        "pacing" => run.pacing(cx).await,
        "resize" => run.resize(cx).await,
        "input" => run.input(cx).await,
        "timer" => run.timer(cx).await,
        "cef" => run.cef(cx).await,
        "model" => run.model(cx).await,
        other => run.note(format!("unknown scenario {other}")),
    }
    for mut child in std::mem::take(&mut run.captures) {
        let _ = child.wait();
    }
    run.drain_log();
    let report = run.options.out.join(format!("{}.txt", run.options.tag));
    let _ = std::fs::write(&report, run.report.join("\n") + "\n");
    println!("{}", run.report.join("\n"));

    // CEF on macOS can only stop from inside the running loop.
    for _ in 0..400 {
        let stopped = canvas::with(|canvas| canvas.source.poll_shutdown()).unwrap_or(true);
        if stopped {
            break;
        }
        sleep(cx, 10).await;
    }
    let _ = run.window.update(cx, |_, window, _| window.remove_window());
    if let Some(canvas) = canvas::uninstall() {
        canvas.close();
    }
    let _ = cx.update(|cx| cx.quit());
}
