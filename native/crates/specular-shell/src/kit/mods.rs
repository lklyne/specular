//! The mod host in the window (ADR 0055): the mods the layout names, each
//! drawn in its region in place of the app's own panel.
//!
//! [`Mods`] is held by the root view, which brings it in step as it draws:
//! the mods folder is written with the shipped mods on the first draw, a
//! region whose layout node names a mod loads it (and its `props` are
//! published as `mods/<name>/props`), and a mod renders again only when a
//! key it read moved since the last draw. The keyed state is the canvas's
//! own [`Revisions`](specular_interact::Revisions), published each time the
//! models are read, so a pan, which moves no key, never enters a script.
//! What the mods cost goes to the runtime for `GET /mods`, and their
//! palette entries to the app. An overlay the layout fills with a mod (the
//! context menu, the Share slot, the notices, the first-run view) is drawn
//! where the app's own one would be.

mod host;

use std::cell::RefCell;
use std::path::PathBuf;

use gpui_kit::{
    AnyElement, AnyView, App, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use serde_json::Value;
use specular_interact::{Event, Layout, ModPaletteEntry, Overlay, Region, RegionNode};
use specular_mods::{Hosting, ModSet, Placed, shipped};

use crate::{canvas, theme};

/// The mods the window has loaded, once it has a folder for them.
#[derive(Default)]
pub(crate) struct Mods {
    set: Option<ModSet>,
    /// Set once the folder could not be had, so it is not asked again.
    off: bool,
    palette: Vec<ModPaletteEntry>,
    stats: Option<Value>,
}

/// `<app home>/mods`, written with the shipped mods, or `None` when the
/// window keeps no settings (a scripted run) or the folder cannot be
/// written.
fn folder() -> Option<PathBuf> {
    let home = canvas::with(|canvas| canvas.runtime.app_home().map(std::path::Path::to_path_buf))??;
    let folder = home.join("mods");
    match shipped::install(&folder) {
        Ok(kept) => {
            for name in kept {
                tracing::info!("mod `{name}` is a clone and was left as it is");
            }
            Some(folder)
        }
        Err(error) => {
            tracing::warn!(folder = %folder.display(), "the shipped mods were not written: {error}");
            None
        }
    }
}

impl Mods {
    /// Brings the mods in step with `layout` and the state, and tells the
    /// app and the API what changed. Called as the root view draws.
    pub(crate) fn sync(&mut self, layout: &Layout, window: &mut Window, cx: &mut App) {
        if self.set.is_none() && !self.off {
            let revisions = canvas::state();
            match (folder(), revisions) {
                (Some(folder), Some(revisions)) => {
                    let hosting = Hosting {
                        host: std::rc::Rc::new(host::KitHost),
                        components: host::components(revisions.clone()),
                        revisions,
                        reads: std::rc::Rc::default(),
                    };
                    self.set = Some(ModSet::new(folder, hosting));
                }
                (None, Some(_)) => self.off = true,
                (_, None) => return,
            }
        }
        let Some(set) = self.set.as_mut() else {
            return;
        };
        if set.sync(layout, window, cx) || set.palette() != self.palette {
            self.palette = set.palette();
            let event = Event::ModPalette(self.palette.clone());
            // Not from inside the draw: the update runs effects that may
            // reach the window.
            window.defer(cx, move |_, _| canvas::dispatch(event));
        }
        set.refresh(cx);
        let stats = set.stats();
        if self.stats.as_ref() != Some(&stats) {
            self.stats = Some(stats.clone());
            canvas::with(|canvas| canvas.runtime.set_mod_stats(stats));
        }
    }

    /// What the layout's node for `region` places, when it names a mod:
    /// the mod's view, or why it did not load. `None` draws the app's own
    /// panel.
    pub(crate) fn region(&self, region: Region, layout: &Layout, cx: &App) -> Option<AnyElement> {
        self.node(layout.node(region), cx).map(Slot::element)
    }

    /// What the layout's node for `overlay` places, as [`Mods::region`].
    pub(crate) fn overlay(
        &self,
        overlay: Overlay,
        layout: &Layout,
        cx: &App,
    ) -> Option<AnyElement> {
        self.node(layout.overlay(overlay), cx).map(Slot::element)
    }

    fn node(&self, node: &RegionNode, cx: &App) -> Option<Slot> {
        let name = node.fill.as_deref()?;
        Some(match self.set.as_ref()?.placed(name, cx)? {
            Placed::View(view) => Slot::View(view),
            Placed::Failed(error) => Slot::Failed(name.to_owned(), error.to_owned()),
        })
    }

    /// Puts what the layout names for the Share slot where this frame's
    /// tab row finds it, the app's own or a mod's.
    pub(crate) fn fill_share_slot(&self, layout: &Layout, shown: bool, cx: &App) {
        let slot = if shown {
            self.node(layout.overlay(Overlay::Share), cx)
                .unwrap_or(Slot::Native)
        } else {
            Slot::Hidden
        };
        SHARE.with_borrow_mut(|share| *share = Some(slot));
    }
}

/// What a slot the layout names holds this frame.
#[derive(Clone)]
enum Slot {
    /// The app's own panel.
    Native,
    /// Nothing: the layout or a toggle hides it.
    Hidden,
    /// A mod's view.
    View(AnyView),
    /// The mod's name and why it did not load.
    Failed(String, String),
}

impl Slot {
    fn element(self) -> AnyElement {
        match self {
            Self::View(view) => view.into_any_element(),
            Self::Failed(name, error) => error_card(&name, &error),
            Self::Native | Self::Hidden => div().into_any_element(),
        }
    }
}

thread_local! {
    /// The Share slot as the root view last filled it.
    static SHARE: RefCell<Option<Slot>> = const { RefCell::new(None) };
    /// The first-run view's Prism while that view is shown, for a welcome
    /// mod's `Prism`.
    static HERO: RefCell<Option<AnyView>> = const { RefCell::new(None) };
}

/// Hands the first-run view's Prism to a welcome mod's `Prism`, or takes
/// it back with `None`.
pub(crate) fn set_hero(hero: Option<AnyView>) {
    HERO.with_borrow_mut(|held| *held = hero);
}

/// The first-run view's Prism, for a welcome mod's `Prism`.
pub(crate) fn hero() -> Option<AnyView> {
    HERO.with_borrow(Clone::clone)
}

/// The Share slot for this frame: a mod's view or its error card, `None`
/// for the app's own Share button, or an empty element while it is hidden.
pub(crate) fn share_slot() -> Option<AnyElement> {
    match SHARE.with_borrow(Clone::clone).unwrap_or(Slot::Native) {
        Slot::Native => None,
        slot => Some(slot.element()),
    }
}

/// What a region shows while its mod does not load: the reason, which is
/// also in `mods/<name>.error`.
fn error_card(name: &str, error: &str) -> AnyElement {
    div()
        .id(gpui_kit::SharedString::from(format!("mod-error-{name}")))
        .occlude()
        .absolute()
        .top(px(8.0))
        .left(px(8.0))
        .w(px(280.0))
        .p_2()
        .rounded(px(8.0))
        .bg(theme::solid(theme::popup()))
        .border_1()
        .border_color(theme::solid(theme::error()))
        .shadow_md()
        .text_color(theme::solid(theme::text()))
        .text_size(px(11.0))
        .child(format!("The mod `{name}` did not load: {error}"))
        .into_any_element()
}
