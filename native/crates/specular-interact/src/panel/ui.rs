//! What the app remembers about the chrome between events, whoever draws
//! it.

use super::{ContextMenu, ControlId, context_menu, named_controls};
use crate::App;

/// What the chrome remembers between events.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelUi {
    /// Whether the shell draws the chrome and the sidebar, with the height
    /// and the docked tools its chrome has. Off in a headless run, which
    /// draws none and presses controls by name.
    pub(crate) shell_chrome: bool,
    /// Whether the chrome and the docked tools cover the viewport's edges.
    /// They do wherever the shell draws them, and in a script run that
    /// draws none but lays out as the window does, so a position or a
    /// camera in a script means the same in both.
    pub(crate) chrome_covers: bool,
    /// The dropdown opened by pressing its name.
    pub(crate) open: Option<ControlId>,
    /// The context menu that is open, if one is.
    pub menu: Option<ContextMenu>,
    /// The item last picked in the sidebar, which a shift-click selects a
    /// run from.
    pub(crate) anchor: Option<specular_doc::ItemId>,
}

impl PanelUi {
    /// The chrome drawn by the shell, with nothing open.
    pub(crate) fn drawn_by_shell() -> Self {
        Self {
            shell_chrome: true,
            chrome_covers: true,
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
