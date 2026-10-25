use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use specular_agent::{
    Model, Notice, Progress, ProgressKind, RunRequest, ThreadId, parse_line, parse_models,
};

use super::{AgentBackend, AgentProcess, Output};

/// A run with no end after this long is stopped.
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_mins(10);

/// The models are listed within this long, or not at all.
const MODELS_TIMEOUT: Duration = Duration::from_secs(20);

const STDERR_LINE_MAX: usize = 320;

/// The backend being asked which models it can run.
struct ModelsAsk {
    process: Box<dyn AgentProcess>,
    started: Instant,
}

struct Run {
    process: Box<dyn AgentProcess>,
    started: Instant,
    stderr_tail: Option<String>,
}

/// The runs in flight, one per thread, and the rules for how each ends:
/// exactly once, as `Finished`, `Failed` or `Cancelled`.
pub struct AgentRuns {
    backend: Box<dyn AgentBackend>,
    runs: BTreeMap<ThreadId, Run>,
    queued: Vec<(ThreadId, Notice)>,
    timeout: Duration,
    models: Option<ModelsAsk>,
}

impl std::fmt::Debug for AgentRuns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuns")
            .field("running", &self.runs.len())
            .finish_non_exhaustive()
    }
}

impl AgentRuns {
    /// Runs over `backend`.
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        Self {
            backend,
            runs: BTreeMap::new(),
            queued: Vec::new(),
            timeout: DEFAULT_TIMEOUT,
            models: None,
        }
    }

    /// Starts a run in the request's `cwd`, or in the space folder `space`
    /// when it names none. A request for a thread that
    /// already has a run is ignored: the model never asks, and the run
    /// going keeps the thread. A spawn error, or no folder, is one `Failed`
    /// from the next [`poll`](Self::poll).
    pub fn start(&mut self, request: &RunRequest, space: Option<&Path>) {
        if self.runs.contains_key(&request.thread) {
            tracing::warn!("a run for {} is already going", request.thread.as_str());
            return;
        }
        let failed = |error: String| (request.thread.clone(), Notice::Failed { error });
        let Some(space) = space else {
            self.queued
                .push(failed("Open a folder first: the agent works in it.".into()));
            return;
        };
        let cwd = request
            .cwd
            .as_deref()
            .map_or_else(|| space.to_path_buf(), PathBuf::from);
        tracing::info!(thread = request.thread.as_str(), cwd = %cwd.display(), "agent run starts");
        match self.backend.start(request, space, &cwd) {
            Ok(process) => {
                self.runs.insert(
                    request.thread.clone(),
                    Run {
                        process,
                        started: Instant::now(),
                        stderr_tail: None,
                    },
                );
            }
            Err(error) => self.queued.push(failed(error.to_string())),
        }
    }

    /// Stops the thread's run and answers with one `Cancelled`. A thread
    /// with no run gets nothing.
    pub fn cancel(&mut self, thread: &ThreadId) {
        if let Some(mut run) = self.runs.remove(thread) {
            run.process.kill();
            self.queued.push((thread.clone(), Notice::Cancelled));
        }
    }

    /// What happened since the last call, in order, never blocking.
    pub fn poll(&mut self) -> Vec<(ThreadId, Notice)> {
        self.poll_at(Instant::now())
    }

    /// [`poll`](Self::poll) with the clock given, for the timeout.
    pub(crate) fn poll_at(&mut self, now: Instant) -> Vec<(ThreadId, Notice)> {
        let mut out = std::mem::take(&mut self.queued);
        let mut ended = Vec::new();
        for (thread, run) in &mut self.runs {
            let end = drain(run, thread, &mut out).or_else(|| {
                (now.saturating_duration_since(run.started) >= self.timeout)
                    .then(|| Notice::Failed {
                        error: "The agent timed out after 10 minutes".into(),
                    })
                    .inspect(|failed| out.push((thread.clone(), failed.clone())))
            });
            if end.is_some() {
                ended.push(thread.clone());
            }
        }
        for thread in ended {
            if let Some(mut run) = self.runs.remove(&thread) {
                run.process.kill();
            }
        }
        out
    }

    /// Asks the backend which models a run can use. The answer comes from
    /// [`take_models`](Self::take_models), once, if the backend can say.
    pub(crate) fn ask_models(&mut self) {
        self.models = self.backend.ask_models().map(|process| ModelsAsk {
            process,
            started: Instant::now(),
        });
    }

    /// The models the backend listed since the last call, never blocking.
    pub(crate) fn take_models(&mut self) -> Option<Vec<Model>> {
        let ask = self.models.as_mut()?;
        let mut over = ask.started.elapsed() >= MODELS_TIMEOUT;
        let mut listed = None;
        for output in ask.process.poll() {
            match output {
                Output::Stdout(line) => listed = listed.or_else(|| parse_models(&line)),
                Output::Stderr(line) => tracing::debug!("models: {}", truncate(&line)),
                Output::Exit { .. } => over = true,
            }
        }
        if listed.is_some() || over {
            if listed.is_none() {
                tracing::warn!("the agent listed no models; the picker keeps the aliases");
            }
            if let Some(mut ask) = self.models.take() {
                ask.process.kill();
            }
        }
        listed
    }

    /// Whether nothing is running and nothing waits to be polled; a loop
    /// that is idle has no reason to poll.
    pub fn is_idle(&self) -> bool {
        self.runs.is_empty() && self.queued.is_empty() && self.models.is_none()
    }
}

