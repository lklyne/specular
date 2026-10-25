//! The browse verbs: a page on the canvas driven by agent-browser over the
//! page's own devtools socket, which `GET /pages/<id>/cdp-target` names.
//!
//! One agent-browser daemon runs per session and page, so two agents on
//! one page, or one agent on two pages, never share a connection.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, anyhow, bail};
use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::command::{self, Parsed};
use crate::output::{self, Block};
use crate::spawn;
use crate::usage;

/// Before every command: agent-browser marks page content, and keeps its
/// output under a limit an agent can read.
const GLOBAL_FLAGS: [&str; 3] = ["--content-boundaries", "--max-output", "100000"];
const SHORT: Duration = Duration::from_secs(5);

fn stale_ref_hint(page: &str) -> String {
    format!(
        "refs may be stale — re-run specular snapshot -i -f {page}, or target by CSS selector \
         or find text \"…\" (re-resolves every call)"
    )
}

/// The warning for a mutation on a page that has navigated since the
/// agent's last snapshot of it. Only a warning: a hot reload never counts
/// as a navigation, so the counter can never prove a ref is good.
fn generation_warning(page: &str, target: &Value) -> Option<String> {
    let current = target["generation"].as_u64()?;
    let seen = target["lastSnapshotGeneration"].as_u64()?;
    (current > seen).then(|| {
        format!(
            "page changed since your last snapshot — refs likely stale; re-run specular \
             snapshot -i -f {page}, or target by CSS selector or find text \"…\" (re-resolves \
             every call)"
        )
    })
}

/// `scheme://host[:port]` of `url`, in lower case.
fn origin(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    let scheme = scheme.to_ascii_lowercase();
    let host = host.to_ascii_lowercase();
    let host = match (scheme.as_str(), host.rsplit_once(':')) {
        ("http", Some((name, "80"))) | ("https", Some((name, "443"))) => name.to_owned(),
        _ => host,
    };
    (!scheme.is_empty()).then(|| format!("{scheme}://{host}"))
}

/// A warning when a snapshot came from another origin than the page's.
fn origin_mismatch(output: &str, expected: &str) -> Option<String> {
    let at = output.find("origin=")?;
    let actual = output[at + "origin=".len()..].split_whitespace().next()?;
    let (expected, actual) = (origin(expected)?, origin(actual)?);
    (expected != actual).then(|| {
        format!(
            "⚠ CDP target mismatch: expected {expected} but connected to {actual}. The page may \
             not have loaded yet, or the webview resolved to a different target. Try re-running \
             the command or use `specular annotation <id>` for annotation-based inspection."
        )
    })
}

/// One page's agent-browser.
struct Driver {
    prefix: Vec<String>,
}

impl Driver {
    fn new(session: &str, page: &str, cdp: &str) -> Self {
        let mut prefix: Vec<String> = GLOBAL_FLAGS.iter().map(|flag| (*flag).to_owned()).collect();
        prefix.extend(["--session".to_owned(), format!("{session}:{page}")]);
        prefix.extend(["--cdp".to_owned(), cdp.to_owned()]);
        Self { prefix }
    }

    fn run(&self, args: &[&str], timeout: Duration) -> anyhow::Result<spawn::Ran> {
        let mut all = self.prefix.clone();
        all.extend(args.iter().map(|arg| (*arg).to_owned()));
        spawn::run(&all, timeout, None)
    }
}

fn page_or_selection(client: &Client, page: Option<&str>) -> anyhow::Result<String> {
    if let Some(page) = page {
        return Ok(page.to_owned());
    }
    let selection = client.get("/selection")?;
    (selection["selectedEntityId"].as_str())
        .or_else(|| selection["selectedEntityIds"][0].as_str())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("No page specified and nothing is selected."))
}

fn screenshot_block(stdout: &str) -> Option<Block> {
    let answer: Value = serde_json::from_str(stdout).ok()?;
    let path = (answer["data"]["path"].as_str()).or_else(|| answer["path"].as_str())?;
    let data = std::fs::read(path).ok()?;
    let _ = std::fs::remove_file(path);
    let jpeg = (Path::new(path).extension())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"));
    Some(Block::Image { data, jpeg })
}

