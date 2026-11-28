//! `specular`: the command an agent drives the running app with.
//!
//! Each verb is one or two requests to the app's HTTP API (`specular-api`),
//! found through `~/.specular/specular-mcp.json` or
//! `SPECULAR_DISCOVERY_FILE`. Answers print as JSON, browse output as text,
//! pictures as temp files. A usage error or a refused request exits 1.
//! Unknown verbs pass through to agent-browser on the page `-f` names.

mod annotations;
mod args;
mod browse;
mod canvas;
mod client;
mod command;
mod edit;
mod js;
mod mod_clone;
mod mods;
mod output;
mod patch;
mod registry;
mod session;
mod shot;
mod spawn;

use std::process::ExitCode;

use serde_json::json;

use crate::args::Args;
use crate::client::Client;

/// The manual for building mods, printed by `specular --skill`.
const SKILL: &str = include_str!("SKILL.md");

const HELP: &str = "\
usage: specular <verb> [args...] [--flag value]
       specular --skill   Agent Skill: building mods for this version of the app

Canvas: canvas, add, update, delete, arrange, focus, group, ungroup
Tabs: tab, tab switch <tab-id|tab-name>, tab delete <tab-id|tab-name>,
      tab new <name> [--project <id|name>]
  Verbs write to the canvas the user has open. Agents: add there, and run
  `tab new` only when the user asks for a new canvas.
  --tab <tab-id|tab-name|project/name> targets another canvas without switching focus
Projects: project, project open <folder>, project close <id|name>
  the folders and repos in the sidebar, each with its canvases
Share: share, share <link>, share join <link>, share stop
Browse: snapshot, click, fill, type, select, screenshot, scroll, wait
Annotations: annotations, annotation, annotate, annotate-selection, ack, resolve, dismiss, reply
Commands: commands, run <name> [json-args]   e.g. run tool.set '{\"tool\":\"draw\"}'
State: state [key]   e.g. state panels/toolbar
Shot: shot [file.png] [--whole]   the canvas as the window shows it, as a PNG; --whole adds the panels
Mods: mod check <dir> [--state fixture.json], mod clone <name> [new-name] [--use],
      mod stats, mod state [key]
Other: breakpoints, apply, upsert, link, unlink, auto-layout, find-placement

Unknown verbs pass through to the bundled agent-browser as raw commands
(some — launch/close/quit/install/upgrade/connect/open — are blocked in
favor of specular equivalents). See the specular skill's passthrough
section for the full command surface, or run `specular skills get core`.";

type Verb = fn(&Client, &Args) -> anyhow::Result<i32>;

/// Prints `line` as a usage error and fails the command.
pub(crate) fn usage(line: &str) -> anyhow::Result<i32> {
    refuse(&format!("usage: {line}"))
}

/// Prints `message` as an error and fails the command.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a verb returns it as its own answer"
)]
pub(crate) fn refuse(message: &str) -> anyhow::Result<i32> {
    output::error(message);
    Ok(1)
}

fn verb(name: &str) -> Verb {
    match name {
        // `workspace` is the verb's older name, kept for skills that use it.
        "canvas" | "workspace" => canvas::canvas,
        "tab" => canvas::tab,
        "project" => canvas::project,
        "share" => canvas::share,
        "selection" => canvas::selection,
        "find-placement" => canvas::find_placement,
        "breakpoints" => canvas::breakpoints,
        "arrange" => canvas::arrange,
        "focus" => canvas::focus,
        "unlink" => canvas::unlink,
        "ungroup" => canvas::ungroup,
        "auto-layout" => canvas::auto_layout,
        "apply" => edit::apply,
        "upsert" => edit::upsert,
        "add" => edit::add,
        "update" => edit::update,
        "delete" => edit::delete,
        "link" => edit::link,
        "group" => edit::group,
        "annotations" => annotations::list,
        "annotation" => annotations::detail,
        "annotate" => annotations::annotate,
        "annotate-selection" => annotations::annotate_selection,
        "ack" | "resolve" | "dismiss" | "reply" => annotations::respond,
        "snapshot" => browse::snapshot,
        "click" | "fill" | "type" | "select" => browse::target,
        "screenshot" => browse::screenshot,
        "scroll" => browse::scroll,
        "wait" => browse::wait,
        "skills" => browse::skills,
        "commands" => registry::commands,
        "run" => registry::run,
        "state" => registry::state,
        "mod" => mods::verb,
        "shot" => shot::shot,
        _ => browse::passthrough,
    }
}

/// The cursor label a canvas verb shows. The browse verbs say their own.
fn presence(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "canvas" | "workspace" | "selection" | "annotations" | "annotation" => "scan_workspace",
        "find-placement" => "find_placement",
        "annotate" => "add_annotation",
        _ => return None,
    })
}

fn run(argv: &[String]) -> anyhow::Result<i32> {
    let args = args::parse(argv);
    // `--help` and `-h` are switches, so they leave no verb.
    if args.verb.is_empty() && args.switch("skill") {
        output::text_line(SKILL);
        return Ok(0);
    }
    if args.verb.is_empty() {
        output::text_line(HELP);
        return Ok(0);
    }
    if mods::is_check(&args) {
        return mods::check(&args);
    }
    if mod_clone::is_clone(&args) {
        return mod_clone::clone(&args);
    }
    let client = Client::new(args.flag("tab"));
    client.open_session();
    if let Some(label) = presence(&args.verb) {
        client.notify(
            "/session/presence",
            &json!({
                "sessionId": client.session(),
                "clientName": client.name(),
                "eventType": "act",
                "surface": "canvas",
                "phase": "acting",
                "labelKey": label,
            }),
        );
    }
    verb(&args.verb)(&client, &args)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    match run(&argv) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            output::error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}
