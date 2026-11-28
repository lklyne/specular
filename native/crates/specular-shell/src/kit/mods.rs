//! The mod host in the window (ADR 0055): the mods the layout names, each
//! drawn in its region in place of the app's own panel.
//!
//! [`Mods`] is held by the root view, which brings it in step as it draws:
//! the mods folder (which the runtime wrote the shipped mods into as it
//! opened) is taken on the first draw, a
//! region whose layout node names a mod loads it (and its `props` are
//! published as `mods/<name>/props`), and a mod renders again only when a
//! key it read moved since the last draw. The keyed state is the canvas's
//! own [`Revisions`](specular_interact::Revisions), published each time the
//! models are read, so a pan, which moves no key, never enters a script.
//! What the mods cost goes to the runtime for `GET /mods`. The mods with
//! canvas items, tools or commands (ADR 0057) run whatever the layout
//! says, drawn out of sight in a box of no size. An overlay the layout fills with a mod (the
//! context menu, the Share slot, the notices, the first-run view) is drawn
//! where the app's own one would be.

mod composer;
mod host;
mod text_field;

use std::cell::RefCell;
use std::path::PathBuf;

use gpui_kit::{
    AnyElement, AnyView, App, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, Window, div, px,
};
use serde_json::Value;
use specular_interact::mods::SectionPlace;
use specular_interact::{Layout, Overlay};
use specular_mods::{Hosting, ModCopy, ModSet, Placed};

use crate::{canvas, theme};

/// The mods the window has loaded, once it has a folder for them.
#[derive(Default)]
pub(crate) struct Mods {
    set: Option<ModSet>,
    /// Set once the folder could not be had, so it is not asked again.
    off: bool,
    /// The mods that run out of sight, for their canvas items, tools and
    /// commands.
    background: Vec<String>,
    /// The mods that add a section to the sidebar, in name order.
    sections: Vec<String>,
    stats: Option<Value>,
    /// Draws the window again when a mod's source changes, so a save loads
    /// it again with nothing else moving.
    watch: Option<gpui_kit::Task<()>>,
    /// The overlays open at the last sync, to start each one that opens
    /// afresh.
    open: Vec<String>,
}

/// The mods folder, which the runtime wrote the shipped mods into as it
/// opened, or `None` when the window keeps no settings (a scripted run) or
/// the folder could not be written.
fn folder() -> Option<PathBuf> {
    canvas::with(|canvas| {
        canvas
            .runtime
            .mods_folder()
            .map(std::path::Path::to_path_buf)
    })?
}

/// Asks `window` to draw again each time the source of a mod in `folder`
/// changes. The folder is looked at off the main thread.
fn watch(folder: PathBuf, window: &mut Window, cx: &mut App) -> gpui_kit::Task<()> {
    window.spawn(cx, async move |cx| {
        let mut seen = None;
        loop {
            cx.background_executor()
                .timer(specular_mods::SOURCE_POLL)
                .await;
            let looked = folder.clone();
            let now = (cx.background_executor())
                .spawn(async move { specular_mods::source_stamp(&looked) })
                .await;
            if seen.is_some_and(|seen| seen != now)
                && cx.update(|window, _| window.refresh()).is_err()
            {
                break;
            }
            seen = Some(now);
        }
    })
}

