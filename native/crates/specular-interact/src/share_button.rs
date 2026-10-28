//! The Share button's side of sharing (ADR 0050): publishing the active
//! canvas, the links it was published with, and taking it live through its
//! edit link. The publishing itself is the shell's: it hears
//! [`Effect::SharePublish`] and answers with
//! [`ShareEvent::Published`](crate::ShareEvent::Published) or
//! [`ShareEvent::PublishFailed`](crate::ShareEvent::PublishFailed).

use std::collections::BTreeMap;

use crate::space::CanvasId;
use crate::{App, Effect, ShareEvent};

/// The three links a published canvas opens with, one per scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareLinks {
    /// Opens the canvas to edit, in the web viewer or another app.
    pub edit: String,
    /// Opens it to read and comment on.
    pub comment: String,
    /// Opens it to read.
    pub view: String,
}

/// What a share link lets its holder do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkScope {
    /// Edit.
    Edit,
    /// Read and comment.
    Comment,
    /// Read.
    View,
}

/// Something done with the Share button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareAction {
    /// Publish the active canvas and take it live. Does nothing while it
    /// is shared or publishing.
    Publish,
    /// Put the active canvas's link of this scope on the clipboard.
    CopyLink(LinkScope),
    /// Stop sharing the active canvas. Its document stays as it is, and the
    /// server keeps its links: a later Publish brings the same doc back.
    Stop,
}

/// Where one canvas's share stands.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Status {
    /// The shell is publishing it.
    Publishing,
    /// Published, with these links. Live once its room answers.
    Published(ShareLinks),
    /// Joined through a link, which is all it knows.
    Joined(String),
    /// Publishing failed, or the room refused the canvas, for this reason.
    Failed(String),
}

/// Each canvas's share, by canvas. Space-wide: a canvas left keeps its own.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shares(BTreeMap<CanvasId, Status>);

impl Shares {
    pub(crate) fn of(&self, canvas: &CanvasId) -> Option<&Status> {
        self.0.get(canvas)
    }

    pub(crate) fn set(&mut self, canvas: CanvasId, status: Status) {
        self.0.insert(canvas, status);
    }
}

pub(crate) fn run(app: &mut App, action: ShareAction, effects: &mut Vec<Effect>) {
    let canvas = app.space.active().id.clone();
    match action {
        ShareAction::Publish => {
            let busy = matches!(app.shares.of(&canvas), Some(Status::Publishing));
            if app.is_shared() || busy {
                return;
            }
            app.shares.set(canvas.clone(), Status::Publishing);
            let title = app.space.active().name.clone();
            effects.push(Effect::SharePublish { canvas, title });
        }
        ShareAction::CopyLink(scope) => {
            if let Some(link) = link(app.shares.of(&canvas), scope) {
                effects.push(Effect::WriteClipboard(link.to_owned()));
            }
        }
        ShareAction::Stop => {
            crate::shared::on_event(app, ShareEvent::Stop, effects);
            app.shares.0.remove(&canvas);
        }
    }
}

/// The link of `scope` that `status` knows, if it knows one.
pub(crate) fn link(status: Option<&Status>, scope: LinkScope) -> Option<&str> {
    match (status?, scope) {
        (Status::Published(links), LinkScope::Edit) => Some(&links.edit),
        (Status::Published(links), LinkScope::Comment) => Some(&links.comment),
        (Status::Published(links), LinkScope::View) => Some(&links.view),
        (Status::Joined(link), LinkScope::Edit) => Some(link),
        (Status::Joined(_), LinkScope::Comment | LinkScope::View)
        | (Status::Publishing | Status::Failed(_), _) => None,
    }
}

/// The shell published `canvas`: it goes live through the edit link, if it
/// is still the one showing.
pub(crate) fn published(
    app: &mut App,
    canvas: CanvasId,
    links: ShareLinks,
    effects: &mut Vec<Effect>,
) {
    let link = links.edit.clone();
    app.shares.set(canvas.clone(), Status::Published(links));
    if canvas == app.space.active().id && !app.is_shared() {
        effects.push(Effect::ShareConnect { canvas, link });
    }
}

/// The room refused `canvas` for good: it is no longer shared.
pub(crate) fn ended(app: &mut App, canvas: &CanvasId, reason: String) {
    if *canvas == app.space.active().id {
        app.history.unshare();
    } else if let Some(parked) = app.space.parked_mut(canvas) {
        parked.history.unshare();
    }
    app.shares.set(canvas.clone(), Status::Failed(reason));
}
