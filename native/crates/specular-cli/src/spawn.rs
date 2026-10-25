//! Running agent-browser: found the way the app bundle ships it, given a
//! time limit, and its output collected.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};

/// Output past this much is dropped.
const MAX_OUTPUT: u64 = 10 * 1024 * 1024;

/// What a finished run printed.
#[derive(Debug, Clone)]
pub(crate) struct Ran {
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    /// The exit code, `None` when a signal ended it.
    pub(crate) code: Option<i32>,
}

impl Ran {
    /// This run, or an error carrying what it said when it failed.
    pub(crate) fn checked(self) -> anyhow::Result<Self> {
        if self.code == Some(0) {
            return Ok(self);
        }
        let said = [&self.stderr, &self.stdout]
            .into_iter()
            .find(|text| !text.is_empty())
            .cloned();
        bail!(said.unwrap_or_else(|| match self.code {
            Some(code) => format!("Process exited with code {code}"),
            None => "Process was killed".to_owned(),
        }))
    }
}

/// `AGENT_BROWSER_PATH`, else the copy in the app bundle beside this
/// executable (`Contents/Resources/bin`), else the first on the `PATH`.
pub(crate) fn agent_browser() -> PathBuf {
    if let Some(path) = std::env::var_os("AGENT_BROWSER_PATH").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    let bundled = (std::env::current_exe().ok())
        .and_then(|exe| exe.canonicalize().ok())
        .and_then(|exe| Some(exe.parent()?.join("../Resources/bin/agent-browser")))
        .filter(|path| path.is_file());
    if let Some(bundled) = bundled {
        return bundled;
    }
    let on_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("agent-browser"))
            .find(|candidate| candidate.is_file())
    });
    on_path.unwrap_or_else(|| PathBuf::from("agent-browser"))
}

fn drain(mut from: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut from).take(MAX_OUTPUT).read_to_end(&mut bytes);
        // Read the rest so the child never blocks on a full pipe.
        let _ = std::io::copy(&mut from, &mut std::io::sink());
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

/// Runs agent-browser with `args`, `input` on its stdin, killing it after
/// `timeout`.
pub(crate) fn run(args: &[String], timeout: Duration, input: Option<&str>) -> anyhow::Result<Ran> {
    let program = agent_browser();
    let mut command = Command::new(&program);
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NO_COLOR", "1");
    // One daemon runs per page; without an idle timeout they would pile up.
    if std::env::var_os("AGENT_BROWSER_IDLE_TIMEOUT_MS").is_none() {
        command.env("AGENT_BROWSER_IDLE_TIMEOUT_MS", "60000");
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("could not run {}", program.display()))?;
    // Dropping stdin closes it, so the child never waits for more.
    if let Some(mut stdin) = child.stdin.take()
        && let Some(input) = input
    {
        let _ = stdin.write_all(input.as_bytes());
    }
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Command timed out after {}ms", timeout.as_millis());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let collect = |handle: Option<std::thread::JoinHandle<String>>| {
        handle
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    };
    Ok(Ran {
        stdout: collect(stdout),
        stderr: collect(stderr),
        code: status.code(),
    })
}
