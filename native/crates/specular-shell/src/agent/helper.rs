//! The canvas agent on the Claude Agent SDK, through `specular-helper`: a
//! Bun-compiled program beside the app (`native/helper/`) that runs the SDK
//! and writes the SDK's messages to stdout, one a line. They are the lines
//! `claude --output-format stream-json` writes, so one parser reads both.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use specular_agent::{ALLOWED_TOOLS, Permissions, RunConfig, RunRequest, SPECULAR_CONTEXT};

use super::claude::find_claude;
use super::{AgentBackend, AgentError, AgentProcess, Waker, process};

/// The helper's name, in the bundle's `Contents/MacOS` and in the dev build.
const HELPER: &str = "specular-helper";

/// The Claude Code runtime the SDK runs, in the bundle's `Contents/Helpers`.
const RUNTIME: &str = "../Helpers/claude";

/// The runtime `bun install` puts under `native/helper/`.
const DEV_RUNTIME: &str = "node_modules/@anthropic-ai/claude-agent-sdk-darwin-arm64/claude";

/// The helper and the runtime it starts.
pub struct Helper {
    program: PathBuf,
    runtime: PathBuf,
    waker: Option<Waker>,
}

impl std::fmt::Debug for Helper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Helper")
            .field("program", &self.program)
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}

impl Helper {
    /// The helper and a runtime, when both are found: each from its
    /// environment variable (`SPECULAR_HELPER`, `SPECULAR_CLAUDE_RUNTIME`),
    /// else the app bundle, else the dev build under `native/helper/`. A
    /// runtime is last looked for as the `claude` the user installed.
    pub(crate) fn find() -> Option<Self> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf));
        let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../helper");
        let program = from_env("SPECULAR_HELPER").or_else(|| {
            [
                exe_dir.as_ref().map(|dir| dir.join(HELPER)),
                Some(dev.join("dist").join(HELPER)),
            ]
            .into_iter()
            .flatten()
            .find(|path| path.is_file())
        })?;
        let runtime = from_env("SPECULAR_CLAUDE_RUNTIME")
            .or_else(|| {
                let bundled = exe_dir.map(|dir| dir.join(RUNTIME));
                let dev = dev.join(DEV_RUNTIME);
                [bundled, Some(dev)]
                    .into_iter()
                    .flatten()
                    .find(|path| path.is_file())
            })
            .or_else(find_claude)?;
        Some(Self {
            program,
            runtime,
            waker: None,
        })
    }
}

fn from_env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

impl AgentBackend for Helper {
    fn start(
        &mut self,
        request: &RunRequest,
        space: &Path,
        cwd: &Path,
    ) -> Result<Box<dyn AgentProcess>, AgentError> {
        let start = start_line(request, &RunConfig::default(), space, cwd, &self.runtime);
        process::spawn(
            &self.program,
            &["agent".to_owned()],
            format!("{start}\n"),
            cwd,
            &[(specular_api::SESSION_ENV, request.thread.as_str())],
            self.waker.clone(),
        )
        .map_err(AgentError::Spawn)
    }

    fn ask_models(&mut self) -> Option<Box<dyn AgentProcess>> {
        let ask = json!({
            "type": "start",
            "prompt": "",
            "cwd": std::env::temp_dir(),
            "permissions": "auto",
            "allowedTools": [],
            "appendSystemPrompt": "",
            "runtime": self.runtime,
            "models": true,
        });
        process::spawn(
            &self.program,
            &["agent".to_owned()],
            format!("{ask}\n"),
            &std::env::temp_dir(),
            &[],
            self.waker.clone(),
        )
        .inspect_err(|error| tracing::warn!("the models were not asked for: {error}"))
        .ok()
    }

    fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }
}

/// The helper's first line: the run, with the request's images read from
/// the space folder `space`. An image that is gone is left out.
fn start_line(
    request: &RunRequest,
    config: &RunConfig,
    space: &Path,
    cwd: &Path,
    runtime: &Path,
) -> Value {
    let images: Vec<Value> = (request.images.iter())
        .filter_map(|image| {
            let bytes = std::fs::read(space.join(&image.path)).ok()?;
            Some(json!({ "mediaType": image.media_type.as_str(), "data": STANDARD.encode(bytes) }))
        })
        .collect();
    let permissions = match config.permissions {
        Permissions::Auto => "auto",
        Permissions::AcceptEdits => "acceptEdits",
        Permissions::Dangerously => "dangerously",
    };
    json!({
        "type": "start",
        "prompt": request.prompt,
        "images": images,
        "cwd": cwd,
        "model": request.model_under(config),
        "permissions": permissions,
        "allowedTools": ALLOWED_TOOLS,
        "appendSystemPrompt": SPECULAR_CONTEXT,
        "resume": request.resume,
        "runtime": runtime,
    })
}
