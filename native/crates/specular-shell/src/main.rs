//! `specular`: the GPUI Kit shell (ADR 0040).
//!
//! One GPUI window. GPUI Kit draws the toolbar, the sidebar, the menus and
//! the dialogs from the app's pure models, and the unchanged compositor
//! draws the canvas and its pages into a view under GPUI's. The runtime,
//! the command line and the `--snapshot` and `--script` modes are the
//! winit shell's own, from `specular-app`.
//!
//! ```sh
//! cargo run -p specular-shell -- fixtures/kitchen-sink.canvas
//! ```

#[cfg(target_os = "macos")]
mod assets;
#[cfg(target_os = "macos")]
mod canvas;
#[cfg(target_os = "macos")]
mod debug_input;
#[cfg(target_os = "macos")]
mod keys;
#[cfg(target_os = "macos")]
mod menus;
#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
mod pacing;
#[cfg(target_os = "macos")]
mod pins;
#[cfg(target_os = "macos")]
mod settings;
#[cfg(target_os = "macos")]
mod settings_repos;
#[cfg(target_os = "macos")]
mod shell;
#[cfg(target_os = "macos")]
mod spaces;
#[cfg(target_os = "macos")]
mod surface;
#[cfg(target_os = "macos")]
mod theme;
#[cfg(target_os = "macos")]
mod view;

#[cfg(target_os = "macos")]
fn main() {
    if let Some(code) = specular_app::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    let launch =
        match specular_app::launch(std::env::args_os().skip(1), specular_app::Unnamed::Chosen) {
            Ok(Some(launch)) => launch,
            // Help was printed, or a headless run is finished.
            Ok(None) => return,
            Err(error) => {
                eprintln!("specular: {error:#}");
                std::process::exit(1);
            }
        };
    if launch.is_bench() {
        eprintln!("specular: --bench runs in the winit shell: cargo run -p specular-app");
        std::process::exit(2);
    }
    let application = gpui_kit::application().with_assets(assets::ShellAssets);
    application.on_open_urls(|urls| spaces::opened_from_finder(&urls));
    application.run(move |cx| {
        gpui_kit::init(cx);
        theme::apply(cx);
        if let Err(error) = shell::open(launch, cx) {
            // Loud on purpose: a pin that moved lands here.
            eprintln!("specular could not start: {error:#}");
            std::process::exit(1);
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("specular: the GPUI Kit shell is macOS only for now; run specular-app");
    std::process::exit(2);
}