/// Tells the app's cursor what is about to happen on `page`, so it can
/// move there first.
fn tell_cursor(client: &Client, page: &str, parsed: &Parsed) {
    let Some(label) = parsed.label() else { return };
    let intent = parsed.effective();
    let editing = matches!(intent, Some("fill" | "type"));
    client.notify(
        "/session/presence/intent",
        &json!({
            "sessionId": client.session(),
            "clientName": client.name(),
            "command": intent,
            "labelKey": label,
            "pageId": page,
            "labelHint": if editing { json!("editing control") } else { Value::Null },
            "targetRef": parsed.target_ref,
            "targetRefSource": parsed.target_ref.as_ref().map(|_| "agent-browser"),
            "targetQuery": parsed.target_ref.is_none().then(|| parsed.target_query()).flatten(),
        }),
    );
}

/// Runs `argv` against `page` (or the selected entity), with the cursor
/// told first. `echo` snapshots the page again after a mutation.
fn drive(
    client: &Client,
    page: Option<&str>,
    argv: Vec<String>,
    echo: bool,
) -> anyhow::Result<Vec<Block>> {
    let page = page_or_selection(client, page)?;
    let parsed = Parsed::new(argv);
    if let Some(verb) = parsed.verb.as_deref()
        && let Some(reason) = command::blocked(verb)
    {
        bail!("{verb}: blocked — {reason}");
    }
    let target = client.get(&format!("/pages/{page}/cdp-target"))?;
    let cdp = (target["webSocketDebuggerUrl"].as_str())
        .context("the app named no devtools socket for the page")?;
    let page_url = target["url"].as_str().unwrap_or_default();
    let driver = Driver::new(client.session(), &page, cdp);

    tell_cursor(client, &page, &parsed);
    let verb = parsed.verb.as_deref().unwrap_or_default();
    let mutation = parsed.mutation().is_some();
    let warning = mutation
        .then(|| generation_warning(&page, &target))
        .flatten();
    if let Some(scroll) = parsed.scroll_target() {
        // Best effort: the click scrolls on its own anyway.
        let _ = driver.run(&["scrollintoview", scroll], SHORT);
    }
    let timeout = Duration::from_secs(if verb == "wait" { 60 } else { 30 });
    let mut args: Vec<&str> = Vec::new();
    if verb == "screenshot" {
        args.push("--json");
    }
    args.extend(parsed.argv.iter().map(String::as_str));
    let ran = driver.run(&args, timeout).and_then(spawn::Ran::checked);
    let ran = match ran {
        Ok(ran) => ran,
        Err(error) if mutation => {
            return Err(mutation_failed(&error, &page, &parsed, warning.as_deref()));
        }
        Err(error) => return Err(error),
    };
    if verb == "screenshot"
        && let Some(block) = screenshot_block(&ran.stdout)
    {
        return Ok(vec![block]);
    }
    let mut text = output_of(&ran);
    if verb == "snapshot" {
        if let Some(mismatch) = origin_mismatch(&text, page_url) {
            text = format!("{mismatch}\n{text}");
        }
        // The baseline a later mutation's staleness check compares with.
        client.notify(&format!("/pages/{page}/snapshot-seen"), &json!({}));
    }
    if let Some(warning) = &warning {
        text = format!("{warning}\n\n{text}");
    }
    if mutation
        && let Ok(url) = driver
            .run(&["get", "url"], SHORT)
            .and_then(spawn::Ran::checked)
    {
        let _ = write!(text, "\nurl: {}", url.stdout.trim());
    }
    let mut blocks = vec![Block::Text(if text.is_empty() {
        "(no output)".to_owned()
    } else {
        text
    })];
    if mutation && echo {
        blocks.extend(echo_snapshot(&driver));
    }
    Ok(blocks)
}

/// A failed mutation's error, with the staleness warning or the stale-ref
/// hint: they say the same, so one of them.
fn mutation_failed(
    error: &anyhow::Error,
    page: &str,
    parsed: &Parsed,
    warning: Option<&str>,
) -> anyhow::Error {
    let hint = if parsed.target_ref.is_some() && warning.is_none() {
        format!("\n{}", stale_ref_hint(page))
    } else {
        String::new()
    };
    let prefix = warning.map(|w| format!("{w}\n\n")).unwrap_or_default();
    anyhow::anyhow!("{prefix}{error}{hint}")
}

/// What a run printed, its errors after its output.
fn output_of(ran: &spawn::Ran) -> String {
    if ran.stderr.is_empty() {
        ran.stdout.clone()
    } else {
        format!("{}\n{}", ran.stdout, ran.stderr)
    }
    .trim()
    .to_owned()
}

