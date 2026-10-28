//! The Share button: Share while the canvas is not shared, Live and its
//! list of links while it is (ADR 0050).

use super::model::{Dropdown, DropdownOption, DropdownSection, Face, OptionLayout};
use super::{ControlId, ShareButton, ShareState};
use crate::share_button::{Status, link};
use crate::{Action, App, LinkScope, ShareAction};

/// The links the list offers, in its order, with what each is called.
const SCOPES: [(LinkScope, &str, &str); 3] = [
    (LinkScope::Edit, "edit", "Can edit"),
    (LinkScope::Comment, "comment", "Can comment"),
    (LinkScope::View, "view", "View only"),
];

pub(super) fn button(app: &App) -> ShareButton {
    let id = ControlId::new("share");
    let status = app.shares.of(&app.space.active().id);
    let publish = Some(Action::Share(ShareAction::Publish));
    let (state, label, tip, action) = match status {
        _ if app.is_shared() => (ShareState::Live, "Live", "Shared: copy a link", None),
        Some(Status::Publishing | Status::Published(_) | Status::Joined(_)) => (
            ShareState::Starting,
            "Sharing…",
            "Connecting to the share server",
            None,
        ),
        Some(Status::Failed(reason)) => {
            let tip = format!("Not shared: {reason}. Press to try again.");
            return ShareButton {
                id,
                state: ShareState::Failed,
                label: "Share".into(),
                tip: tip.into(),
                action: publish,
                menu: None,
            };
        }
        None => (ShareState::Off, "Share", "Share this canvas", publish),
    };
    let menu = (state == ShareState::Live).then(|| menu(&id, status));
    ShareButton {
        id,
        state,
        label: label.into(),
        tip: tip.into(),
        action,
        menu,
    }
}

fn menu(id: &ControlId, status: Option<&Status>) -> Dropdown {
    let row =
        |id: ControlId, face: &str, trailing: Option<&str>, action: ShareAction| DropdownOption {
            id,
            label: face.to_owned().into(),
            face: Face::text(face.to_owned()),
            trailing: trailing.map(|text| text.to_owned().into()),
            fields: Vec::new(),
            chord: None,
            selected: false,
            enabled: true,
            action: Action::Share(action),
        };
    let links = (SCOPES.iter())
        .filter(|(scope, ..)| link(status, *scope).is_some())
        .map(|&(scope, name, label)| {
            row(
                id.child(name),
                label,
                Some("Copy link"),
                ShareAction::CopyLink(scope),
            )
        })
        .collect::<Vec<_>>();
    let stop = row(id.child("stop"), "Stop sharing", None, ShareAction::Stop);
    let mut content = Vec::new();
    if !links.is_empty() {
        content.push(DropdownSection::Options {
            layout: OptionLayout::List,
            options: links,
        });
    }
    content.push(DropdownSection::Options {
        layout: OptionLayout::List,
        options: vec![stop],
    });
    Dropdown {
        id: id.clone(),
        label: "Share".into(),
        summary: Face::text("Live"),
        content,
    }
}
