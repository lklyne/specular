use crate::text::truncate;

const RESOLVE_MARKER: &str = "<<RESOLVE>>";
const WAITING_MARKER: &str = "<<WAITING>>";
// A runaway guard only: a concise plan still runs a few thousand characters.
const MAX_REPLY_CHARS: usize = 50_000;

/// What the agent's final message says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// The text the user sees, without the marker.
    pub summary: String,
    /// True when the last marker is `<<RESOLVE>>`.
    pub should_resolve: bool,
}

/// Splits the final message into the answer and its marker. The last marker wins.
pub fn parse_output(stdout: &str) -> Reply {
    let text = stdout.trim();
    if text.is_empty() {
        return Reply {
            summary: "(no output)".into(),
            should_resolve: false,
        };
    }
    let resolve = text.rfind(RESOLVE_MARKER);
    let waiting = text.rfind(WAITING_MARKER);
    let Some(marker) = resolve.max(waiting) else {
        return Reply {
            summary: truncate(text, MAX_REPLY_CHARS),
            should_resolve: false,
        };
    };
    let answer = text[..marker].trim();
    Reply {
        summary: truncate(
            if answer.is_empty() {
                "(no summary)"
            } else {
                answer
            },
            MAX_REPLY_CHARS,
        ),
        should_resolve: resolve > waiting,
    }
}
