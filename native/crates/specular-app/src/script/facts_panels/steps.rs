//! The steps that play the shell's and the agent's side: the right panel's
//! buttons, the agent's stream, a page's reports, the repo bindings, a
//! Document's file and the threads read from disk.
//!
//! ```text
//! chat-panel on                        # the shell draws a right panel
//! send-chat make it blue               # Send, with this typed (none: empty)
//! send-image see this                  # the same with a small PNG pasted
//! chat new-thread                      # back, close, stop, toggle, drop-draft,
//!                                      # resolve, open N, model N, card N M,
//!                                      # bubble N, auto, resize W
//! agent-says session sess-1            # text T, delta T, progress LABEL,
//!                                      # finished T, failed T, cancelled,
//!                                      # made PATH new|edited
//! page-reports p1 title Home           # url U, loading on|off back forward,
//!                                      # scrolled X Y, loaded STATUS, devtools U
//! bind https://example.com /scratch/site auto
//! note-text plan.md # Plan\n\nBody     # a Document's file says this; also
//!                                      # note-missing F, note-failed F,
//!                                      # note-refused F DISK | OURS; `\s` is
//!                                      # a space in a file's name
//! threads-loaded t1@tab_1 t2@tab_2     # the threads on disk, each canvas's open
//! models default=Default|sonnet=Sonnet 5
//! model-saved default
//! ime-cancel                           # the input method drops its composition
//! ```

use anyhow::{Context as _, bail};
use specular_interact::{
    Action, App, Artifact, ArtifactTarget, AssetBytes, ChatAction, Event, ImageUpload, MediaType,
    Model, ModelsNews, NoteNotice, Notice, PageNotice, Progress, ProgressKind, Thread, ThreadId,
    ThreadIndex, chat,
};

use crate::script::words::unescaped;

/// One of these steps.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelStep {
    /// Whether the shell draws a right panel.
    ChatPanel(bool),
    /// Send pressed with this typed in the composer, and a small PNG
    /// pasted when the flag is on.
    SendChat(String, bool),
    /// A button of the right panel, by its word and any number after it.
    Chat(String, Vec<usize>),
    /// The open thread's agent says this.
    AgentSays(Notice),
    /// A page says something about itself.
    PageReports(String, PageNotice),
    /// An origin bound to a repo folder, with auto-fix on or off.
    Bind {
        /// The site's origin.
        origin: String,
        /// The repo folder.
        path: String,
        /// Whether a comment there is sent the moment it is placed.
        auto: bool,
    },
    /// What the shell read of a Document's markdown file.
    Note(String, NoteNotice),
    /// The threads read from disk: each id and the canvas it is open on.
    ThreadsLoaded(Vec<(String, String)>),
    /// The models the runtime lists.
    Models(Vec<(String, String)>),
    /// The model chosen at an earlier launch.
    ModelSaved(String),
    /// The input method drops its composition.
    ImeCancel,
}

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
        (
            "chat-panel" | "chat" | "agent-says" | "page-reports" | "bind" | "note-text"
            | "model-saved",
            _,
        ) => Err(anyhow::anyhow!("the wrong arguments")),
        _ => return None,
    };
    Some(step)
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
        ("loaded", [status]) => PageNotice::Loaded {
            http_status: status.parse().context("`loaded` takes a status")?,
        },
        ("devtools", [url]) => PageNotice::DevtoolsUrl((*url).to_owned()),
        _ => bail!("a page reports title, url, loading, scrolled, loaded or devtools"),
    })
}

