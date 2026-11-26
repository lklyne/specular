//! [`notices`]: what the app has to tell the person that no panel shows: a
//! page whose process went away, a load that ended in an HTTP error, a
//! share that failed. The `notices` state key, which a toast draws.
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
}

const CRASHED: &str = "crashed";
const HTTP: &str = "http";

/// The notices of `app` now: the pages' in the order of their ids, then the
/// share's.
pub fn notices(app: &App) -> Vec<AppNotice> {
    let mut pages: Vec<_> = app.session.pages.iter().collect();
    pages.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
    let mut out = Vec::new();
    for (id, state) in pages {
        let Some(entity) = app.document.entity(id) else {
            continue;
        };
        let name = labels::page_label(app, entity);
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
            });
        }
        if let Some(status) = state.http_error {
            out.push(AppNotice {
                id: format!("page.{}.{HTTP}", id.as_str()),
                level: "warning",
                text: format!("{name} answered HTTP {status}."),
                action: reload,
                dismiss: true,
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
        });
    }
    out
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
