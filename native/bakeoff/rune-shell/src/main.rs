//! `rune-shell [FILE.canvas] [--camera x,y,zoom] [--still] [Rune's flags]`
//!
//! A window with no flags of Rune's; headless with `--snap`, `--tree` or
//! `--script` (see the README beside this crate).

use std::path::PathBuf;

use glam::Vec2;
use rune_shell_spike::canvas::Canvas;
use rune_shell_spike::{Options, app};
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
    let mut options = Options::default();
    // Rune's headless runs draw at 2 device pixels a point unless told.
    let mut scale = None;
    let mut headless = false;
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
    app(options).args(runes).compositor(Canvas::new()).run();
}
