//! The `mod` verbs (ADR 0055): `mod check <dir> [--state fixture.json]`
//! loads a mod with no app and prints its element tree, `mod stats` prints
//! what each mod in the running window has cost, and `mod state [key]` is
//! `state`. `mod clone` is in `mod_clone`.
//!
//! The check needs a GPUI runtime, which this command keeps out of itself:
//! it runs `specular-mod-check`, found as `SPECULAR_MOD_CHECK` names it,
//! else beside this executable (the app bundle has it there), else on the
//! `PATH`. With no `--state` it renders against the running app's state,
//! or a fresh app's when no app answers, and says which.

use std::path::PathBuf;
use std::process::Command;

use crate::args::Args;
use crate::client::Client;
use crate::{output, refuse, registry, usage};

const USAGE: &str = "specular mod check <dir> [--state fixture.json] | mod clone <name> [new-name] [--use] | mod stats | mod state [key]";

/// The name of the checker's executable.
const CHECKER: &str = "specular-mod-check";

/// Whether `args` is `mod check`, which needs no running app.
pub(crate) fn is_check(args: &Args) -> bool {
    args.verb == "mod" && args.nth(0) == Some("check")
}

/// Where `specular-mod-check` is.
fn checker() -> PathBuf {
    if let Some(path) = std::env::var_os("SPECULAR_MOD_CHECK").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    let beside = (std::env::current_exe().ok())
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| Some(exe.parent()?.join(CHECKER)))
        .filter(|path| path.is_file());
    beside.unwrap_or_else(|| PathBuf::from(CHECKER))
}

/// `mod check <dir> [--state fixture.json]`: the checker's output and exit
/// code, as its own.
pub(crate) fn check(args: &Args) -> anyhow::Result<i32> {
    let Some(dir) = args.nth(1) else {
        return usage(USAGE);
    };
    let mut command = Command::new(checker());
    command.arg(dir);
    let mut scratch = None;
    match args.flag("state") {
        Some(state) => {
            command.args(["--state", state]);
        }
        None => match running_state() {
            Some(path) => {
                eprintln!("checking against the running app's state");
                command.arg("--state").arg(&path);
                scratch = Some(path);
            }
            None => eprintln!("no app answered: checking against a fresh app's state"),
        },
    }
    let status = command.status();
    if let Some(path) = scratch {
        let _ = std::fs::remove_file(path);
    }
    match status {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(error) => refuse(&format!(
            "could not run {CHECKER} ({error}); build it with `cargo build -p specular-mod-check`, or name it with SPECULAR_MOD_CHECK"
        )),
    }
}

/// The running app's state written to a scratch file, or `None` when no
/// app answers.
fn running_state() -> Option<PathBuf> {
    let state = Client::new(None).get("/state").ok()?;
    let path = std::env::temp_dir().join(format!("specular-mod-check-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&state).ok()?).ok()?;
    Some(path)
}

/// `mod stats` and `mod state [key]`.
pub(crate) fn verb(client: &Client, args: &Args) -> anyhow::Result<i32> {
    match args.nth(0) {
        Some("stats") => {
            output::json(&client.get("/mods")?);
            Ok(0)
        }
        Some("state") => registry::state(client, &args.shifted()),
        _ => usage(USAGE),
    }
}
