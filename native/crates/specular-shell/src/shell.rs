//! Opening the window and tying the canvas to GPUI: the window with the
//! canvas view under GPUI's, the runtime, and the tasks that carry word
//! between the canvas's frame and GPUI's.

use std::rc::Rc;
use std::time::Duration;

use anyhow::Context as _;
use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::base::Root;
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, Global, Styled as _,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowOptions, point, px,
    size, transparent_black,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use specular_app::{Bench, Launch, Runtime};
use specular_interact::Event;

use crate::canvas;
use crate::native::{Id, NativeCanvas};
use crate::pacing::Pacing;
use crate::surface::{CanvasSurface, WindowAsks};
use crate::view::ShellView;
use crate::{debug_input, keys, menus, pins, refresh, spaces, theme};

/// The window size when the command line names none: the Electron app's.
const DEFAULT_SIZE: (u32, u32) = (1600, 1000);
/// How long the canvas may go without a frame before GPUI gives it a turn:
/// a display link stops while its window is covered, and autosave must not.
/// A canvas at rest pauses its own link and asks for its turns sooner.
const IDLE_TURN: Duration = Duration::from_millis(250);

/// The window and its root view, for an action that needs them.
#[derive(Clone)]
struct ShellHandle {
    window: AnyWindowHandle,
    view: Entity<ShellView>,
}

impl Global for ShellHandle {}

/// Runs `with` on the root view in its window, once the current update is
/// over: an action handler runs while its window is already borrowed.
pub(crate) fn with_view(
    cx: &mut App,
    with: impl FnOnce(&mut ShellView, &mut Window, &mut Context<'_, ShellView>) + 'static,
) {
    cx.defer(move |cx| {
        let Some(handle) = cx.try_global::<ShellHandle>().cloned() else {
            return;
        };
        let view = handle.view;
        let updated = handle.window.update(cx, |_, window, cx| {
            view.update(cx, |view, cx| with(view, window, cx));
        });
        if let Err(error) = updated {
            tracing::warn!("the window is gone: {error}");
        }
    });
}

/// Starts shutting down: what is unsaved is written and the pages are
/// closed. The app quits once the page backend says it is done.
pub(crate) fn begin_exit(cx: &mut App) {
    if canvas::with(|canvas| canvas.runtime.exit()).is_none() {
        cx.quit();
    }
}

/// Ends the run once the page backend has shut down.
fn finish(cx: &mut App) {
    let Some(mut canvas) = canvas::uninstall() else {
        return;
    };
    if let Some(surface) = canvas.runtime.window_mut() {
        surface.close();
    }
    if let Err(error) = canvas.runtime.into_result() {
        eprintln!("specular: {error:#}");
        std::process::exit(1);
    }
    cx.quit();
}

/// The canvas changed something GPUI shows, or the run is over.
fn on_wake(cx: &mut App) {
    if canvas::with(|canvas| canvas.is_finished()) == Some(true) {
        finish(cx);
        return;
    }
    if let Some(create) = canvas::with(canvas::Canvas::take_space_dialog).flatten() {
        spaces::choose(create, cx);
    }
    if let Some(models) = canvas::models() {
        menus::sync(&models.menus, cx);
        if models.appearance != theme::appearance() {
            theme::apply(models.appearance, cx);
            cx.refresh_windows();
        }
    }
    if let Some(handle) = cx.try_global::<ShellHandle>().cloned() {
        handle.view.update(cx, |_, cx| cx.notify());
    }
}

/// GPUI's own view: the `NSView` its window handle carries.
fn gpui_view(window: &Window) -> anyhow::Result<Id> {
    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("no native window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr().cast()),
        other => anyhow::bail!("not an AppKit window: {other:?}"),
    }
}

fn window_options(launch: &Launch) -> WindowOptions {
    let (width, height) = launch.window_size().unwrap_or(DEFAULT_SIZE);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(80.0), px(60.0)),
            size(px(width as f32), px(height as f32)),
        ))),
        // GPUI's Metal layer is opaque unless the window is transparent, and
        // an opaque layer above the canvas view would hide it.
        window_background: WindowBackgroundAppearance::Transparent,
        // The tab row is the title bar, with the traffic lights in its
        // left padding.
        titlebar: Some(TitlebarOptions {
            title: Some("Specular".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.0), px(13.0))),
        }),
        window_min_size: Some(size(px(640.0), px(400.0))),
        ..WindowOptions::default()
    }
}

/// Tells the app which of its chrome this shell draws itself.
fn say_what_the_kit_draws(runtime: &mut Runtime<CanvasSurface>, benching: bool) {
    // The Kit draws the chrome and the sidebar. The menu a right press on
    // the canvas opens stays in the canvas's own pass, except in a
    // benchmark, which draws the canvas alone as the winit shell's does.
    runtime.dispatch(if benching {
        Event::BuiltinPanels(false)
    } else {
        Event::BuiltinMenu
    });
    if !benching {
        // The Kit draws a right panel, so a comment is written there and
        // the canvas keeps only its marker.
        runtime.dispatch(Event::ChatPanel(true));
    }
    runtime.dispatch(Event::About(pins::about(runtime.source_name())));
}

