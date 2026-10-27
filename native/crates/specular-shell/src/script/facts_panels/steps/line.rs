//! Reading a panel step from its line.

use anyhow::{Context as _, bail};
use specular_core::Cursor;
use specular_interact::{
    Artifact, ArtifactTarget, NoteNotice, Notice, PageNotice, Progress, ProgressKind,
};

use super::PanelStep;
use crate::script::words::unescaped;

/// The step `verb` and `rest` name, when it is one of these.
pub(crate) fn parse(verb: &str, rest: &str) -> Option<anyhow::Result<PanelStep>> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let step = match (verb, words.as_slice()) {
        ("chat-panel", ["on"]) => Ok(PanelStep::ChatPanel(true)),
        ("chat-panel", ["off"]) => Ok(PanelStep::ChatPanel(false)),
        ("send-chat", _) => Ok(PanelStep::SendChat(unescaped(rest), false)),
        ("send-image", _) => Ok(PanelStep::SendChat(unescaped(rest), true)),
        ("chat", [button, numbers @ ..]) => numbers
            .iter()
            .map(|number| number.parse().context("`chat` takes whole numbers"))
            .collect::<anyhow::Result<_>>()
            .map(|numbers| PanelStep::Chat((*button).to_owned(), numbers)),
        ("agent-says", [kind, ..]) => notice(kind, after_first(rest)).map(PanelStep::AgentSays),
        ("page-reports", [page, kind, ..]) => page_notice(kind, &words[2..])
            .map(|notice| PanelStep::PageReports((*page).to_owned(), notice)),
        ("bind", [origin, path]) => Ok(PanelStep::Bind {
            origin: (*origin).to_owned(),
            path: (*path).to_owned(),
            auto: false,
        }),
        ("bind", [origin, path, "auto"]) => Ok(PanelStep::Bind {
            origin: (*origin).to_owned(),
            path: (*path).to_owned(),
            auto: true,
        }),
        ("note-text", [file, ..]) => Ok(PanelStep::Note(
            unescaped(file),
            NoteNotice::Text(unescaped(after_first(rest))),
        )),
        ("note-missing", [file]) => Ok(PanelStep::Note(unescaped(file), NoteNotice::Missing)),
        ("note-failed", [file]) => Ok(PanelStep::Note(unescaped(file), NoteNotice::Failed)),
        ("note-refused", [file, ..]) => refused(file, after_first(rest)),
        ("threads-loaded", pairs) => pairs
            .iter()
            .map(|pair| {
                let (id, tab) = pair.split_once('@').context("a thread is `ID@CANVAS`")?;
                Ok((id.to_owned(), tab.to_owned()))
            })
            .collect::<anyhow::Result<_>>()
            .map(PanelStep::ThreadsLoaded),
        ("models", _) => rest
            .split('|')
            .map(|pair| {
                let (value, name) = pair
                    .trim()
                    .split_once('=')
                    .context("a model is `VALUE=NAME`")?;
                Ok((value.to_owned(), name.to_owned()))
            })
            .collect::<anyhow::Result<_>>()
            .map(PanelStep::Models),
        ("model-saved", [value]) => Ok(PanelStep::ModelSaved((*value).to_owned())),
        ("ime-cancel", []) => Ok(PanelStep::ImeCancel),
        ("space-needed", words) => space_needed(words),
        ("first-run-choice", [name]) => Ok(PanelStep::FirstRunChoice((*name).to_owned())),
        ("settings-loaded", words) => Ok(PanelStep::SettingsLoaded {
            sidebar: words.contains(&"sidebar"),
            chat: words.contains(&"chat"),
        }),
        ("about", [name, version]) => {
            Ok(PanelStep::About((*name).to_owned(), (*version).to_owned()))
        }
        ("prepare-tab", ["none"]) => Ok(PanelStep::PrepareTab(None)),
        ("prepare-tab", [item]) => Ok(PanelStep::PrepareTab(Some((*item).to_owned()))),
        (
            "chat-panel" | "chat" | "agent-says" | "page-reports" | "bind" | "note-text"
            | "model-saved",
            _,
        ) => Err(anyhow::anyhow!("the wrong arguments")),
        _ => return None,
    };
    Some(step)
}

