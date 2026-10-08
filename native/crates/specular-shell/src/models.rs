//! What GPUI draws from, a part to a view.
//!
//! The models are built from the app whole, but each view of the Kit's UI
//! draws one part, and a part that says what it said before is kept as the
//! same allocation. A view is told to draw again only when the part it
//! holds is no longer the one here ([`same`]), so a streamed word re-draws
//! the transcript and leaves the sidebar's two thousand rows alone.

use std::rc::Rc;

use specular_interact::{
    App, ChatModel, Menu, OnboardingModel, PopupAnchor, PopupModel, SidebarModel, ToolbarModel,
    chat, menus, onboarding, popup_for, sidebar, toolbar,
};

/// Puts `new` in `held` unless it says what `held` already does. Returns
/// whether it was put there.
pub(crate) fn keep<T: PartialEq>(held: &mut Rc<T>, new: T) -> bool {
    if **held == new {
        return false;
    }
    *held = Rc::new(new);
    true
}

/// [`keep`] for a part that may not be there at all.
pub(crate) fn keep_some<T: PartialEq>(held: &mut Option<Rc<T>>, new: Option<T>) -> bool {
    match (held.as_mut(), new) {
        (None, None) => false,
        (Some(held), Some(new)) => keep(held, new),
        (_, new) => {
            *held = new.map(Rc::new);
            true
        }
    }
}

/// Whether a view holding `held` is showing `now`: the same allocation, or
/// nothing in both.
pub(crate) fn same<T>(held: Option<&Rc<T>>, now: Option<&Rc<T>>) -> bool {
    match (held, now) {
        (None, None) => true,
        (Some(held), Some(now)) => Rc::ptr_eq(held, now),
        _ => false,
    }
}

/// The models, as `update` last left the app.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Models {
    /// The tool buttons and the zoom readout.
    pub(crate) toolbar: Rc<ToolbarModel>,
    /// The popup hung from the toolbar, when the tool in hand has one. A
    /// popup beside a canvas item is drawn in the canvas's own pass.
    pub(crate) popup: Option<Rc<PopupModel>>,
    /// The left sidebar.
    pub(crate) sidebar: Rc<SidebarModel>,
    /// The right panel: the canvas's agent threads and the composer.
    pub(crate) chat: Rc<ChatModel>,
    /// The menu bar's menus that come from the app.
    pub(crate) menus: Rc<Vec<Menu>>,
    /// The first-run view, while no space is open.
    pub(crate) onboarding: Option<Rc<OnboardingModel>>,
}

/// The popup the Kit draws: the one hung from the toolbar.
fn toolbar_popup(app: &App) -> Option<PopupModel> {
    popup_for(app).filter(|popup| matches!(popup.anchor, PopupAnchor::Toolbar { .. }))
}

impl Models {
    /// The models of `app`.
    pub(crate) fn of(app: &App) -> Self {
        Self {
            toolbar: Rc::new(toolbar(app)),
            popup: toolbar_popup(app).map(Rc::new),
            sidebar: Rc::new(sidebar(app)),
            chat: Rc::new(chat(app)),
            menus: Rc::new(menus(app)),
            onboarding: onboarding(app).map(Rc::new),
        }
    }

    /// Brings every part in step with `app`, keeping each that says what it
    /// said. Returns whether any changed.
    pub(crate) fn refresh(&mut self, app: &App) -> bool {
        // Nothing draws a hidden sidebar, and its rows are a walk of the
        // whole document: it is read again when it is shown.
        let sidebar_read = app.session().sidebar.shown() || self.sidebar.visible;
        // Not `||`: every part is brought in step, whichever changed first.
        let changed = [
            keep(&mut self.toolbar, toolbar(app)),
            keep_some(&mut self.popup, toolbar_popup(app)),
            sidebar_read && keep(&mut self.sidebar, sidebar(app)),
            keep(&mut self.chat, chat(app)),
            keep(&mut self.menus, menus(app)),
            keep_some(&mut self.onboarding, onboarding(app)),
        ];
        changed.contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;
    use specular_core::Camera;
    use specular_interact::{Action, SidebarAction, Tool};
    use specular_testkit::TestApp;

    use super::*;

    /// The parts of `models` that are no longer the allocations `before`
    /// has, by name: the views that would be told to draw again.
    fn redrawn(before: &Models, models: &Models) -> Vec<&'static str> {
        let parts = [
            ("toolbar", Rc::ptr_eq(&before.toolbar, &models.toolbar)),
            ("popup", same(before.popup.as_ref(), models.popup.as_ref())),
            ("sidebar", Rc::ptr_eq(&before.sidebar, &models.sidebar)),
            ("chat", Rc::ptr_eq(&before.chat, &models.chat)),
        ];
        (parts.iter())
            .filter_map(|(name, kept)| (!kept).then_some(*name))
            .collect()
    }

    #[test]
    fn a_change_to_the_app_redraws_only_the_parts_whose_model_it_changed() {
        let pan = Action::SetCamera(Camera::new(Vec2::new(-240.0, 80.0), 1.0));
        let zoom = Action::SetCamera(Camera::new(Vec2::new(-240.0, 80.0), 2.0));
        let toggle = || Action::Sidebar(SidebarAction::Toggle);
        let cases: [(&str, Action, &[&str]); 9] = [
            ("the sidebar shown", toggle(), &["toolbar", "sidebar"]),
            ("a pan", pan, &[]),
            ("a zoom", zoom, &["toolbar"]),
            ("an item made", Action::Duplicate, &["sidebar"]),
            ("the sidebar hidden", toggle(), &["toolbar", "sidebar"]),
            // A hidden sidebar's rows are not read: the model kept is the
            // one from when it was hidden.
            (
                "an item made behind a hidden sidebar",
                Action::Duplicate,
                &[],
            ),
            ("the sidebar shown again", toggle(), &["toolbar", "sidebar"]),
            (
                "a tool with a popup taken",
                Action::SetTool(Tool::AddShape),
                &["toolbar", "popup"],
            ),
            (
                "the same tool taken again",
                Action::SetTool(Tool::AddShape),
                &[],
            ),
        ];
        let mut app = TestApp::with_pages(2);
        app.with_chat_panel().select(&["p1"]);
        let mut models = Models::of(app.app());
        for (what, action, expected) in cases {
            let before = models.clone();
            app.act(action);
            let changed = models.refresh(app.app());
            assert_eq!(redrawn(&before, &models), expected, "{what}");
            assert_eq!(changed, !expected.is_empty(), "{what}");
            if models.sidebar.visible {
                assert_eq!(models, Models::of(app.app()), "{what}");
            }
        }
    }
}
