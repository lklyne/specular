use crate::pill::{Pill, focus_prompt};
use crate::thread::{Role, Thread};

/// Where the turn may change source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteTarget {
    /// The canvas and space folder only.
    Space,
    /// A linked site repo.
    Repo {
        /// The site's origin.
        origin: String,
        /// The repo on disk.
        repo_path: String,
    },
}

/// One line saying what a comment is pinned to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentContext {
    /// The comment.
    pub annotation_id: String,
    /// E.g. `on element "button.cta" of page p1 (http://localhost:3000/)`.
    pub description: String,
}

/// What a prompt needs besides the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptContext {
    /// The space folder.
    pub space_path: String,
    /// Where source may change.
    pub write_target: WriteTarget,
    /// What the user has picked.
    pub pill: Pill,
    /// The canvas's name, shown by the empty pill.
    pub canvas_name: String,
    /// Descriptions of the comments the thread mentions.
    pub comments: Vec<CommentContext>,
}

impl PromptContext {
    fn comment(&self, id: &str) -> Option<String> {
        let found = self.comments.iter().find(|c| c.annotation_id == id)?;
        Some(format!("  (comment {id} {})", found.description))
    }
}

const REPLY_FORMAT: [&str; 6] = [
    "Reply format — REQUIRED:",
    "- Your entire final message is the only thing the user sees, so keep it brief and self-contained — no references to your steps, tool output, or anything \"above\" they cannot see.",
    "- End the message with one of:",
    "  <<RESOLVE>>   if you have addressed the request (made the change or answered it)",
    "  <<WAITING>>   if you need more information from the user",
    "Do not write anything after the marker.",
];

fn focus_lines(ctx: &PromptContext, out: &mut Vec<String>) {
    match focus_prompt(&ctx.pill) {
        None => {
            out.push(format!(
                "Current selection: {}",
                ctx.pill.label(&ctx.canvas_name)
            ));
            out.push(String::new());
        }
        Some(focus) => {
            out.push(focus);
            out.push("Prefer changing those. Touch other canvas items only if the request clearly needs it.".into());
            out.push(String::new());
        }
    }
}

fn reply_format(out: &mut Vec<String>) {
    out.extend(REPLY_FORMAT.iter().map(|l| (*l).to_owned()));
}

/// The full prompt: where to work, what is selected, the whole thread.
pub fn thread_prompt(thread: &Thread, ctx: &PromptContext) -> String {
    let mut lines: Vec<String> = vec![format!(
        "Working directory (space folder): {}",
        ctx.space_path
    )];
    match &ctx.write_target {
        WriteTarget::Repo { origin, repo_path } => {
            lines.push(format!(
                "This turn should change source for {origin} in the repo at {repo_path}."
            ));
            lines.push("Edit that repo. Pages already on the canvas reload from source.".into());
            lines.push("Anything new reaches the user only once it is on the canvas: `specular add page <full url> --at x,y`.".into());
        }
        WriteTarget::Space => {
            lines.push("This turn is about the canvas / space, not a linked site repo.".into());
            lines.push("Use `specular add` / `update` / `delete` / `arrange` to change what the user sees.".into());
            lines.push("Files you write in this folder reach the user once they are on the canvas: `specular add file <path> --at x,y`.".into());
        }
    }
    lines.push(String::new());
    focus_lines(ctx, &mut lines);
    if thread.messages.iter().any(|m| !m.images.is_empty()) {
        lines.push(
            "Images the user pasted this turn are attached; earlier ones are at the listed paths."
                .into(),
        );
    }
    lines.push("Thread:".into());
    for message in &thread.messages {
        let who = if message.role == Role::Agent {
            "Agent"
        } else {
            "User"
        };
        let mut line = format!("[{who}] {}", message.text);
        for image in &message.images {
            line.push_str(" [image: ");
            line.push_str(&image.path);
            line.push(']');
        }
        lines.push(line);
        if message.role == Role::User
            && let Some(note) = message
                .annotation_id
                .as_deref()
                .and_then(|id| ctx.comment(id))
        {
            lines.push(note);
        }
    }
    lines.push(String::new());
    lines.push("Inspecting a live page (when one is in play):".into());
    lines.push("  specular snapshot -i -f <pageId>".into());
    lines.push("  specular get styles @<ref>".into());
    lines.push("  specular screenshot -f <pageId>".into());
    lines.push(String::new());
    reply_format(&mut lines);
    lines.join("\n")
}

/// The prompt for a follow-up turn on a resumed session.
pub fn follow_up_prompt(
    text: &str,
    ctx: &PromptContext,
    image_count: usize,
    annotation_ids_in_turn: &[String],
) -> String {
    let fallback = if image_count > 0 {
        "See the attached image."
    } else {
        "Continue addressing the latest feedback in this thread."
    };
    let message = if text.trim().is_empty() {
        fallback
    } else {
        text.trim()
    };
    let mut lines = vec![
        "The user followed up in the same canvas agent thread:".to_owned(),
        format!("[User] {message}"),
    ];
    lines.extend(
        annotation_ids_in_turn
            .iter()
            .filter_map(|id| ctx.comment(id)),
    );
    match image_count {
        0 => {}
        1 => lines.push("(An image is attached to this message.)".into()),
        n => lines.push(format!("({n} images are attached to this message.)")),
    }
    lines.push(String::new());
    focus_lines(ctx, &mut lines);
    lines.push("Continue the thread — make a change if it calls for one, or just answer if it is a question.".into());
    lines.push("Use the specular skill as before.".into());
    lines.push(String::new());
    reply_format(&mut lines);
    lines.join("\n")
}
