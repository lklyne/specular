use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;
use specular_agent::{RunConfig, RunRequest, claude_args};

use super::{AgentBackend, AgentError, AgentProcess, Output, Waker};

/// Where a GUI app launched from Finder, with a bare `PATH`, can still find it.
const FALLBACKS: [&str; 3] = [
    ".local/bin/claude",
    "/opt/homebrew/bin/claude",
    "/usr/local/bin/claude",
];

/// The real thing: the `claude` CLI as a child process.
#[derive(Default)]
pub struct ClaudeCli {
    program: Option<PathBuf>,
    waker: Option<Waker>,
}

impl std::fmt::Debug for ClaudeCli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeCli")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}

impl ClaudeCli {
    /// Finds the program on each start.
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs this program instead of looking for `claude`.
    #[must_use]
    pub fn with_program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = Some(program.into());
        self
    }

    fn program(&self) -> Result<PathBuf, AgentError> {
        if let Some(program) = &self.program {
            return Ok(program.clone());
        }
        find_claude().ok_or(AgentError::NotInstalled)
    }
}

fn find_claude() -> Option<PathBuf> {
    if let Some(bin) = std::env::var_os("SPECULAR_CLAUDE_BIN").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(bin));
    }
    let on_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join("claude"))
            .find(|candidate| candidate.is_file())
    });
    on_path.or_else(|| {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        FALLBACKS
            .iter()
            .map(|fallback| match &home {
                Some(home) if !fallback.starts_with('/') => home.join(fallback),
                _ => PathBuf::from(fallback),
            })
            .find(|candidate| candidate.is_file())
    })
}

impl AgentBackend for ClaudeCli {
    fn start(
        &mut self,
        request: &RunRequest,
        space: &Path,
        cwd: &Path,
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        let program = self.program()?;
        let mut args = claude_args(request, &RunConfig::default());
        let input = stdin_text(request, space);
        if input.streams_json {
            args.push("--input-format".into());
            args.push("stream-json".into());
        }
        self.spawn(&program, &args, input, cwd)
    }

    fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }
}

impl ClaudeCli {
    fn spawn(
        &self,
        program: &Path,
        args: &[String],
        input: Input,
        cwd: &Path,
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => AgentError::NotInstalled,
                _ => AgentError::Spawn(error),
            })?;
        let (tx, rx) = mpsc::channel();
        let mut threads = Vec::new();
        if let Some(mut stdin) = child.stdin.take() {
            // Off the loop's thread: a big image would block on a full pipe.
            threads.push(std::thread::spawn(move || {
                // A child that exited early closes the pipe; its exit says why.
                let _ = stdin.write_all(input.text.as_bytes());
            }));
        }
        if let Some(stdout) = child.stdout.take() {
            threads.push(reader(
                stdout,
                tx.clone(),
                self.waker.clone(),
                Output::Stdout,
            ));
        }
        if let Some(stderr) = child.stderr.take() {
            threads.push(reader(
                stderr,
                tx.clone(),
                self.waker.clone(),
                Output::Stderr,
            ));
        }
        let child = Arc::new(Mutex::new(child));
        let waiter = wait_for_exit(Arc::clone(&child), threads, tx, self.waker.clone());
        Ok(Box::new(Process {
            child,
            output: rx,
            waiter: Some(waiter),
        }))
    }

    #[cfg(test)]
    fn start_with_args(
        &self,
        request: &RunRequest,
        cwd: &Path,
        args: &[String],
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        let program = self.program()?;
        self.spawn(&program, args, stdin_text(request, cwd), cwd)
    }
}

struct Input {
    text: String,
    streams_json: bool,
}

/// What goes to stdin: the prompt, or one stream-json user message when
/// images come along, read from the space folder `space`.
fn stdin_text(request: &RunRequest, space: &Path) -> Input {
    if request.images.is_empty() {
        return Input {
            text: request.prompt.clone(),
            streams_json: false,
        };
    }
    let mut content: Vec<serde_json::Value> = request
        .images
        .iter()
        .filter_map(|image| {
            let bytes = std::fs::read(space.join(&image.path)).ok()?;
            Some(json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": image.media_type.as_str(),
                    "data": STANDARD.encode(bytes),
                },
            }))
        })
        .collect();
    content.push(json!({ "type": "text", "text": request.prompt }));
    let message = json!({ "type": "user", "message": { "role": "user", "content": content } });
    Input {
        text: format!("{message}\n"),
        streams_json: true,
    }
}

fn reader<R: std::io::Read + Send + 'static>(
    source: R,
    tx: Sender<Output>,
    waker: Option<Waker>,
    wrap: fn(String) -> Output,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(source).lines() {
            let Ok(line) = line else { break };
            if tx.send(wrap(line)).is_err() {
                break;
            }
            if let Some(wake) = &waker {
                wake();
            }
        }
    })
}

