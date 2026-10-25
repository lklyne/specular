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
use specular_interact::Event;
use specular_shell::{Bench, Launch, Runtime};

use crate::canvas;
use crate::native::{Id, NativeCanvas};
use crate::pacing::Pacing;
use crate::surface::{CanvasSurface, WindowAsks};
use crate::view::ShellView;
use crate::{debug_input, keys, menus, motion, pins, refresh, spaces, theme};

/// The window size when the command line names none: the Electron app's.
pub(crate) const DEFAULT_SIZE: (u32, u32) = (1600, 1000);
/// The window while it shows the first-run view, which grows to
/// [`DEFAULT_SIZE`] once a space opens.
pub(crate) const WELCOME_SIZE: (u32, u32) = (720, 540);
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
    specular_core::ledger::flush();
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
    if let Some(ask) = canvas::with(canvas::Canvas::take_folder_dialog).flatten() {
        spaces::choose_folder(ask, cx);
    }
    if let Some(models) = canvas::models() {
        menus::sync(&models.menus, cx);
        if let Some(system) = theme::choose(models.theme) {
            canvas::dispatch(Event::SystemAppearance(system));
        }
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

/// The native canvas view under GPUI's, with the window placed.
fn install_canvas(
    window: &mut Window,
    cx: &mut App,
    remembered: bool,
    bench: bool,
) -> anyhow::Result<NativeCanvas> {
    let native = NativeCanvas::install(gpui_view(window)?)?;
    if remembered {
        native.place_main_window(f64::from(DEFAULT_SIZE.0), f64::from(DEFAULT_SIZE.1));
        // GPUI's own resize callback cannot reach the window while the app
        // is borrowed, as it is here, so the window learns its new size now.
        window.bounds_changed(cx);
    }
    // A covered window is given no frames, which a benchmark would record
    // as a profile that drew nothing.
    if bench || std::env::var_os("SPECULAR_FLOAT_WINDOW").is_some() {
        native.float();
    }
    Ok(native)
}

/// Whether this run's window may take and save the place the user last
/// left the canvas window in. One opened at a size asked for, for a
/// benchmark or for a script may not.
#[derive(Clone, Copy)]
pub(crate) struct PlacesWindow(pub(crate) bool);

impl Global for PlacesWindow {}

fn places_window(launch: &Launch) -> bool {
    launch.window_size().is_none() && launch.bench().is_none() && !debug_input::scripted()
}

/// Whether the window opens where the user last left it. One that opens on
/// the first-run view takes that place once a space opens instead.
fn remembers_place(launch: &Launch) -> bool {
    places_window(launch) && !launch.asks_for_space()
}

fn window_options(launch: &Launch, cx: &App) -> WindowOptions {
    let asked = launch.window_size();
    let welcome = asked.is_none() && launch.asks_for_space();
    let (width, height) = asked.unwrap_or(if welcome { WELCOME_SIZE } else { DEFAULT_SIZE });
    let content = size(px(width as f32), px(height as f32));
    let bounds = if welcome {
        Bounds::centered(None, content, cx)
    } else {
        Bounds::new(point(px(80.0), px(60.0)), content)
    };
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
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
    // The Kit draws the chrome, the sidebar and the context menu, except in
    // a benchmark, which draws the canvas alone.
    if !benching {
        runtime.dispatch(Event::ShellChrome);
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
                // A window told to be light or dark says that, not what
                // the system is.
                if theme::follows_system() {
                    let system = theme::of_system(window.appearance());
                    canvas::dispatch(Event::SystemAppearance(system));
                }
            })
            .detach();
    })?;
    Ok(())
}

/// The loops that turn the canvas: on a wake from a page or the agent, on
/// a catch-up asked by a late frame, on an API request, and on the idle
/// timer that covers for a window the system gives no refreshes.
fn spawn_loops(
    mut woken: mpsc::Receiver<()>,
    mut run_again: mpsc::Receiver<()>,
    mut api_woken: mpsc::Receiver<()>,
    cx: &mut App,
) {
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
    cx.set_global(PlacesWindow(places_window(&launch)));

    // Not `gpui_kit::open_window`: the Kit's root paints the theme's
    // background over the whole window, which would cover the canvas, and
    // only a root built here can have that fill cleared.
    let mut root_view = None;
    let view_asks = Rc::clone(&asks);
    let window: AnyWindowHandle = cx
        .open_window(window_options(&launch, cx), |window, cx| {
            let view = cx.new(|cx| ShellView::new(view_asks, window, cx));
            root_view = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx).bg(transparent_black()))
        })
        .context("opening the window")?
        .into();
    let view = root_view.context("the window built no view")?;

    let (wake, woken) = mpsc::channel::<()>(1);
    let (api_wake, api_woken) = mpsc::channel::<()>(1);
    let (again, run_again) = mpsc::channel::<()>(1);
    let remembered = remembers_place(&launch);
    let opening = launch.into_opening();
    window.update(cx, |_, window, cx| -> anyhow::Result<()> {
        let mut native = install_canvas(window, cx, remembered, bench.is_some())?;
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
    motion::follow_system(cx);
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

    spawn_loops(woken, run_again, api_woken, cx);
    debug_input::run_from_env(window, cx);
    cx.activate(true);
    Ok(())
}
