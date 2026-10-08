use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specular_agent::RunRequest;

use super::{AgentBackend, AgentError, AgentProcess, Output};

/// Every request a [`Scripted`] backend was started with, and where.
pub type StartedRuns = Arc<Mutex<Vec<(RunRequest, PathBuf)>>>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Stdout(String),
    Stderr(String),
    Wait(Duration),
    Exit(i32),
}

/// A fake `claude` that plays a script, for tests and for driving the UI by
/// hand. One step a line: a line starting with `{` is a stdout line,
/// `wait <ms>`, `stderr <text>`, `exit <code>` (the default at the end is
/// 0). Blank lines and `#` comments are ignored. Every start replays the
/// script from the top.
#[derive(Debug, Clone)]
pub struct Scripted {
    steps: Vec<Step>,
    instant: bool,
    started: StartedRuns,
}

impl Scripted {
    /// Reads a script. Waits are real time.
    pub fn from_text(text: &str) -> Result<Self, AgentError> {
        let mut steps = Vec::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            steps.push(parse_step(line)?);
        }
        Ok(Self {
            steps,
            instant: false,
            started: StartedRuns::default(),
        })
    }

    /// Reads a script file.
    pub fn from_file(path: &Path) -> Result<Self, AgentError> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| AgentError::Script(format!("{}: {error}", path.display())))?;
        Self::from_text(&text)
    }

    /// Plays every `wait` at once, so a test never sleeps.
    #[must_use]
    pub fn instant(mut self) -> Self {
        self.instant = true;
        self
    }

    /// The requests started so far; reads stay valid after the backend moved.
    pub fn started(&self) -> StartedRuns {
        Arc::clone(&self.started)
    }
}

fn parse_step(line: &str) -> Result<Step, AgentError> {
    if line.starts_with('{') {
        return Ok(Step::Stdout(line.to_owned()));
    }
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    let bad = || AgentError::Script(format!("cannot read `{line}`"));
    match word {
        "wait" => rest
            .trim()
            .parse()
            .map(|ms| Step::Wait(Duration::from_millis(ms)))
            .map_err(|_| bad()),
        "stderr" => Ok(Step::Stderr(rest.to_owned())),
        "exit" => rest.trim().parse().map(Step::Exit).map_err(|_| bad()),
        _ => Err(bad()),
    }
}

impl AgentBackend for Scripted {
    fn start(
        &mut self,
        request: &RunRequest,
        _space: &Path,
        cwd: &Path,
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        if let Ok(mut started) = self.started.lock() {
            started.push((request.clone(), cwd.to_path_buf()));
        }
        Ok(Box::new(Play {
            steps: self.steps.iter().cloned().collect(),
            instant: self.instant,
            due: Instant::now(),
            done: false,
        }))
    }
}

struct Play {
    steps: VecDeque<Step>,
    instant: bool,
    due: Instant,
    done: bool,
}

impl AgentProcess for Play {
    fn poll(&mut self) -> Vec<Output> {
        let mut out = Vec::new();
        let now = Instant::now();
        while !self.done && now >= self.due {
            match self.steps.pop_front() {
                Some(Step::Stdout(line)) => out.push(Output::Stdout(line)),
                Some(Step::Stderr(line)) => out.push(Output::Stderr(line)),
                Some(Step::Wait(wait)) => {
                    if !self.instant {
                        self.due += wait;
                    }
                }
                Some(Step::Exit(code)) => out.push(exit(code, &mut self.done)),
                None => out.push(exit(0, &mut self.done)),
            }
        }
        out
    }

    fn kill(&mut self) {
        self.done = true;
        self.steps.clear();
    }
}

fn exit(code: i32, done: &mut bool) -> Output {
    *done = true;
    Output::Exit {
        success: code == 0,
        detail: format!("exit code {code}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_reads_each_kind_of_step_and_rejects_the_rest() {
        let text = "# hi\n\n{\"type\":\"x\"}\nwait 5\nstderr boom\nexit 2\n";
        let script = Scripted::from_text(text).unwrap();
        assert_eq!(
            script.steps,
            [
                Step::Stdout("{\"type\":\"x\"}".to_owned()),
                Step::Wait(Duration::from_millis(5)),
                Step::Stderr("boom".to_owned()),
                Step::Exit(2),
            ]
        );
        assert!(Scripted::from_text("dance").is_err());
        assert!(Scripted::from_text("wait soon").is_err());
        assert!(Scripted::from_text("exit soon").is_err());
    }
}
