//! The Share button's popover (ADR 0050): who the link lets in, and Copy
//! link.

use super::model::{Button, Dropdown, DropdownOption, DropdownSection, Face, OptionLayout};
use super::{ControlId, ShareButton};
use crate::share_button::{Status, is_sharing};
use crate::{Action, App, LinkScope, ShareAction};

/// The choices of who the link lets in, in the list's order.
const ACCESS: [(Option<LinkScope>, &str, &str); 4] = [
    (None, "private", "Only you"),
    (
        Some(LinkScope::View),
        "view",
        "Anyone with the link can view",
    ),
    (
        Some(LinkScope::Comment),
        "comment",
        "Anyone with the link can comment",
    ),
    (
        Some(LinkScope::Edit),
        "edit",
        "Anyone with the link can edit",
    ),
];

pub(super) fn button(app: &App) -> ShareButton {
    let id = ControlId::new("share");
    let canvas = &app.space.active().id;
    let status = app.shares.of(canvas);
    let chosen = is_sharing(app, canvas).then(|| app.shares.access(canvas));
    let access_id = id.child("access");
    let options = (ACCESS.iter())
        .map(|&(scope, name, label)| DropdownOption {
            id: access_id.child(name),
            label: label.into(),
            face: Face::text(label),
            trailing: None,
            fields: Vec::new(),
            chord: None,
            selected: scope == chosen,
            enabled: true,
            action: Action::Share(ShareAction::SetAccess(scope)),
        })
        .collect();
    let summary = (ACCESS.iter())
        .find(|(scope, ..)| *scope == chosen)
        .map_or("Only you", |(_, _, label)| label);
    let publishing = matches!(status, Some(Status::Publishing { .. }));
    ShareButton {
        access: Dropdown {
            id: access_id,
            label: "Who can open the link".into(),
            summary: Face::text(summary),
            content: vec![DropdownSection::Options {
                layout: OptionLayout::List,
                options,
            }],
        },
        copy: Button {
            id: id.child("copy"),
            label: "Copy link".into(),
            face: Face::text("Copy link"),
            enabled: !publishing,
            chord: None,
            action: Action::Share(ShareAction::CopyLink),
        },
        note: match status {
            Some(Status::Failed(reason)) => Some(format!("Not shared: {reason}").into()),
            _ => None,
        },
        id,
    }
}
