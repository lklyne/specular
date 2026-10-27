use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;
use specular_agent::{RunConfig, RunRequest, claude_args};

use super::{AgentBackend, AgentError, AgentProcess, Waker, process};

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

    fn program(&self) -> Result<PathBuf, AgentError> {
        if let Some(program) = &self.program {
            return Ok(program.clone());
        }
        find_claude().ok_or(AgentError::NotInstalled)
    }
}

/// The `claude` the user installed, wherever a GUI app can find it.
pub(super) fn find_claude() -> Option<PathBuf> {
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
        self.spawn(&program, &args, input, cwd, request.thread.as_str())
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
        thread: &str,
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        // The `specular` CLI the run calls sends this back with each
        // request, which is how a canvas change is known to be this run's.
        let env = [(specular_api::SESSION_ENV, thread)];
        process::spawn(program, args, input.text, cwd, &env, self.waker.clone()).map_err(|error| {
            match error.kind() {
                std::io::ErrorKind::NotFound => AgentError::NotInstalled,
                _ => AgentError::Spawn(error),
            }
        })
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
            model: None,
        }
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
