//! Where the mods drawn over the canvas go: a toolbar mod floating where
//! the app's own ribbon floats while the tools float, and the notices over
//! the canvas's lower right corner. The mod draws the panel itself; these
//! give it the box and keep the canvas from taking the clicks it gets. The
//! layout's blocks are placed by `frame.rs`.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _,
    Window, div, px,
};
use specular_interact::{Overlay, Region, ToolsPlace};

use super::ShellView;
use super::onboarding::HeroPrism;
use super::toolbar::RIBBON_INSET;
use crate::canvas::Models;

/// The mods drawn this frame over the canvas, in place of the app's own.
pub(super) struct ModPanels {
    /// Whether the tab row keeps its own zoom readout: no toolbar mod
    /// carries it.
    pub(super) zoom_in_tabs: bool,
    /// A toolbar mod, floating while the tools float.
    pub(super) floating: Option<AnyElement>,
    /// A dock mod, in the dock's place.
    pub(super) dock: Option<AnyElement>,
    /// A notices mod, placed over the canvas.
    pub(super) toast: Option<AnyElement>,
}

impl ShellView {
    /// Brings the mods in step and gives back those drawn over the canvas.
    /// The Share slot is filled for the tab row to find.
    pub(super) fn mod_panels(
        &mut self,
        models: Option<&Models>,
        window: &mut Window,
        cx: &mut App,
    ) -> ModPanels {
        let Some(models) = models else {
            return ModPanels {
                zoom_in_tabs: true,
                floating: None,
                dock: None,
                toast: None,
            };
        };
        self.mods.sync(models, window, cx);
        let layout = &models.layout;
        let toolbar_mod = layout.fill(Region::Toolbar).is_some();
        let floating = (toolbar_mod && models.toolbar.tools != ToolsPlace::Docked)
            .then(|| self.mods.copy(Region::Toolbar.name(), cx))
            .flatten();
        (self.mods).fill_share_slot(layout, models.shows(Overlay::Share), cx);
        self.mods.fill_sections(cx);
        let toast = if models.shows(Overlay::Toast) {
            self.mods.overlay(Overlay::Toast, layout, cx)
        } else {
            None
        };
        let toast = toast.map(|toast| corner(toast, models.insets.bottom));
        let dock = (models.shows(Overlay::Dock))
            .then(|| self.mods.overlay(Overlay::Dock, layout, cx))
            .flatten();
        ModPanels {
            zoom_in_tabs: !toolbar_mod,
            floating,
            dock,
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
        self.mods.sync(models, window, cx);
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

/// The edges of what the canvas is seen through that a floating ribbon is
/// placed against.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlotEdges {
    /// The canvas's left edge, from the window's.
    pub(super) left: f32,
    /// The canvas's top edge, from the window's.
    pub(super) top: f32,
    /// The foot of what lies over the canvas's top: the tab row and the
    /// dock's bar while it is shown.
    pub(super) under: f32,
}

/// The toolbar mod floating where the app's own ribbon floats. Only the
/// ribbon takes clicks, not the row or column it is centred in.
pub(super) fn floating_mod(ribbon: AnyElement, place: ToolsPlace, edges: SlotEdges) -> AnyElement {
    let ribbon = div().id("ribbon-mod").occlude().child(ribbon);
    floating(ribbon.into_any_element(), place, edges)
}

/// `ribbon` placed as the tools float: centred over the top or the bottom
/// of what the canvas is seen through, or centred down its left edge.
pub(super) fn floating(ribbon: AnyElement, place: ToolsPlace, edges: SlotEdges) -> AnyElement {
    match place {
        ToolsPlace::FloatLeft => v_flex()
            .absolute()
            .left(px(edges.left + RIBBON_INSET))
            .top(px(edges.top))
            .bottom_0()
            .justify_center()
            .child(ribbon)
            .into_any_element(),
        ToolsPlace::Docked | ToolsPlace::FloatTop | ToolsPlace::FloatBottom => h_flex()
            .absolute()
            .left(px(edges.left))
            .right_0()
            .when(place == ToolsPlace::FloatTop, |this| {
                this.top(px(edges.under + RIBBON_INSET))
            })
            .when(place == ToolsPlace::FloatBottom, |this| {
                this.bottom(px(RIBBON_INSET))
            })
            .justify_center()
            .child(ribbon)
            .into_any_element(),
    }
}
