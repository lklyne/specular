//! `rune-shell [FILE.canvas] [--camera x,y,zoom] [--still] [--chrome rust|luau]
//! [--panel DIR] [--source synthetic|cef] [--pump timer|caller]
//! [--probe-keys SECONDS] [Rune's flags]`
//!
//! A window with no flags of Rune's; headless with `--snap`, `--tree` or
//! `--script` (see the README beside this crate).

use std::path::PathBuf;

use glam::Vec2;
use rune::GpuBackend;
use rune_shell_spike::canvas::Canvas;
use rune_shell_spike::pages::{PumpKind, SourceKind};
use rune_shell_spike::{Chrome, Options, app, toolbar_panel};
use specular_core::Camera;

fn camera(value: &str) -> Option<Camera> {
    let mut parts = value.split(',').map(str::parse::<f32>);
    let (x, y, zoom) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    Some(Camera {
        pan: Vec2::new(x, y),
        zoom,
    })
}

fn size(value: &str) -> Option<(f32, f32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn main() {
    // Chromium's children are copies of this binary in the bundle's helpers.
    #[cfg(feature = "cef")]
    if let Some(code) = rune_shell_spike::cef::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    #[cfg(feature = "cef")]
    rune_shell_spike::cef::make_main_bundle();
    let mut options = Options::default();
    // Rune's headless runs draw at 2 device pixels a point unless told.
    let mut scale = None;
    let mut headless = false;
    // Rune's own Metal backend panics on multisampling, which our 4x pass
    // needs, so wgpu's unless told.
    let mut backend = GpuBackend::Wgpu;
    // Ours are taken out; the rest is handed to Rune as it came.
    let mut runes = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--camera" => {
                let value = args.next().unwrap_or_default();
                let Some(at) = camera(&value) else {
                    eprintln!("rune-shell: --camera expects x,y,zoom, got `{value}`");
                    std::process::exit(2);
                };
                options.camera = Some(at);
            }
            "--still" => options.still = true,
            "--source" => {
                options.source = match args.next().unwrap_or_default().as_str() {
                    "synthetic" => SourceKind::Synthetic,
                    "cef" => SourceKind::Cef,
                    other => {
                        eprintln!("rune-shell: --source expects synthetic or cef, got `{other}`");
                        std::process::exit(2);
                    }
                };
            }
            "--pump" => {
                options.pump = match args.next().unwrap_or_default().as_str() {
                    "timer" => PumpKind::Timer,
                    "caller" => PumpKind::Caller,
                    other => {
                        eprintln!("rune-shell: --pump expects timer or caller, got `{other}`");
                        std::process::exit(2);
                    }
                };
            }
            "--probe-keys" => {
                let value = args.next().unwrap_or_default();
                let Ok(at) = value.parse() else {
                    eprintln!("rune-shell: --probe-keys expects seconds, got `{value}`");
                    std::process::exit(2);
                };
                options.probe_keys_at = Some(at);
            }
            "--backend" => {
                backend = match args.next().unwrap_or_default().as_str() {
                    "wgpu" => GpuBackend::Wgpu,
                    "metal" => GpuBackend::Metal,
                    other => {
                        eprintln!("rune-shell: --backend expects wgpu or metal, got `{other}`");
                        std::process::exit(2);
                    }
                };
            }
            "--samples" => {
                let value = args.next().unwrap_or_default();
                options.samples = match value.parse() {
                    Ok(samples @ (1 | 4)) => samples,
                    _ => {
                        eprintln!("rune-shell: --samples expects 1 or 4, got `{value}`");
                        std::process::exit(2);
                    }
                };
            }
            "--chrome" => {
                let value = args.next().unwrap_or_default();
                options.chrome = match value.as_str() {
                    "rust" => Chrome::Rust,
                    "luau" => Chrome::Luau(toolbar_panel()),
                    _ => {
                        eprintln!("rune-shell: --chrome expects rust or luau, got `{value}`");
                        std::process::exit(2);
                    }
                };
            }
            // The toolbar script's folder, when it is not the shipped one.
            "--panel" => {
                options.chrome = Chrome::Luau(PathBuf::from(args.next().unwrap_or_default()));
            }
            "--size" => {
                let value = args.next().unwrap_or_default();
                if let Some(points) = size(&value) {
                    options.size = points;
                }
                runes.extend(["--size".to_owned(), value]);
            }
            "--scale" => {
                let value = args.next().unwrap_or_default();
                scale = value.parse().ok();
                runes.extend(["--scale".to_owned(), value]);
            }
            "--snap" | "--tree" | "--script" => {
                headless = true;
                runes.push(arg);
            }
            file if file.ends_with(".canvas") => options.canvas = Some(PathBuf::from(file)),
            _ => runes.push(arg),
        }
    }
    options.scale = scale.unwrap_or(if headless { 2.0 } else { 1.0 });
    options.headless = headless;
    rune::prewarm_gpu_on(backend);
    let samples = options.samples;
    app(options)
        .gpu_backend(backend)
        .args(runes)
        .compositor(Canvas::new(samples))
        .run();
}