fn space_needed(words: &[&str]) -> anyhow::Result<PanelStep> {
    let mut missing = None;
    let mut electron = None;
    for pair in words.chunks(2) {
        match pair {
            ["missing", path] => missing = Some(unescaped(path)),
            ["electron", path] => electron = Some(unescaped(path)),
            _ => bail!("`space-needed [missing PATH] [electron PATH]`"),
        }
    }
    Ok(PanelStep::SpaceNeeded { missing, electron })
}

fn refused(file: &str, texts: &str) -> anyhow::Result<PanelStep> {
    let (disk, ours) = texts
        .split_once('|')
        .context("`note-refused FILE DISK | OURS`")?;
    let notice = NoteNotice::Refused {
        disk: unescaped(disk.trim()),
        ours: unescaped(ours.trim()),
    };
    Ok(PanelStep::Note(unescaped(file), notice))
}

/// `text` after its first word.
fn after_first(text: &str) -> &str {
    let text = text.trim_start();
    text.split_once(char::is_whitespace)
        .map_or("", |(_, rest)| rest.trim_start())
}

fn notice(kind: &str, text: &str) -> anyhow::Result<Notice> {
    let text = unescaped(text);
    Ok(match kind {
        "session" => Notice::Session(text),
        "text" => Notice::Text(text),
        "delta" => Notice::TextDelta(text),
        "progress" => Notice::Progress(Progress {
            kind: ProgressKind::ToolUse,
            text: format!("{text} the page"),
            label: Some(text),
        }),
        "finished" => Notice::Finished { text },
        "failed" => Notice::Failed { error: text },
        "cancelled" => Notice::Cancelled,
        "made" => {
            let (path, how) = text.rsplit_once(' ').context("`made PATH new|edited`")?;
            Notice::Made(Artifact {
                target: ArtifactTarget::File(path.to_owned()),
                created: how == "new",
            })
        }
        other => bail!(
            "the agent says session, text, delta, progress, finished, failed, cancelled or made, not `{other}`"
        ),
    })
}

fn page_notice(kind: &str, words: &[&str]) -> anyhow::Result<PageNotice> {
    let number = |word: &&str| -> anyhow::Result<f64> {
        word.parse()
            .with_context(|| format!("`{word}` is not a number"))
    };
    Ok(match (kind, words) {
        ("title", _) => PageNotice::Title(unescaped(&words.join(" "))),
        ("url", [url]) => PageNotice::Url((*url).to_owned()),
        ("loading", [on, rest @ ..]) => PageNotice::Loading {
            loading: *on == "on",
            can_go_back: rest.contains(&"back"),
            can_go_forward: rest.contains(&"forward"),
        },
        ("scrolled", [x, y]) => PageNotice::Scrolled {
            x: number(x)?,
            y: number(y)?,
        },
        ("scroll-progress", [x, y]) => PageNotice::ScrollProgress {
            x: number(x)?,
            y: number(y)?,
        },
        ("loaded", [status]) => PageNotice::Loaded {
            http_status: status.parse().context("`loaded` takes a status")?,
        },
        ("devtools", [url]) => PageNotice::DevtoolsUrl((*url).to_owned()),
        ("favicon", []) => PageNotice::Favicon(Some(vec![0x89, b'P', b'N', b'G'].into())),
        ("cursor", ["default"]) => PageNotice::Cursor(Cursor::Default),
        ("cursor", ["pointer"]) => PageNotice::Cursor(Cursor::Pointer),
        ("cursor", ["text"]) => PageNotice::Cursor(Cursor::Text),
        _ => bail!("a page reports title, url, loading, scrolled, loaded, devtools or cursor"),
    })
}
