use crate::thread::{Image, ThreadId};

/// Tools pre-approved for the `auto` and `acceptEdits` modes.
pub const ALLOWED_TOOLS: [&str; 14] = [
    "Read",
    "Edit",
    "Write",
    "Grep",
    "Glob",
    "Bash(git status:*)",
    "Bash(git diff:*)",
    "Bash(git log:*)",
    "Bash(pnpm typecheck:*)",
    "Bash(pnpm test:unit:*)",
    "Bash(pnpm lint:*)",
    "Bash(npm run typecheck:*)",
    "Bash(tsc:*)",
    "Bash(specular:*)",
];

/// Appended to the Claude Code system prompt: the app the agent runs in and
/// the skill that drives it.
pub const SPECULAR_CONTEXT: &str =
    "You are running inside Specular — the app the user is looking at right now.
Specular is a spatial canvas of live web pages, notes, files, and drawings,
stored as .canvas documents in the space folder.

The `specular` skill drives that running app. Load it with the Skill tool at
the start of any turn that touches the canvas: reading what is on it, adding
or editing entities, arranging them, or snapshotting and screenshotting a
live page. A turn with no source repo to change is still a canvas turn — the
user is looking at the canvas, so the result belongs there, not only in your
reply.

Do not hand-edit .canvas files while the app is running: it owns that
document and its next save writes over yours. Go through the skill instead.
Do not reach for chrome-devtools or other browser automation — the skill
already has the page the user means.";

/// How `claude` asks before a tool runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permissions {
    /// A classifier approves each call.
    Auto,
    /// Edits go through; the allowlist covers the rest.
    AcceptEdits,
    /// Nothing is checked.
    Dangerously,
}

/// Settings for a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfig {
    /// The model alias.
    pub model: String,
    /// The permission mode.
    pub permissions: Permissions,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            model: "sonnet".into(),
            permissions: Permissions::Auto,
        }
    }
}

/// One run the caller should start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    /// The thread it answers.
    pub thread: ThreadId,
    /// Written to the process's stdin.
    pub prompt: String,
    /// The session to resume.
    pub resume: Option<String>,
    /// The images pasted into this turn, relative to the space folder. They
    /// go to the model beside the prompt.
    pub images: Vec<Image>,
}

/// The arguments after the program name. The prompt goes to stdin.
pub fn claude_args(request: &RunRequest, config: &RunConfig) -> Vec<String> {
    let mut args: Vec<String> = [
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--model",
    ]
    .map(String::from)
    .into();
    args.push(config.model.clone());
    args.push("--permission-mode".into());
    let (mode, allow) = match config.permissions {
        Permissions::Auto => ("auto", true),
        Permissions::AcceptEdits => ("acceptEdits", true),
        Permissions::Dangerously => ("bypassPermissions", false),
    };
    args.push(mode.into());
    if allow {
        args.push("--allowedTools".into());
        args.extend(ALLOWED_TOOLS.map(String::from));
    }
    args.push("--append-system-prompt".into());
    args.push(SPECULAR_CONTEXT.into());
    args.push("--settings".into());
    args.push(r#"{"outputStyle":"Concise"}"#.into());
    if let Some(session) = &request.resume {
        args.push("--resume".into());
        args.push(session.clone());
    }
    args
}
