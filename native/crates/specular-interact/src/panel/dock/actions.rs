//! The buttons the dock ends with for every item (`EntityActions` and
//! `ArrangeButtons` in `CanvasItemPopup.tsx`): arrange a selection of
//! several and annotate it. Pages have their sync toggle among them
//! (`PagePopup.tsx`). Electron's focus button is not here: a tab is how one
//! item is brought into attention (ADR 0045).

use super::super::build::{button, toggle};
use super::super::{Control, ControlId, Face, Icon};
use crate::{Action, ArrangeMode};

/// Which of the buttons the dock has. Arranging needs two or more items.
#[derive(Debug, Clone, Copy)]
pub(super) struct Actions<'a> {
    /// What the buttons are about: "shape", "3 pages", "2 items".
    pub(super) noun: &'a str,
    /// How many items are selected.
    pub(super) count: usize,
    /// The sync toggle of pages, and whether it is on: the selection is one
    /// sync set, which pressing it takes apart.
    pub(super) sync: Option<bool>,
    /// Whether to offer annotating.
    pub(super) annotate: bool,
}

impl<'a> Actions<'a> {
    /// Arranging and annotating, as the dock has them for one kind of item
    /// them.
    pub(super) const fn all(noun: &'a str, count: usize) -> Self {
        Self {
            noun,
            count,
            sync: None,
            annotate: true,
        }
    }

    /// The buttons, in the order the Electron popups have them: arrange,
    /// sync, annotate.
    pub(super) fn controls(self) -> Vec<Control> {
        let mut controls = Vec::new();
        if self.count >= 2 {
            let modes = [
                (ArrangeMode::Row, "row", Icon::ArrangeRow),
                (ArrangeMode::Column, "column", Icon::ArrangeColumn),
                (ArrangeMode::Grid, "grid", Icon::ArrangeGrid),
            ];
            controls.extend(modes.into_iter().map(|(mode, name, icon)| {
                button(
                    ControlId::new("item.arrange").child(name),
                    format!("Arrange in a {name}"),
                    Face::icon(icon),
                    Action::Arrange(mode),
                )
            }));
        }
        if let Some(synced) = self.sync {
            let label = if synced {
                "Unsync navigation"
            } else {
                "Sync navigation"
            };
            controls.push(Control::Toggle(toggle(
                ControlId::new("page.sync"),
                label,
                Face::icon(Icon::Sync),
                synced,
                Action::ToggleSync,
            )));
        }
        if self.annotate {
            controls.push(button(
                ControlId::new("item.annotate"),
                format!("Annotate {}", self.noun),
                Face::icon(Icon::Annotate),
                Action::AnnotateSelection,
            ));
        }
        controls
    }
}
