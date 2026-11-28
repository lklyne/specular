//! The Page menu's commands and the tab row's.

use serde_json::Value;
use specular_core::PageNav;

use super::args::{act, can, choice, entity, idle, maybe_entity, switch, text};
use super::{CommandSpec, Run, RunError};
use crate::{Action, ApiRun, Lens, Orientation, PageStep, Property, Showing};

fn navigate(args: &Value) -> Result<Run, RunError> {
    let url = text(args, "url")?;
    match maybe_entity(args, "page") {
        Some(page) => Ok(Run::Api(ApiRun::Navigate {
            page,
            nav: PageNav::To(url),
        })),
        None => act(Action::PageNavigate(url)),
    }
}

fn orientation(args: &Value) -> Result<Run, RunError> {
    let orientation = choice(
        args,
        "orientation",
        &[
            ("portrait", Orientation::Portrait),
            ("landscape", Orientation::Landscape),
        ],
    )?;
    act(Action::SetProperty(Property::Orientation(orientation)))
}

/// The page in the dock, or the one entered.
pub(super) const PAGE: &[CommandSpec] = &[
    CommandSpec {
        name: "page.new-tab",
        label: "New page tab",
        args: "",
        about: "Make a page in a free spot, show its tab and put the caret in its address.",
        enabled: idle,
        build: |_| act(Action::NewPageTab),
    },
    CommandSpec {
        name: "page.edit-address",
        label: "Edit address",
        args: "",
        about: "Put the caret in the address of the page the dock holds.",
        enabled: |app| can(app, &Action::EditPageUrl),
        build: |_| act(Action::EditPageUrl),
    },
    CommandSpec {
        name: "page.back",
        label: "Back",
        args: "",
        about: "Take the entered or selected page an entry back in its history.",
        enabled: |app| can(app, &Action::PageBack),
        build: |_| act(Action::PageBack),
    },
    CommandSpec {
        name: "page.forward",
        label: "Forward",
        args: "",
        about: "Take the entered or selected page an entry forward in its history.",
        enabled: |app| can(app, &Action::PageForward),
        build: |_| act(Action::PageForward),
    },
    CommandSpec {
        name: "page.reload",
        label: "Reload",
        args: "",
        about: "Load the entered or selected page's address again.",
        enabled: |app| can(app, &Action::PageReload),
        build: |_| act(Action::PageReload),
    },
    CommandSpec {
        name: "page.stop",
        label: "Stop",
        args: "",
        about: "Abandon the entered or selected page's load in flight.",
        enabled: |app| can(app, &Action::PageStop),
        build: |_| act(Action::PageStop),
    },
    CommandSpec {
        name: "page.navigate",
        label: "",
        args: "{url: address, page?: entity id}",
        about: "Take a page, the one in the dock by default, to an address.",
        enabled: idle,
        build: navigate,
    },
    CommandSpec {
        name: "page.frame",
        label: "Device frame",
        args: "",
        about: "Move the selected pages' device frame to its next style.",
        enabled: |app| can(app, &Action::StepPage(PageStep::Frame)),
        build: |_| act(Action::StepPage(PageStep::Frame)),
    },
    CommandSpec {
        name: "page.frame-color",
        label: "Device color",
        args: "",
        about: "Move the selected pages' device colour to the other one.",
        enabled: |app| can(app, &Action::StepPage(PageStep::FrameColor)),
        build: |_| act(Action::StepPage(PageStep::FrameColor)),
    },
    CommandSpec {
        name: "page.rotate",
        label: "Rotate",
        args: "",
        about: "Turn the selected pages between portrait and landscape.",
        enabled: |app| can(app, &Action::StepPage(PageStep::Rotate)),
        build: |_| act(Action::StepPage(PageStep::Rotate)),
    },
    CommandSpec {
        name: "page.scheme",
        label: "Color scheme",
        args: "",
        about: "Show the selected pages in the color scheme they are not shown in.",
        enabled: |app| can(app, &Action::StepPage(PageStep::Scheme)),
        build: |_| act(Action::StepPage(PageStep::Scheme)),
    },
    CommandSpec {
        name: "page.orientation",
        label: "",
        args: "{orientation: portrait|landscape}",
        about: "Set the selected pages' orientation.",
        enabled: idle,
        build: orientation,
    },
    CommandSpec {
        name: "page.toggle-sync",
        label: "",
        args: "",
        about: "Put the selected pages in one sync set, or take them out of theirs.",
        enabled: |app| can(app, &Action::ToggleSync),
        build: |_| act(Action::ToggleSync),
    },
];

/// The tab row: what is shown and through which lens.
pub(super) const TAB: &[CommandSpec] = &[
    CommandSpec {
        name: "tab.show",
        label: "",
        args: "{item?: entity id}",
        about: "Show one page or Document in its tab, or with no item the canvas.",
        enabled: idle,
        build: |args| {
            let showing = maybe_entity(args, "item").map_or(Showing::Canvas, Showing::Item);
            act(Action::Show(showing))
        },
    },
    CommandSpec {
        name: "tab.open",
        label: "",
        args: "{item: entity id}",
        about: "Open a page or Document in a tab from the canvas.",
        enabled: idle,
        build: |args| act(Action::OpenTab(entity(args, "item")?)),
    },
    CommandSpec {
        name: "tab.close",
        label: "Close tab",
        args: "{item?: entity id}",
        about: "Close an item's tab, the one showing by default; the item stays on the canvas.",
        enabled: idle,
        build: |args| act(Action::CloseTab(maybe_entity(args, "item"))),
    },
    CommandSpec {
        name: "tab.next",
        label: "Show next tab",
        args: "",
        about: "Show the tab after the one showing, going round to the first.",
        enabled: idle,
        build: |_| act(Action::ShowNext),
    },
    CommandSpec {
        name: "tab.previous",
        label: "Show previous tab",
        args: "",
        about: "Show the tab before the one showing, going round to the last.",
        enabled: idle,
        build: |_| act(Action::ShowPrevious),
    },
    CommandSpec {
        name: "tab.lens",
        label: "",
        args: "{lens: fill|device}",
        about: "Look at the item of the tab showing through this lens.",
        enabled: idle,
        build: |args| {
            let lens = choice(args, "lens", &Lens::ALL.map(|lens| (lens.key(), lens)))?;
            act(Action::SetLens(lens))
        },
    },
    CommandSpec {
        name: "tab.others",
        label: "",
        args: "{shown: bool}",
        about: "Whether Device draws what is around its item.",
        enabled: idle,
        build: |args| act(Action::ShowOthers(switch(args, "shown")?)),
    },
];