impl Mods {
    /// Brings the mods in step with `layout` and the state, and tells the
    /// app and the API what changed. Called as the root view draws.
    pub(crate) fn sync(&mut self, models: &canvas::Models, window: &mut Window, cx: &mut App) {
        if self.set.is_none() && !self.off {
            let revisions = canvas::state();
            match (folder(), revisions) {
                (Some(folder), Some(revisions)) => {
                    self.watch = Some(watch(folder.clone(), window, cx));
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
        // An overlay that opens is a new instance of its mod, as if loaded
        // for the first time: its fields are empty and the one it focuses
        // has the keys.
        let open: Vec<String> = (models.mod_overlays.iter())
            .map(|spec| spec.id.clone())
            .collect();
        for id in open.iter().filter(|id| !self.open.contains(id)) {
            set.restart(id);
        }
        self.open = open;
        let mut copies = ModCopy::of_layout(&models.layout);
        // An overlay open by an overlay's own name (the palette) is that
        // copy, with its overlay props.
        copies.retain(|copy| !models.mod_overlays.iter().any(|open| open.id == copy.id));
        copies.extend(models.mod_overlays.iter().map(|spec| ModCopy {
            id: spec.id.clone(),
            name: spec.name.clone(),
            props: spec.copy_props(),
        }));
        let (sections, background): (Vec<String>, Vec<String>) = canvas::with(|canvas| {
            let catalog = canvas.runtime.app().mod_catalog();
            let sections = (catalog.sections(SectionPlace::Sidebar))
                .map(|installed| installed.name.clone())
                .collect();
            let background = (catalog.background())
                .map(|installed| installed.name.clone())
                .collect();
            (sections, background)
        })
        .unwrap_or_default();
        // A mod the layout places already runs: its items, tools and
        // commands need no second copy, and it is not a section too.
        let unplaced = |names: Vec<String>, copies: &[ModCopy]| -> Vec<String> {
            (names.into_iter())
                .filter(|name| !copies.iter().any(|copy| copy.name == *name))
                .collect()
        };
        self.sections = unplaced(sections, &copies);
        copies.extend(self.sections.iter().map(|name| ModCopy::background(name)));
        self.background = unplaced(background, &copies);
        copies.extend(self.background.iter().map(|name| ModCopy::background(name)));
        set.sync(&copies, window, cx);
        set.refresh(cx);
        let stats = set.stats();
        if self.stats.as_ref() != Some(&stats) {
            self.stats = Some(stats.clone());
            canvas::with(|canvas| canvas.runtime.set_mod_stats(stats));
        }
    }

    /// The mods that run out of sight, in a box of no size: a mod's script
    /// runs only as its view is drawn.
    pub(crate) fn background(&self, cx: &App) -> Option<AnyElement> {
        let views = self.set.as_ref()?.views(&self.background, cx);
        (!views.is_empty()).then(|| {
            div()
                .absolute()
                .w(px(0.0))
                .h(px(0.0))
                .overflow_hidden()
                .children(views)
                .into_any_element()
        })
    }

    /// Puts the sidebar's section mods, each its view or why it did not
    /// load, where this frame's sidebar finds them, the app's own or a
    /// mod's `SidebarSections`.
    pub(crate) fn fill_sections(&self, cx: &App) {
        let slots = (self.sections.iter())
            .filter_map(|id| self.slot(id, cx))
            .collect();
        SECTIONS.with_borrow_mut(|sections| *sections = slots);
    }

    /// What the copy `id` shows: its view, or why it did not load.
    /// `None` when no copy by that id runs.
    pub(crate) fn copy(&self, id: &str, cx: &App) -> Option<AnyElement> {
        self.slot(id, cx).map(Slot::element)
    }

    fn slot(&self, id: &str, cx: &App) -> Option<Slot> {
        Some(match self.set.as_ref()?.placed(id, cx)? {
            Placed::View(view) => Slot::View(view),
            Placed::Failed(error) => Slot::Failed(id.to_owned(), error.to_owned()),
        })
    }

    /// What the layout's mod for `overlay` places, when it names one: the
    /// mod's view, or why it did not load. `None` draws the app's own.
    pub(crate) fn overlay(
        &self,
        overlay: Overlay,
        layout: &Layout,
        cx: &App,
    ) -> Option<AnyElement> {
        layout.overlay(overlay).fill.as_ref()?;
        self.copy(overlay.name(), cx)
    }

    /// Puts what the layout names for the Share slot where this frame's
    /// tab row finds it, the app's own or a mod's.
    pub(crate) fn fill_share_slot(&self, layout: &Layout, shown: bool, cx: &App) {
        let slot = if shown {
            (layout.overlay(Overlay::Share).fill.as_ref())
                .and_then(|_| self.slot(Overlay::Share.name(), cx))
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
    /// The sidebar's section mods as the root view last filled them.
    static SECTIONS: RefCell<Vec<Slot>> = const { RefCell::new(Vec::new()) };
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

/// The sidebar's section mods for this frame, in name order: each a mod's
/// view, or a line saying why it did not load.
pub(crate) fn sidebar_sections() -> Vec<AnyElement> {
    SECTIONS.with_borrow(|sections| {
        (sections.iter().cloned())
            .map(|slot| match slot {
                Slot::Failed(name, error) => div()
                    .px(px(16.0))
                    .py(px(4.0))
                    .text_size(px(11.0))
                    .text_color(theme::solid(theme::error()))
                    .child(format!("The mod `{name}` did not load: {error}"))
                    .into_any_element(),
                slot => slot.element(),
            })
            .collect()
    })
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
        .p(crate::theme::space(2.0))
        .rounded(crate::theme::corner(8.0))
        .bg(theme::solid(theme::popup()))
        .border(crate::theme::line())
        .border_color(theme::solid(theme::error()))
        .shadow_md()
        .text_color(theme::solid(theme::text()))
        .text_size(px(11.0))
        .child(format!("The mod `{name}` did not load: {error}"))
        .into_any_element()
}
