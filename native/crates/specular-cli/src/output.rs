//! What a verb prints: JSON answers, text, and pictures saved to a temp
//! file whose path an agent can read.

use std::io::Write as _;

use serde_json::Value;

use crate::session;

/// A piece of a browse or annotation answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    /// Printed as it is.
    Text(String),
    /// Saved to a temp file, whose path is printed.
    Image {
        /// The encoded picture.
        data: Vec<u8>,
        /// JPEG, else PNG.
        jpeg: bool,
    },
}

/// Prints `value` as indented JSON.
pub(crate) fn json(value: &Value) {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    text_line(&text);
}

/// Prints `text` and a newline.
pub(crate) fn text_line(text: &str) {
    let mut out = std::io::stdout().lock();
    // A closed pipe is the reader's choice; there is no one to tell.
    let _ = writeln!(out, "{text}");
}

/// Prints `message` as an error on stderr.
pub(crate) fn error(message: &str) {
    let _ = writeln!(std::io::stderr().lock(), "error: {message}");
}

/// Prints each block: text as it is, a picture as the path it was saved to
/// and how to look at it.
pub(crate) fn blocks(blocks: &[Block]) -> anyhow::Result<()> {
    for block in blocks {
        match block {
            Block::Text(text) => text_line(text),
            Block::Image { data, jpeg } => {
                let extension = if *jpeg { "jpg" } else { "png" };
                let path =
                    std::env::temp_dir().join(format!("specular-{}.{extension}", session::uuid()));
                std::fs::write(&path, data)?;
                let path = path.display();
                text_line(&path.to_string());
                text_line(&format!("(image: use Read(\"{path}\") to view)"));
            }
        }
    }
    Ok(())
}
