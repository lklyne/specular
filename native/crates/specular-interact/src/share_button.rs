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

/// Something done in the Share popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareAction {
    /// Who the active canvas's link lets in. `None` is only you, which
    /// stops sharing it; a scope shares it, publishing it first when it is
    /// not shared yet.
    SetAccess(Option<LinkScope>),
    /// Copy the active canvas's link, publishing it and taking it live
    /// first when it is not shared yet.
    CopyLink,
}

/// Where one canvas's share stands.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Status {
    /// The shell is publishing it; `copy`, for a Copy link, which copies
    /// the link once it is published.
    Publishing {
        /// Whether the link goes on the clipboard once published.
        copy: bool,
    },
    /// Published, with these links. Live once its room answers.
    Published(ShareLinks),
    /// Joined through a link, which is all it knows.
    Joined(String),
    /// Publishing failed, or the room refused the canvas, for this reason.
    Failed(String),
}

/// What a canvas's link lets in until its owner picks otherwise.
pub(crate) const DEFAULT_ACCESS: LinkScope = LinkScope::Edit;

/// Each canvas's share, by canvas, and the access picked for it.
/// Space-wide: a canvas left keeps its own.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shares {
    status: BTreeMap<CanvasId, Status>,
    access: BTreeMap<CanvasId, LinkScope>,
}

impl Shares {
    pub(crate) fn of(&self, canvas: &CanvasId) -> Option<&Status> {
        self.status.get(canvas)
    }

    pub(crate) fn set(&mut self, canvas: CanvasId, status: Status) {
        self.status.insert(canvas, status);
    }

    /// What `canvas`'s link lets in.
    pub(crate) fn access(&self, canvas: &CanvasId) -> LinkScope {
        self.access.get(canvas).copied().unwrap_or(DEFAULT_ACCESS)
    }
}

pub(crate) fn run(app: &mut App, action: ShareAction, effects: &mut Vec<Effect>) {
    let canvas = app.space.active().id.clone();
    match action {
        ShareAction::SetAccess(Some(scope)) => {
            app.shares.access.insert(canvas.clone(), scope);
            publish(app, canvas, false, effects);
        }
        ShareAction::SetAccess(None) => {
            crate::shared::on_event(app, ShareEvent::Stop, effects);
            app.shares.status.remove(&canvas);
        }
        ShareAction::CopyLink => {
            let status = app.shares.of(&canvas);
            if let Some(link) = link(status, app.shares.access(&canvas)) {
                effects.push(Effect::WriteClipboard(link.to_owned()));
            } else {
                publish(app, canvas, true, effects);
            }
        }
    }
}

/// Publishes `canvas` unless it is shared or publishing already.
fn publish(app: &mut App, canvas: CanvasId, copy: bool, effects: &mut Vec<Effect>) {
    match app.shares.of(&canvas) {
        Some(Status::Publishing { .. } | Status::Published(_) | Status::Joined(_)) => {}
        None | Some(Status::Failed(_)) => {
            app.shares.set(canvas.clone(), Status::Publishing { copy });
            let title = app.space.active().name.clone();
            effects.push(Effect::SharePublish { canvas, title });
        }
    }
}

/// Whether `canvas` is shared, or on its way to being shared.
pub(crate) fn is_sharing(app: &App, canvas: &CanvasId) -> bool {
    matches!(
        app.shares.of(canvas),
        Some(Status::Publishing { .. } | Status::Published(_) | Status::Joined(_))
    ) || (*canvas == app.space.active().id && app.is_shared())
}

/// The link of `scope` that `status` knows, if it knows one.
pub(crate) fn link(status: Option<&Status>, scope: LinkScope) -> Option<&str> {
    match (status?, scope) {
        (Status::Published(links), LinkScope::Edit) => Some(&links.edit),
        (Status::Published(links), LinkScope::Comment) => Some(&links.comment),
        (Status::Published(links), LinkScope::View) => Some(&links.view),
        (Status::Joined(link), LinkScope::Edit) => Some(link),
        (Status::Joined(_), LinkScope::Comment | LinkScope::View)
        | (Status::Publishing { .. } | Status::Failed(_), _) => None,
    }
}

/// The shell published `canvas`: the link is copied if a Copy link asked
/// for it, and the canvas goes live through the edit link if it is still
/// the one showing.
pub(crate) fn published(
    app: &mut App,
    canvas: CanvasId,
    links: ShareLinks,
    effects: &mut Vec<Effect>,
) {
    let copy = matches!(
        app.shares.of(&canvas),
        Some(Status::Publishing { copy: true })
    );
    let edit = links.edit.clone();
    let status = Status::Published(links);
    if let Some(link) = link(Some(&status), app.shares.access(&canvas)).filter(|_| copy) {
        effects.push(Effect::WriteClipboard(link.to_owned()));
    }
    app.shares.set(canvas.clone(), status);
    if canvas == app.space.active().id && !app.is_shared() {
        let link = edit;
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
