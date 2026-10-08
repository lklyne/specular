//! GPUI Kit owns the window and draws the UI around the canvas. Specular's
//! own wgpu compositor draws the canvas into a child NSView of that window.
//! Each scenario answers one question of ADR 0040 and writes its evidence to
//! `shots/`.
//!
//! ```sh
//! cargo run -- --layer below --scenario overlays
//! cargo run -- --layer above --drive gpui --scenario pacing
//! ```

mod canvas;
mod log;
mod native;
mod script;
mod shell;

use std::path::PathBuf;

use gpui_kit::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use specular_core::PageSource;

use crate::canvas::{Canvas, Drive};
use crate::native::{NativeCanvas, Order};
use crate::shell::{Layout, Shell};

#[derive(Debug, Clone)]
pub struct Options {
    pub layout: Layout,
    /// In the `above` layering, let pointer events fall through to GPUI.
    pub pass_through: bool,
    pub gpui_animates: bool,
    pub scenario: String,
    pub out: PathBuf,
    pub tag: String,
    pub canvas: PathBuf,
    pub cef: bool,
    /// Pump CEF from GPUI's foreground executor instead of a run-loop timer.
    pub pump_from_gpui: bool,
    pub seconds: f64,
}

fn options() -> Options {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut options = Options {
        layout: Layout {
            order: Order::Below,
            drive: Drive::Link,
        },
        pass_through: false,
        gpui_animates: false,
        scenario: "overlays".to_owned(),
        out: root.join("out"),
        tag: String::new(),
        canvas: root.join("../../fixtures/kitchen-sink.canvas"),
        cef: false,
        pump_from_gpui: false,
        seconds: 8.0,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_default();
        match arg.as_str() {
            "--layer" => match value().as_str() {
                "above" => options.layout.order = Order::Above,
                "above-through" => {
                    options.layout.order = Order::Above;
                    options.pass_through = true;
                }
                _ => options.layout.order = Order::Below,
            },
            "--drive" => {
                options.layout.drive = if value() == "gpui" {
                    Drive::Gpui
                } else {
                    Drive::Link
                }
            }
            "--gpui-anim" => options.gpui_animates = value() == "on",
            "--scenario" => options.scenario = value(),
            "--out" => options.out = PathBuf::from(value()),
            "--tag" => options.tag = value(),
            "--canvas" => options.canvas = PathBuf::from(value()),
            "--source" => options.cef = value() == "cef",
            "--pump" => options.pump_from_gpui = value() == "gpui",
            "--seconds" => options.seconds = value().parse().unwrap_or(8.0),
            // CEF adds its own switches to a relaunched helper; ignore them.
            _ => {}
        }
    }
    if options.tag.is_empty() {
        let layer = match (options.layout.order, options.pass_through) {
            (Order::Below, _) => "below",
            (Order::Above, false) => "above",
            (Order::Above, true) => "above-through",
        };
        options.tag = format!("{}-{layer}", options.scenario);
    }
    options
}

fn page_source(options: &Options) -> anyhow::Result<Box<dyn PageSource>> {
    if !options.cef {
        return Ok(Box::new(specular_core::SyntheticPageSource::new()));
    }
    #[cfg(feature = "cef")]
    {
        let config = specular_cef::CefConfig {
            remote_debugging_port: None,
            cache_path: None,
            shared_texture: true,
            pump: if options.pump_from_gpui {
                specular_cef::Pump::Caller
            } else {
                specular_cef::Pump::RunLoopTimer
            },
        };
        Ok(Box::new(specular_cef::CefPageSource::new(config)?))
    }
    #[cfg(not(feature = "cef"))]
    anyhow::bail!("built without `--features cef`")
}

fn gpui_view(window: &Window) -> anyhow::Result<native::Id> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("no native window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr().cast()),
        other => anyhow::bail!("not an AppKit window: {other:?}"),
    }
}

fn open(options: &Options, cx: &mut App) -> anyhow::Result<(AnyWindowHandle, Entity<Shell>)> {
    let below = options.layout.order == Order::Below;
    let window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(120.), px(80.)),
            size(px(1200.), px(760.)),
        ))),
        // GPUI's Metal layer is opaque unless the window is transparent, and
        // an opaque layer above the canvas view would hide it.
        window_background: if below {
            WindowBackgroundAppearance::Transparent
        } else {
            WindowBackgroundAppearance::Opaque
        },
        titlebar: Some(TitlebarOptions {
            title: Some(format!("GPUI Kit hybrid: {}", options.tag).into()),
            ..TitlebarOptions::default()
        }),
        ..WindowOptions::default()
    };
    let layout = options.layout;
    let animate = options.gpui_animates;
    // `gpui_kit::open_window`, with one change: the kit's Root paints the
    // theme background over the whole window, which would cover a canvas
    // view below it, so that fill is cleared for the `below` layering.
    let mut shell = None;
    let window: AnyWindowHandle = cx
        .open_window(window_options, |window, cx| {
            let view = cx.new(|cx| Shell::new(layout, animate, cx));
            shell = Some(view.clone());
            cx.new(|cx| {
                let root = base::Root::new(view, window, cx);
                if below {
                    root.bg(transparent_black())
                } else {
                    root
                }
            })
        })?
        .into();
    let shell = shell.ok_or_else(|| anyhow::anyhow!("the window built no shell"))?;
    let json = std::fs::read_to_string(&options.canvas)?;
    let source = page_source(options)?;
    native::PASS_THROUGH.store(options.pass_through, std::sync::atomic::Ordering::Relaxed);
    window.update(cx, |_, window, _| -> anyhow::Result<()> {
        let mut native = NativeCanvas::new(gpui_view(window)?, layout.order);
        if layout.drive == Drive::Link {
            native.start_display_link();
        }
        let mut canvas = Canvas::new(native, source, &json, layout.drive)?;
        canvas.pump_in_frame = !options.pump_from_gpui;
        canvas::install(canvas);
        window.refresh();
        Ok(())
    })??;
    native::install_event_monitor();
    Ok((window, shell))
}

fn main() {
    #[cfg(feature = "cef")]
    if let Some(code) = specular_cef::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    let options = options();
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            match open(&options, cx) {
                Ok((window, shell)) => {
                    cx.activate(true);
                    cx.spawn(async move |cx| script::run(options, window, shell, cx).await)
                        .detach();
                }
                Err(error) => {
                    eprintln!("could not start: {error:#}");
                    cx.quit();
                }
            }
        });
}