/// Reads one run's output into `out`. `Some` once the run has ended.
fn drain(run: &mut Run, thread: &ThreadId, out: &mut Vec<(ThreadId, Notice)>) -> Option<Notice> {
    for output in run.process.poll() {
        match output {
            Output::Stdout(line) => {
                for notice in parse_line(&line) {
                    let end = matches!(notice, Notice::Finished { .. } | Notice::Failed { .. });
                    out.push((thread.clone(), notice.clone()));
                    if end {
                        return Some(notice);
                    }
                }
            }
            Output::Stderr(line) => {
                out.push((
                    thread.clone(),
                    Notice::Progress(Progress {
                        kind: ProgressKind::Stderr,
                        text: truncate(&line),
                        label: None,
                    }),
                ));
                run.stderr_tail = Some(line);
            }
            Output::Exit { detail, .. } => {
                let tail = run
                    .stderr_tail
                    .as_deref()
                    .map(|line| format!(": {}", truncate(line)))
                    .unwrap_or_default();
                let failed = Notice::Failed {
                    error: format!("The agent stopped without a result ({detail}){tail}"),
                };
                out.push((thread.clone(), failed.clone()));
                return Some(failed);
            }
        }
    }
    None
}

fn truncate(line: &str) -> String {
    if line.chars().count() <= STDERR_LINE_MAX {
        return line.to_owned();
    }
    let mut cut: String = line.chars().take(STDERR_LINE_MAX - 1).collect();
    cut.push('…');
    cut
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Scripted;

    const SESSION: &str = r#"{"type":"system","subtype":"init","session_id":"s1","model":"m"}"#;
    const DONE: &str =
        r#"{"type":"result","subtype":"success","is_error":false,"result":"all done"}"#;
    const BROKEN: &str =
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"errors":["no"]}"#;

    fn id(name: &str) -> ThreadId {
        ThreadId(name.into())
    }

    fn request(name: &str) -> RunRequest {
        RunRequest {
            thread: id(name),
            prompt: "p".into(),
            resume: None,
            images: Vec::new(),
            cwd: None,
            model: None,
        }
    }

    fn runs(script: &str) -> AgentRuns {
        AgentRuns::new(Box::new(Scripted::from_text(script).unwrap().instant()))
    }

    fn notices(runs: &mut AgentRuns, name: &str) -> Vec<Notice> {
        runs.poll()
            .into_iter()
            .filter(|(t, _)| t.as_str() == name)
            .map(|(_, n)| n)
            .collect()
    }

    fn started(runs: &mut AgentRuns, thread: &str) -> Vec<Notice> {
        runs.start(&request(thread), Some(Path::new("/space")));
        notices(runs, thread)
    }

    #[test]
    fn stdout_lines_become_notices_in_order_and_the_run_ends_with_the_result() {
        let mut r = runs(&format!("{SESSION}\n{DONE}\nstderr late\n{BROKEN}"));
        let got = started(&mut r, "t");
        assert_eq!(got[0], Notice::Session("s1".into()));
        assert_eq!(
            got.last(),
            Some(&Notice::Finished {
                text: "all done".into()
            })
        );
        assert!(
            !got.iter().any(|n| matches!(n, Notice::Failed { .. })),
            "{got:?}"
        );
        assert!(r.is_idle());
        assert_eq!(r.poll(), vec![]);
    }
}
