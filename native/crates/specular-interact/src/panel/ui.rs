//! What the app remembers about the chrome between events, whoever draws
//! it, and the heights of the chrome's rows that the canvas lays out under.

use super::{ContextMenu, ControlId, context_menu, named_controls};
use crate::App;

/// The first row of the chrome: the window's title bar strip, where the
/// tabs go. Tall enough to centre the traffic lights a window puts at
/// (14, 13).
pub const TAB_ROW: f32 = 38.0;
/// The bar under the chrome that holds the dock while a tab shows its item.
pub const DOCK_ROW: f32 = 36.0;
/// The chrome across the top of a window that draws it: the tab row alone.
/// Its tools are a ribbon and its dock a bar over the canvas.
pub const CHROME_HEIGHT: f32 = TAB_ROW;

/// What the chrome remembers between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelUi {
    /// Whether the shell draws the chrome and the sidebar, with the height
    /// and the docked tools its chrome has. Off in a headless run, which
    /// draws none and presses controls by name.
    pub(crate) shell_chrome: bool,
    /// The dropdown opened by pressing its name.
    pub(crate) open: Option<ControlId>,
    /// The context menu that is open, if one is.
    pub menu: Option<ContextMenu>,
    /// The item last picked in the sidebar, which a shift-click selects a
    /// run from.
    pub(crate) anchor: Option<specular_doc::ItemId>,
}

impl PanelUi {
    /// How much of the viewport's top edge the chrome covers.
    pub(crate) const fn chrome_height(&self) -> f32 {
        if self.shell_chrome {
            CHROME_HEIGHT
        } else {
            0.0
        }
    }

    /// The chrome drawn by the shell, with nothing open.
    pub(crate) fn drawn_by_shell() -> Self {
        Self {
            shell_chrome: true,
            ..Self::default()
        }
    }
}

/// Escape with a menu or a dropdown open closes it and does nothing else.
pub(crate) fn cancel(app: &mut App) -> bool {
    let ui = &mut app.session.panel;
    ui.menu.take().is_some() | ui.open.take().is_some()
}

/// Forgets what names a control that is no longer there: the selection or
/// the tool changed under it. A field being edited whose control has gone
/// is dropped with what was typed.
pub(crate) fn tidy(app: &mut App) {
    if let Some(edit) = app.session.editing.as_ref().filter(|edit| edit.is_field()) {
        let id = ControlId::from(edit.entity().as_str().to_owned());
        if super::field_named(app, &id).is_none() {
            app.session.editing = None;
        }
    }
    let menu_gone = (app.session.panel.menu.as_ref())
        .is_some_and(|open| context_menu(app, &open.target).is_none());
    if menu_gone {
        app.session.panel.menu = None;
    }
    let open_gone =
        (app.session.panel.open.as_ref()).is_some_and(|open| !named_controls(app).contains(open));
    if open_gone {
        app.session.panel.open = None;
    }
}