impl PanelStep {
    /// The event the shell sends for this step, read against `app` as it
    /// is now: which thread is open, which button the model holds.
    pub fn event(&self, app: &App) -> anyhow::Result<Event> {
        Ok(match self {
            Self::ChatPanel(on) => Event::ChatPanel(*on),
            Self::SendChat(text, image) => Event::Action(Action::Chat(ChatAction::Send {
                text: text.clone(),
                images: (image.then(|| ImageUpload {
                    media_type: MediaType::Png,
                    bytes: AssetBytes::from(vec![1, 2, 3]),
                }))
                .into_iter()
                .collect(),
            })),
            Self::Chat(button, numbers) => Event::Action(chat_button(app, button, numbers)?),
            Self::AgentSays(notice) => {
                let tab = app.space().active().id.as_str();
                let thread = (app.threads().active(tab))
                    .map(|thread| thread.id.clone())
                    .context("no thread is open for the agent to talk in")?;
                Event::Agent {
                    thread,
                    notice: notice.clone(),
                }
            }
            Self::PageReports(page, notice) => Event::Page {
                page: page.as_str().into(),
                notice: notice.clone(),
            },
            Self::Bind { origin, path, auto } => {
                let mut repos = app.repos().clone();
                repos.bind(origin, path);
                repos.set_auto_fix(origin, *auto);
                Event::ReposLoaded(Box::new(repos))
            }
            Self::Note(file, notice) => Event::Note {
                file: file.clone(),
                notice: notice.clone(),
            },
            Self::ThreadsLoaded(pairs) => threads_loaded(pairs),
            Self::Models(listed) => Event::Models(ModelsNews::Listed(
                (listed.iter())
                    .map(|(value, name)| Model {
                        value: value.clone(),
                        name: name.clone(),
                        description: String::new(),
                    })
                    .collect(),
            )),
            Self::ModelSaved(value) => Event::Models(ModelsNews::Saved(value.clone())),
            Self::ImeCancel => Event::Ime(specular_core::ImeEvent::Cancel),
        })
    }
}

/// The action of the right panel's button `button`, from its model now.
fn chat_button(app: &App, button: &str, numbers: &[usize]) -> anyhow::Result<Action> {
    let model = chat(app);
    let nth = |index: usize| -> anyhow::Result<usize> {
        let place = numbers.get(index).copied().context("which one, from 1?")?;
        place.checked_sub(1).context("places count from 1")
    };
    let transcript = || model.transcript.as_ref().context("no thread is open");
    let missing = || anyhow::anyhow!("the panel has no `{button}` now");
    Ok(match button {
        "new-thread" => model.new_thread.clone(),
        "back" => model.back.clone().ok_or_else(missing)?,
        "close" => model.close.clone().ok_or_else(missing)?,
        "toggle" => model.toggle.clone(),
        "stop" => (transcript()?.run.as_ref())
            .map(|run| run.stop.clone())
            .ok_or_else(missing)?,
        "drop-draft" => (model.composer.draft.as_ref())
            .map(|draft| draft.remove.clone())
            .ok_or_else(missing)?,
        "resolve" => (model.composer.open_comments.as_ref())
            .map(|open| open.resolve.clone())
            .ok_or_else(missing)?,
        "auto" => (model.composer.auto.as_ref())
            .map(|auto| auto.toggle.clone())
            .ok_or_else(missing)?,
        "open" => (model.threads.get(nth(0)?))
            .map(|row| row.select.clone())
            .ok_or_else(missing)?,
        "model" => (model.composer.model.options.get(nth(0)?))
            .map(|option| option.choose.clone())
            .ok_or_else(missing)?,
        "bubble" => (transcript()?.messages.get(nth(0)?))
            .and_then(|bubble| bubble.focus.clone())
            .ok_or_else(missing)?,
        "card" => (transcript()?.messages.get(nth(0)?))
            .and_then(|bubble| bubble.artifacts.get(nth(1).ok()?))
            .and_then(|card| card.focus.clone())
            .ok_or_else(missing)?,
        "resize" => Action::Chat(ChatAction::Resize(
            numbers.first().copied().ok_or_else(missing)? as f32,
        )),
        _ => bail!(
            "the panel's buttons are new-thread, back, close, toggle, stop, drop-draft, resolve, auto, open, model, bubble, card and resize"
        ),
    })
}

fn threads_loaded(pairs: &[(String, String)]) -> Event {
    let now = "2026-01-01T00:00:00.000Z";
    let threads = (pairs.iter())
        .filter_map(|(id, tab)| {
            let json = serde_json::json!({
                "id": id,
                "tabId": tab,
                "title": id,
                "status": "open",
                "createdAt": now,
                "updatedAt": now,
                "annotationIds": [],
                "messages": [],
            });
            Thread::from_json(&json.to_string(), tab, now)
        })
        .collect();
    let index = ThreadIndex {
        active: None,
        by_canvas: (pairs.iter())
            .map(|(id, tab)| (tab.clone(), ThreadId(id.clone())))
            .collect(),
    };
    Event::ThreadsLoaded { threads, index }
}
