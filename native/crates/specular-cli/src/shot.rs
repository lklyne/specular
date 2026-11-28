//! `shot [file.png]`: the canvas as the window shows it, saved as a PNG,
//! so an agent can look at what a change drew. The app writes the file, so
//! the path is made absolute here, where the agent's folder is known.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::args::Args;
use crate::client::Client;
use crate::output;

/// The file `shot` writes when none is named, in the current folder.
const DEFAULT_FILE: &str = "specular-shot.png";

/// `shot [file.png]`: prints the PNG's absolute path and size.
pub(crate) fn shot(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let path = absolute(Path::new(args.nth(0).unwrap_or(DEFAULT_FILE)))?;
    output::json(&client.post("/window/screenshot", &json!({ "path": path }))?);
    Ok(0)
}

fn absolute(path: &Path) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()?.join(path))
}