/// The page's interactive elements again, after a mutation.
fn echo_snapshot(driver: &Driver) -> Option<Block> {
    let snapshot = driver.run(&["snapshot", "-i", "-c"], Duration::from_secs(10));
    let snapshot = snapshot.and_then(spawn::Ran::checked).ok()?;
    let shown = snapshot.stdout.trim();
    Some(Block::Text(
        if shown.is_empty() {
            "(no output)"
        } else {
            shown
        }
        .to_owned(),
    ))
}

fn run(client: &Client, args: &Args, argv: Vec<String>) -> anyhow::Result<i32> {
    let blocks = drive(client, args.page(), argv, args.switch("echo"))?;
    output::blocks(&blocks)?;
    Ok(0)
}

fn words(verb: &str, rest: &[&str]) -> Vec<String> {
    std::iter::once(verb)
        .chain(rest.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// `snapshot [-i] [-c] [-u] [-s SEL] [-d N] [--format F]`.
pub(crate) fn snapshot(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let mut argv = words("snapshot", &[]);
    for (short, long) in [("i", "interactive"), ("c", "compact"), ("u", "urls")] {
        if args.switch(short) || args.switch(long) {
            argv.push(format!("-{short}"));
        }
    }
    for (flag, as_flag) in [
        ("s", "-s"),
        ("selector", "-s"),
        ("d", "-d"),
        ("depth", "-d"),
        ("format", "--format"),
    ] {
        if let Some(value) = args.flag(flag) {
            argv.extend([as_flag.to_owned(), value.to_owned()]);
        }
    }
    run(client, args, argv)
}

/// `click`, `fill`, `type` and `select`: a target, and text for all but
/// `click`.
pub(crate) fn target(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let verb = args.verb.as_str();
    let text = args.words(1);
    let (needs_text, what) = match verb {
        "click" => (false, ""),
        "select" => (true, " <value>"),
        _ => (true, " <text>"),
    };
    let Some(target) = args.nth(0).filter(|_| !needs_text || !text.is_empty()) else {
        return usage(&format!("specular {verb} <ref>{what} [--echo]"));
    };
    let mut argv = words(verb, &[target]);
    if needs_text {
        argv.push(text);
    }
    run(client, args, argv)
}

/// `screenshot [--annotate]`.
pub(crate) fn screenshot(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let annotate: &[&str] = if args.switch("annotate") {
        &["--annotate"]
    } else {
        &[]
    };
    run(client, args, words("screenshot", annotate))
}

/// `scroll [direction] [amount]`.
pub(crate) fn scroll(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let direction = args.nth(0).unwrap_or("down");
    let amount: Vec<&str> = args.nth(1).into_iter().collect();
    let mut argv = words("scroll", &[direction]);
    argv.extend(amount.iter().map(|amount| (*amount).to_owned()));
    run(client, args, argv)
}

/// `wait [ms|selector] [--load S] [--timeout N] [--text T] [--url U]`.
pub(crate) fn wait(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let mut argv = words("wait", &[]);
    if let Some(load) = args.flag("load") {
        argv.extend(["--load".to_owned(), load.to_owned()]);
    }
    argv.extend(args.nth(0).map(str::to_owned));
    for flag in ["timeout", "text", "url"] {
        if let Some(value) = args.flag(flag) {
            argv.extend([format!("--{flag}"), value.to_owned()]);
        }
    }
    run(client, args, argv)
}

/// The words of a passthrough without Specular's own `--page` and `-f`.
fn without_page(rest: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        if word == "--page" || word == "-f" {
            words.next();
            continue;
        }
        out.push(word.clone());
    }
    out
}

/// Any other verb: agent-browser's own command, on the page.
pub(crate) fn passthrough(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if let Some(reason) = command::blocked(&args.verb) {
        return crate::refuse(&format!("specular {}: blocked — {reason}", args.verb));
    }
    let mut argv = vec![args.verb.clone()];
    argv.extend(without_page(&args.rest));
    run(client, args, argv)
}

/// `skills ...`: agent-browser's own documentation, with no page.
pub(crate) fn skills(_client: &Client, args: &Args) -> anyhow::Result<i32> {
    let mut argv = vec!["skills".to_owned()];
    argv.extend(without_page(&args.rest));
    let ran = spawn::run(&argv, Duration::from_secs(30), None)?.checked()?;
    output::text_line(ran.stdout.trim_end());
    Ok(0)
}
