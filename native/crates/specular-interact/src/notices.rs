//! [`notices`]: what the app has to tell the person that no panel shows: a
//! page whose process went away, a load that ended in an HTTP error, a
//! share that failed, a canvas that needs mods that are not installed. The
//! `notices` state key, which a toast draws.
//!
//! A page's notice goes when a load starts on it, or when it is dismissed
//! (`notice.dismiss`); a share's goes when the next share works.

use serde::Serialize;
use serde_json::{Value, json};

use crate::share_button::Status;
use crate::{App, labels};

/// A command a notice offers, as `run(name, args)` takes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoticeAction {
    /// The button's text.
    pub label: &'static str,
    /// The command it runs.
    pub run: &'static str,
    /// The command's arguments.
    pub args: Value,
}

/// One thing to tell the person.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppNotice {
    /// Its name, which `notice.dismiss` takes.
    pub id: String,
    /// `info`, `warning` or `error`.
    pub level: &'static str,
    /// What it says.
    pub text: String,
    /// The command it offers, if it offers one.
    pub action: Option<NoticeAction>,
    /// Whether `notice.dismiss` takes it away.
    pub dismiss: bool,
    /// The entity it is about, which `view.reveal` takes, if it is about one.
    pub about: Option<String>,
}

const CRASHED: &str = "crashed";
const HTTP: &str = "http";

/// The notices of `app` now: the canvas's mods to install, the pages' in
/// the order of their ids, then the share's.
pub fn notices(app: &App) -> Vec<AppNotice> {
    let mut pages: Vec<_> = app.session.pages.iter().collect();
    pages.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    let mut out: Vec<AppNotice> = crate::mods::install_notice(app).into_iter().collect();
    for (id, state) in pages {
        let Some(entity) = app.document.entity(id) else {
            continue;
        };
        let name = page_name(app, entity, state.url.as_deref());
        let reload = state.url.as_ref().map(|url| NoticeAction {
            label: "Reload",
            run: "page.navigate",
            args: json!({ "page": id.as_str(), "url": url }),
        });
        if let Some(reason) = &state.crashed {
            out.push(AppNotice {
                id: format!("page.{}.{CRASHED}", id.as_str()),
                level: "error",
                text: format!("{name} stopped ({reason})."),
                action: reload.clone(),
                dismiss: true,
                about: Some(id.as_str().to_owned()),
            });
        }
        if let Some(status) = state.http_error {
            out.push(AppNotice {
                id: format!("page.{}.{HTTP}", id.as_str()),
                level: "warning",
                text: format!("{name} answered HTTP {status}."),
                action: reload,
                dismiss: true,
                about: Some(id.as_str().to_owned()),
            });
        }
    }
    let canvas = &app.space.active().id;
    if let Some(Status::Failed(reason)) = app.shares.of(canvas) {
        out.push(AppNotice {
            id: "share.failed".to_owned(),
            level: "error",
            text: format!("Not shared: {reason}"),
            action: Some(NoticeAction {
                label: "Try again",
                run: "share.copy-link",
                args: Value::Null,
            }),
            dismiss: false,
            about: None,
        });
    }
    out
}

/// A page as a notice names it: its label in quotes, and the address it is
/// at, so two pages with one title are told apart.
fn page_name(app: &App, entity: &specular_doc::Entity, url: Option<&str>) -> String {
    let label = labels::page_label(app, entity);
    let Some(url) = url else {
        return format!("“{label}”");
    };
    let bare = url.split_once("://").map_or(url, |(_, rest)| rest);
    let bare = bare.strip_suffix('/').unwrap_or(bare);
    format!("“{label}” ({bare})")
}

/// Takes the page notice `id` away. Returns whether there was one.
pub(crate) fn dismiss(app: &mut App, id: &str) -> bool {
    let Some(rest) = id.strip_prefix("page.") else {
        return false;
    };
    let Some((page, kind)) = rest.rsplit_once('.') else {
        return false;
    };
    let Some(state) = app.session.pages.get_mut(&page.into()) else {
        return false;
    };
    match kind {
        CRASHED => state.crashed.take().is_some(),
        HTTP => state.http_error.take().is_some(),
        _ => false,
    }
}
