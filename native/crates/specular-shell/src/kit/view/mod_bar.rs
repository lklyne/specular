//! Where a toolbar mod is drawn: in the docked ribbon's place, or floating
//! where the app's own ribbon floats. The mod draws the ribbon itself; these
//! give it the box and keep the canvas from taking the clicks it gets.

use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, Window,
    div, px,
};
use specular_interact::{Region, TOOLS_DOCK_WIDTH, ToolsPlace};

use super::ShellView;
use super::toolbar::RIBBON_INSET;
use crate::canvas::Models;
use crate::theme;

impl ShellView {
    /// Brings the mods in step and draws a toolbar mod, which takes the
    /// place of the app's own ribbon and of the tab row's zoom readout:
    /// whether the tab row keeps its zoom, then the mod docked or floating.
    pub(super) fn mod_ribbon(
        &mut self,
        models: Option<&Models>,
        window: &mut Window,
        cx: &mut App,
    ) -> (bool, Option<AnyElement>, Option<AnyElement>) {
        let Some(models) = models else {
            return (true, None, None);
        };
        self.mods.sync(&models.layout, window, cx);
        let ribbon = self.mods.region(Region::Toolbar, &models.layout, cx);
        let zoom_in_tabs = ribbon.is_none();
        if models.toolbar.tools == ToolsPlace::Docked {
            (zoom_in_tabs, ribbon, None)
        } else {
            (zoom_in_tabs, None, ribbon)
        }
    }
}

/// The mod in the docked ribbon's box, `left` from the window's edge.
pub(super) fn docked(ribbon: AnyElement, left: f32) -> impl IntoElement {
    div()
        .id("ribbon-mod")
        .occlude()
        .absolute()
        .left(px(left))
        .top(px(theme::chrome_top()))
        .bottom_0()
        .w(px(TOOLS_DOCK_WIDTH))
        .child(ribbon)
}

/// The mod centred over the top or the bottom of what the canvas is seen
/// through, as [`super::toolbar::floating`] places the app's own ribbon.
/// Only the ribbon takes clicks, not the row it is centred in.
pub(super) fn floating(
    ribbon: AnyElement,
    place: ToolsPlace,
    (left, top): (f32, f32),
) -> impl IntoElement {
    h_flex()
        .absolute()
        .left(px(left))
        .right_0()
        .when(place == ToolsPlace::FloatTop, |this| {
            this.top(px(top + RIBBON_INSET))
        })
        .when(place == ToolsPlace::FloatBottom, |this| {
            this.bottom(px(RIBBON_INSET))
        })
        .justify_center()
        .child(div().id("ribbon-mod").occlude().child(ribbon))
}