/// Keeps the app told of the operating system's appearance while it runs,
/// which is what the `System` theme draws.
fn follow_system_appearance(window: AnyWindowHandle, cx: &mut App) -> anyhow::Result<()> {
    window.update(cx, |_, window, _| {
        window
            .observe_window_appearance(|window, _| {
                let system = theme::of_system(window.appearance());
                canvas::dispatch(Event::SystemAppearance(system));
            })
            .detach();
    })?;
    Ok(())
}

/// Opens the window on `launch` and starts everything that runs with it.
pub(crate) fn open(launch: Launch, cx: &mut App) -> anyhow::Result<()> {
    // GPUI has made `NSApp` by now, which CEF needs before it starts.
    let source = launch
        .create_source()
        .context("creating the page backend")?;
    let options = launch.runtime_options();
    let bench = launch.bench();
    let asks = Rc::new(WindowAsks::default());

    // Not `gpui_kit::open_window`: the Kit's root paints the theme's
    // background over the whole window, which would cover the canvas, and
    // only a root built here can have that fill cleared.
    let mut root_view = None;
    let view_asks = Rc::clone(&asks);
    let window: AnyWindowHandle = cx
        .open_window(window_options(&launch), |window, cx| {
            let view = cx.new(|cx| ShellView::new(view_asks, window, cx));
            root_view = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx).bg(transparent_black()))
        })
        .context("opening the window")?
        .into();
    let view = root_view.context("the window built no view")?;

    let (wake, mut woken) = mpsc::channel::<()>(1);
    let (api_wake, mut api_woken) = mpsc::channel::<()>(1);
    let (again, mut run_again) = mpsc::channel::<()>(1);
    let opening = launch.into_opening();
    window.update(cx, |_, window, _| -> anyhow::Result<()> {
        let mut native = NativeCanvas::install(gpui_view(window)?)?;
        // A covered window is given no frames, which a benchmark would
        // record as a profile that drew nothing.
        if bench.is_some() || std::env::var_os("SPECULAR_FLOAT_WINDOW").is_some() {
            native.float();
        }
        native.start_display_link();
        let refresh = refresh::interval(native.window_number());
        let mut surface = CanvasSurface::new(native, Rc::clone(&asks))?;
        surface.pacing = Pacing::from_env();
        let mut runtime = Runtime::new(source, options);
        tracing::info!(
            backend = runtime.source_name(),
            "starting the GPUI Kit shell"
        );
        runtime.attach_window(surface);
        say_what_the_kit_draws(&mut runtime, bench.is_some());
        runtime.dispatch(Event::SystemAppearance(theme::of_system(
            window.appearance(),
        )));
        runtime.open(opening)?;
        // Finder's file, when the app was launched to open one.
        if let Some(file) = spaces::take_waiting() {
            runtime.open_canvas_file(&file)?;
        }
        let bench = bench.map(|options| Bench::start(&mut runtime, options, "kit", refresh));
        if bench.is_none() {
            // A benchmark takes no outside input, so it has no API.
            runtime.start_api(move || {
                // Each clone has a slot of its own, so a wake is never lost.
                let _ = api_wake.clone().try_send(());
            });
        }
        canvas::install(runtime, asks, (wake, again), refresh, bench);
        Ok(())
    })??;

    follow_system_appearance(window, cx)?;
    cx.set_global(ShellHandle { window, view });
    keys::install_monitor();
    menus::install(cx);
    on_wake(cx);
    window.update(cx, |_, window, cx| {
        // Closing the window is quitting, and both wait for the pages.
        window.on_window_should_close(cx, |_, cx| {
            begin_exit(cx);
            false
        });
    })?;

    cx.spawn(async move |cx| {
        while woken.next().await.is_some() {
            cx.update(on_wake);
        }
    })
    .detach();
    cx.spawn(async move |_| {
        while run_again.next().await.is_some() {
            canvas::with(canvas::Canvas::catch_up);
        }
    })
    .detach();
    cx.spawn(async move |_| {
        while api_woken.next().await.is_some() {
            canvas::with(|canvas| {
                canvas.runtime.serve_api();
                canvas.refresh_models();
            });
        }
    })
    .detach();
    cx.spawn(async move |cx| {
        loop {
            let wait = canvas::with(|canvas| canvas.timer_wait(IDLE_TURN)).unwrap_or(IDLE_TURN);
            cx.background_executor().timer(wait).await;
            canvas::with(|canvas| {
                if canvas.last_frame().elapsed() >= wait {
                    canvas.frame();
                }
            });
        }
    })
    .detach();
    debug_input::run_from_env(window, cx);
    cx.activate(true);
    Ok(())
}
