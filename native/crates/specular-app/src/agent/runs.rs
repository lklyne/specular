use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use specular_agent::{Notice, Progress, ProgressKind, RunRequest, ThreadId, parse_line};

use super::{AgentBackend, AgentProcess, Output, Waker};

/// A run with no end after this long is stopped.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(10);

const STDERR_LINE_MAX: usize = 320;

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
        }
    }

    /// Stops a run that has gone on this long.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Hands the backend the hook to call when output arrives.
    pub fn set_waker(&mut self, waker: Waker) {
        self.backend.set_waker(waker);
    }

    /// Starts a run in `cwd`, the space folder. A request for a thread that
    /// already has a run is ignored: the model never asks, and the run
    /// going keeps the thread. A spawn error, or no folder, is one `Failed`
    /// from the next [`poll`](Self::poll).
    pub fn start(&mut self, request: &RunRequest, cwd: Option<&Path>) {
        if self.runs.contains_key(&request.thread) {
            tracing::warn!("a run for {} is already going", request.thread.as_str());
            return;
        }
        let failed = |error: String| (request.thread.clone(), Notice::Failed { error });
        let Some(cwd) = cwd else {
            self.queued
                .push(failed("Open a folder first: the agent works in it.".into()));
            return;
        };
        match self.backend.start(request, cwd) {
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
    pub fn poll_at(&mut self, now: Instant) -> Vec<(ThreadId, Notice)> {
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

    /// Whether nothing is running and nothing waits to be polled; a loop
    /// that is idle has no reason to poll.
    pub fn is_idle(&self) -> bool {
        self.runs.is_empty() && self.queued.is_empty()
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
    use crate::agent::{Disabled, Scripted};

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

    #[test]
    fn a_failed_result_ends_the_run_once() {
        let mut r = runs(&format!("{BROKEN}\n{DONE}"));
        let got = started(&mut r, "t");
        assert_eq!(got.last(), Some(&Notice::Failed { error: "no".into() }));
        assert_eq!(
            got.iter()
                .filter(|n| matches!(n, Notice::Finished { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn a_spawn_error_or_no_folder_is_one_failed_notice() {
        let mut off = AgentRuns::new(Box::new(Disabled));
        off.start(&request("t"), Some(Path::new("/space")));
        assert!(!off.is_idle());
        let got = notices(&mut off, "t");
        assert!(matches!(&got[..], [Notice::Failed { error }] if error.contains("off")));
        assert!(off.is_idle());

        let mut r = runs(DONE);
        r.start(&request("t"), None);
        assert!(matches!(&notices(&mut r, "t")[..], [Notice::Failed { .. }]));
    }

    #[test]
    fn stderr_lines_are_progress_cut_to_320_and_name_the_failure_when_no_result_comes() {
        let long = "x".repeat(400);
        let mut r = runs(&format!("stderr first\nstderr {long}\nexit 3"));
        let got = started(&mut r, "t");
        let Notice::Progress(p) = &got[1] else {
            panic!("{got:?}")
        };
        assert_eq!(
            (p.kind, p.text.chars().count(), p.label.clone()),
            (ProgressKind::Stderr, 320, None)
        );
        let Some(Notice::Failed { error }) = got.last() else {
            panic!("{got:?}")
        };
        assert!(
            error.contains("exit code 3") && error.contains('x'),
            "{error}"
        );
        assert_eq!(
            got.iter()
                .filter(|n| matches!(n, Notice::Failed { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn an_exit_with_no_result_and_no_stderr_still_fails() {
        let mut r = runs("");
        let got = started(&mut r, "t");
        assert!(
            matches!(&got[..], [Notice::Failed { error }] if error.contains("without a result"))
        );
    }

    #[test]
    fn cancel_gives_one_cancelled_and_drops_later_lines_and_nothing_without_a_run() {
        let mut r = runs(&format!("wait 1\n{DONE}"));
        r.cancel(&id("t"));
        assert_eq!(r.poll(), vec![]);
        r.start(&request("t"), Some(Path::new("/space")));
        r.cancel(&id("t"));
        assert_eq!(notices(&mut r, "t"), vec![Notice::Cancelled]);
        assert!(r.is_idle());
        assert_eq!(r.poll(), vec![]);
    }

    #[test]
    fn a_run_with_no_end_times_out_after_ten_minutes() {
        // A script that waits in real time never ends within the test.
        let mut r = AgentRuns::new(Box::new(Scripted::from_text("wait 3600000").unwrap()));
        r.start(&request("t"), Some(Path::new("/space")));
        assert_eq!(r.poll(), vec![]);
        let later = Instant::now() + DEFAULT_TIMEOUT;
        let got: Vec<_> = r.poll_at(later).into_iter().map(|(_, n)| n).collect();
        assert_eq!(
            got,
            vec![Notice::Failed {
                error: "The agent timed out after 10 minutes".into()
            }]
        );
        assert!(r.is_idle());
    }

    #[test]
    fn runs_of_different_threads_overlap_independently() {
        let mut r = runs(DONE);
        r.start(&request("a"), Some(Path::new("/space")));
        r.start(&request("b"), Some(Path::new("/space")));
        r.cancel(&id("a"));
        let got = r.poll();
        assert!(
            got.iter()
                .any(|(t, n)| t.as_str() == "a" && *n == Notice::Cancelled)
        );
        assert!(
            got.iter()
                .any(|(t, n)| t.as_str() == "b" && matches!(n, Notice::Finished { .. }))
        );
    }

    #[test]
    fn a_second_start_for_a_thread_with_a_run_is_ignored() {
        let script = Scripted::from_text("wait 3600000").unwrap();
        let log = script.started();
        let mut r = AgentRuns::new(Box::new(script));
        r.start(&request("t"), Some(Path::new("/space")));
        r.start(&request("t"), Some(Path::new("/space")));
        assert_eq!(log.lock().unwrap().len(), 1);
        assert_eq!(r.poll(), vec![]);
    }

    #[test]
    fn every_start_replays_the_script_and_records_the_request() {
        let script = Scripted::from_text(DONE).unwrap().instant();
        let log = script.started();
        let mut r = AgentRuns::new(Box::new(script));
        for _ in 0..2 {
            r.start(&request("t"), Some(Path::new("/space")));
            assert!(matches!(
                notices(&mut r, "t").last(),
                Some(Notice::Finished { .. })
            ));
        }
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].1, Path::new("/space"));
    }
}