fn wait_for_exit(
    child: Arc<Mutex<Child>>,
    readers: Vec<JoinHandle<()>>,
    tx: Sender<Output>,
    waker: Option<Waker>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let status = loop {
            let polled = match child.lock() {
                Ok(mut child) => child.try_wait(),
                Err(_) => return,
            };
            match polled {
                Ok(Some(status)) => break Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => break None,
            }
        };
        // Every line is in the channel before the exit.
        for reader in readers {
            let _ = reader.join();
        }
        let exit = match status {
            Some(status) => Output::Exit {
                success: status.success(),
                detail: status.to_string(),
            },
            None => Output::Exit {
                success: false,
                detail: "the process could not be waited on".into(),
            },
        };
        let _ = tx.send(exit);
        if let Some(wake) = &waker {
            wake();
        }
    })
}

struct Process {
    child: Arc<Mutex<Child>>,
    output: Receiver<Output>,
    waiter: Option<JoinHandle<()>>,
}

impl AgentProcess for Process {
    fn poll(&mut self) -> Vec<Output> {
        self.output.try_iter().collect()
    }

    fn kill(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            // Already gone is the goal; `wait` reaps it either way.
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(waiter) = self.waiter.take() {
            // The pipes close with the child, so the readers and the waiter end.
            let _ = waiter.join();
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specular_agent::{Image, MediaType, ThreadId};

    fn request(images: Vec<Image>) -> RunRequest {
        RunRequest {
            thread: ThreadId("t".into()),
            prompt: "hello".into(),
            resume: None,
            images,
            cwd: None,
        }
    }

    fn drain(process: &mut dyn AgentProcess) -> Vec<Output> {
        let mut all = Vec::new();
        for _ in 0..500 {
            all.extend(process.poll());
            if all.iter().any(|o| matches!(o, Output::Exit { .. })) {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        all
    }

    #[test]
    fn the_prompt_reaches_stdin_and_output_lines_come_back_before_the_exit() {
        let cli = ClaudeCli::new().with_program("/bin/cat");
        // `cat` ignores the claude flags only if they are files; give it none.
        let request = request(Vec::new());
        let mut process = cli
            .start_with_args(&request, Path::new("/tmp"), &[])
            .unwrap();
        let output = drain(&mut *process);
        assert_eq!(output.first(), Some(&Output::Stdout("hello".into())));
        assert!(matches!(
            output.last(),
            Some(Output::Exit { success: true, .. })
        ));

        // Output written after the child itself has exited (a grandchild
        // still holds the pipe) still lands ahead of the exit.
        let mut process = cli
            .with_program("/bin/sh")
            .start_with_args(
                &request,
                Path::new("/tmp"),
                &["-c".into(), "(sleep 0.3; echo late) & exit 0".into()],
            )
            .unwrap();
        let output = drain(&mut *process);
        assert_eq!(output.first(), Some(&Output::Stdout("late".into())));
        assert!(matches!(output.last(), Some(Output::Exit { .. })));
    }

    #[test]
    fn kill_ends_a_process_that_never_would() {
        let cli = ClaudeCli::new().with_program("/bin/sleep");
        let mut process = cli
            .start_with_args(&request(Vec::new()), Path::new("/tmp"), &["30".into()])
            .unwrap();
        let started = std::time::Instant::now();
        process.kill();
        process.kill();
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "kill waited for the child to finish by itself"
        );
        assert!(
            process
                .poll()
                .iter()
                .all(|o| !matches!(o, Output::Stdout(_)))
        );
    }

    #[test]
    fn a_missing_program_says_how_to_install_claude() {
        let mut cli = ClaudeCli::new().with_program("/nonexistent/claude");
        let error = cli
            .start(&request(Vec::new()), Path::new("/tmp"), Path::new("/tmp"))
            .err()
            .unwrap();
        assert!(matches!(error, AgentError::NotInstalled));
        assert!(error.to_string().contains("run `claude`"));
    }

    #[test]
    fn images_become_one_stream_json_message_and_missing_files_are_skipped() {
        let dir = std::env::temp_dir().join(format!("specular-agent-img-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.png"), [0xfb, 0xff, 0xfe]).unwrap();
        let images = vec![
            Image {
                path: "a.png".into(),
                media_type: MediaType::Png,
            },
            Image {
                path: "gone.png".into(),
                media_type: MediaType::Png,
            },
        ];
        let input = stdin_text(&request(images), &dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(input.streams_json && input.text.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(input.text.trim()).unwrap();
        let content = value["message"]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["source"]["data"], "+//+");
        assert_eq!(content[0]["source"]["media_type"], "image/png");
        assert_eq!(content[1]["text"], "hello");
    }
}
