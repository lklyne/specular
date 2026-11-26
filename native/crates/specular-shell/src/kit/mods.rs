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
//! palette entries to the app.

mod host;

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _,
    Window, div, px,
};
use serde_json::Value;
use specular_interact::{Event, Layout, ModPaletteEntry, Region};
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
        let name = layout.node(region).fill.as_deref()?;
        let placed = self.set.as_ref()?.placed(name, cx)?;
        Some(match placed {
            Placed::View(view) => view.into_any_element(),
            Placed::Failed(error) => error_card(name, error),
        })
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
