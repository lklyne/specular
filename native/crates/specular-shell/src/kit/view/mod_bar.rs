//! Where the mods the layout names are drawn: a toolbar mod in the docked
//! ribbon's place or floating where the app's own ribbon floats, a tab row
//! mod in the tab row, and the notices over the canvas's lower right
//! corner. The mod draws the panel itself; these give it the box and keep
//! the canvas from taking the clicks it gets.

use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _,
    Window, div, px,
};
use specular_interact::{Overlay, Region, TOOLS_DOCK_WIDTH, ToolsPlace};

use super::ShellView;
use super::onboarding::HeroPrism;
use super::toolbar::RIBBON_INSET;
use crate::canvas::Models;
use crate::theme;

/// The mods drawn this frame in place of the app's own panels.
pub(super) struct ModPanels {
    /// Whether the tab row keeps its own zoom readout: no toolbar mod
    /// carries it.
    pub(super) zoom_in_tabs: bool,
    /// A toolbar mod, docked.
    pub(super) docked: Option<AnyElement>,
    /// A toolbar mod, floating.
    pub(super) floating: Option<AnyElement>,
    /// A tab row mod.
    pub(super) tabs: Option<AnyElement>,
    /// A notices mod, placed over the canvas.
    pub(super) toast: Option<AnyElement>,
}

impl ShellView {
    /// Brings the mods in step and gives back those the layout names for
    /// the regions and overlays drawn in the window's main view. The Share
    /// slot is filled for the tab row to find.
    pub(super) fn mod_panels(
        &mut self,
        models: Option<&Models>,
        window: &mut Window,
        cx: &mut App,
    ) -> ModPanels {
        let Some(models) = models else {
            return ModPanels {
                zoom_in_tabs: true,
                docked: None,
                floating: None,
                tabs: None,
                toast: None,
            };
        };
        self.mods.sync(&models.layout, window, cx);
        let layout = &models.layout;
        let ribbon = self.mods.region(Region::Toolbar, layout, cx);
        let zoom_in_tabs = ribbon.is_none();
        let (docked, floating) = if models.toolbar.tools == ToolsPlace::Docked {
            (ribbon, None)
        } else {
            (None, ribbon)
        };
        (self.mods).fill_share_slot(layout, models.shows(Overlay::Share), cx);
        let toast = if models.shows(Overlay::Toast) {
            self.mods.overlay(Overlay::Toast, layout, cx)
        } else {
            None
        };
        let toast = toast.map(|toast| corner(toast, models.insets.bottom));
        ModPanels {
            zoom_in_tabs,
            docked,
            floating,
            tabs: self.mods.region(Region::Tabs, layout, cx),
            toast,
        }
    }
}

impl ShellView {
    /// A welcome mod in place of the app's own first-run view, over the
    /// whole window, with `hero` as its `Prism`. `None` draws the app's own.
    pub(super) fn welcome_mod(
        &mut self,
        models: Option<&Models>,
        hero: &Entity<HeroPrism>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let models = models?;
        self.mods.sync(&models.layout, window, cx);
        crate::mods::set_hero(Some(hero.clone().into()));
        let welcome = self.mods.overlay(Overlay::Welcome, &models.layout, cx)?;
        Some(
            div()
                .id("welcome-mod")
                .size_full()
                .child(welcome)
                .into_any_element(),
        )
    }
}

/// A tab row mod in the tab row's box.
pub(super) fn tab_row(row: AnyElement) -> AnyElement {
    div()
        .id("tabs-mod")
        .w_full()
        .h(px(theme::TAB_ROW))
        .flex_shrink_0()
        .child(row)
        .into_any_element()
}

/// The notices mod over the canvas's lower right corner, `bottom` up from
/// the window's edge. Only what the mod draws takes clicks.
fn corner(toast: AnyElement, bottom: f32) -> AnyElement {
    div()
        .absolute()
        .right(px(12.0))
        .bottom(px(bottom + 12.0))
        .child(div().id("toast-mod").occlude().child(toast))
        .into_any_element()
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
