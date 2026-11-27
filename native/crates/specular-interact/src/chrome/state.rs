//! The layouts the app has, the one in force, and what was toggled over
//! each; and the gate that turns what the app shows now into the room each
//! node takes.

use std::collections::{BTreeMap, BTreeSet};

use super::layout::{Layout, SHIPPED};
use super::look::ThemeFile;
use super::node::{Content, Node};
use super::solve::{Frame, Gate, solve};
use super::{Insets, Overlay, Region, Regions, Toggles};
use crate::{App, ToolsPlace};

/// The name of the layout shown when none was picked.
pub(crate) const DEFAULT: &str = "default";

/// What the app shows now that decides the room the app's own ids take.
#[derive(Debug, Clone, Copy, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "four independent facts the window reports, not a state machine"
)]
pub struct Shown {
    /// Whether the chrome covers the viewport's edges: a window that draws
    /// it, or a script laid out as one. Without it only the sidebar does.
    pub covers: bool,
    /// Whether the sidebar is open.
    pub sidebar: bool,
    /// Whether the tools are docked down the left edge rather than floating.
    pub tools_docked: bool,
    /// Whether a tab shows its item, which keeps the dock's bar under the
    /// tab row.
    pub item_dock: bool,
    /// The right panel's width while it is shown.
    pub chat: Option<f32>,
}

impl Shown {
    /// What `app` shows now.
    pub fn of(app: &App) -> Self {
        let chat = app.session.chat;
        Self {
            covers: app.session.panel.chrome_covers,
            sidebar: app.session.sidebar.shown(),
            tools_docked: app.settings.tools == ToolsPlace::Docked,
            item_dock: app.lens().is_some(),
            chat: chat.shown().then(|| chat.width()),
        }
    }
}

/// What the app makes of `node` when it shows `shown`, with `hidden` saying
/// whether the layout and the toggles hide an id: the sidebar comes and
/// goes with Command+B, the tools take no room while they float, and the
/// right panel is as wide as it was dragged.
pub fn gate(node: &Node, shown: Shown, hidden: &dyn Fn(&Node) -> bool) -> Gate {
    if hidden(node) {
        return Gate::Hidden;
    }
    let canvas = node.content == Some(Content::Canvas);
    if !shown.covers && node.content.is_some() && !canvas && !node.is("sidebar") {
        return Gate::Hidden;
    }
    match node.id.as_deref() {
        Some("sidebar") if !shown.sidebar => Gate::Hidden,
        Some("toolbar") if !shown.tools_docked => Gate::Hidden,
        Some("chat") => shown.chat.map_or(Gate::Hidden, Gate::Px),
        _ => Gate::AsIs,
    }
}

/// Whether `node` is hidden by its id's toggle in `toggles`, else by its
/// own `hidden`.
fn hidden_by(toggles: &Toggles, node: &Node) -> bool {
    match node.id.as_deref() {
        Some(id) => toggles.get(id).unwrap_or(node.hidden),
        None => node.hidden,
    }
}

/// Every node of `layout` shown in `window`, with `toggles` over it and the
/// app showing `shown`, parents first: what the window draws and what a
/// headless run takes the canvas's insets from.
pub fn place<'a>(
    layout: &'a Layout,
    toggles: &Toggles,
    shown: Shown,
    window: Frame,
) -> Vec<super::Placed<'a>> {
    let hidden = |node: &Node| hidden_by(toggles, node);
    solve(&layout.root, window, &|node| gate(node, shown, &hidden))
}

/// Every layout, the one in force, the toggles over each, and the theme
/// files read.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChromeState {
    /// Every layout by name as drawn: [`Self::written`] without the mods
    /// turned off.
    pub(crate) layouts: BTreeMap<String, Layout>,
    /// Every layout by name as written: the shipped ones, then the files
    /// read.
    written: BTreeMap<String, Layout>,
    /// The mods turned off in the settings.
    off: BTreeSet<String>,
    /// The name of the one in force.
    pub(crate) active: String,
    /// What was toggled at run time, by layout.
    pub(crate) toggles: BTreeMap<String, Toggles>,
    /// The theme files, by name.
    pub(crate) themes: BTreeMap<String, ThemeFile>,
    fallback: Layout,
    none: Toggles,
}

impl Default for ChromeState {
    fn default() -> Self {
        let layouts: BTreeMap<String, Layout> = (SHIPPED.iter())
            .filter_map(|(name, text)| Some(((*name).to_owned(), Layout::parse(text).ok()?)))
            .collect();
        Self {
            written: layouts.clone(),
            off: BTreeSet::new(),
            layouts,
            active: DEFAULT.to_owned(),
            toggles: BTreeMap::new(),
            themes: BTreeMap::new(),
            fallback: Layout::classic(),
            none: Toggles::default(),
        }
    }
}

impl ChromeState {
    /// The layout in force.
    pub(crate) fn layout(&self) -> &Layout {
        (self.layouts.get(&self.active))
            .or_else(|| self.layouts.get(DEFAULT))
            .unwrap_or(&self.fallback)
    }

    /// The toggles over the layout in force.
    pub(crate) fn toggles(&self) -> &Toggles {
        self.toggles.get(&self.active).unwrap_or(&self.none)
    }

    /// The toggles over the layout in force, to change.
    pub(crate) fn toggles_mut(&mut self) -> &mut Toggles {
        self.toggles.entry(self.active.clone()).or_default()
    }

    /// Whether `id`, a node's, an overlay's or a control's, is hidden: as
    /// it was toggled, else as the layout has it.
    pub(crate) fn hidden(&self, id: &str) -> bool {
        self.toggles()
            .get(id)
            .unwrap_or_else(|| self.layout().hides(id))
    }

    /// Whether `node` is hidden by its id's toggle or its own `hidden`.
    fn node_hidden(&self, node: &Node) -> bool {
        hidden_by(self.toggles(), node)
    }

    /// Which regions may be drawn: the blocks with their names, and the
    /// dock overlay.
    pub(crate) fn regions(&self) -> Regions {
        let layout = self.layout();
        let drawn = |region: Region| match region {
            Region::Dock => !self.hidden(Overlay::Dock.name()),
            _ => layout
                .node(region.name())
                .is_some_and(|node| !self.node_hidden(node)),
        };
        Regions {
            tabs: drawn(Region::Tabs),
            toolbar: drawn(Region::Toolbar),
            dock: drawn(Region::Dock),
            sidebar: drawn(Region::Sidebar),
            chat: drawn(Region::Chat),
        }
    }

    /// Every node shown in `window` when the app shows `shown`, and where.
    pub(crate) fn solve(&self, window: Frame, shown: Shown) -> Vec<super::Placed<'_>> {
        place(self.layout(), self.toggles(), shown, window)
    }

    /// The canvas's insets in `window`: the canvas block's corner. What is
    /// above and left of it lies over the canvas; what is below and right
    /// sits beside it.
    pub(crate) fn insets(&self, window: Frame, shown: Shown) -> Insets {
        let canvas = (self.solve(window, shown).into_iter())
            .find(|placed| placed.node.content == Some(Content::Canvas))
            .map_or_else(Frame::default, |placed| placed.frame);
        Insets {
            left: canvas.x,
            top: canvas.y,
            right: 0.0,
            bottom: 0.0,
        }
    }
}

/// What the shell read of the layouts folder and the themes folder.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutsRead {
    /// Each layout file that read, by name.
    pub layouts: BTreeMap<String, Layout>,
    /// What was toggled over each in earlier runs, by layout.
    pub toggles: BTreeMap<String, Toggles>,
    /// The layout shown last, when one was kept.
    pub active: Option<String>,
    /// Each theme file that read, by name.
    pub themes: BTreeMap<String, ThemeFile>,
}

impl ChromeState {
    /// Takes in what the shell read. A layout no file has keeps its last
    /// good version; a kept choice of a layout there is not falls back to
    /// the default.
    pub(crate) fn load(&mut self, read: LayoutsRead) {
        self.written.extend(read.layouts);
        self.redraw();
        if !read.toggles.is_empty() {
            self.toggles = read.toggles;
        }
        if let Some(active) = read.active {
            self.active = if self.layouts.contains_key(&active) {
                active
            } else {
                DEFAULT.to_owned()
            };
        }
        self.themes = read.themes;
    }

    /// The layout in force as written, with the mods turned off still in it.
    pub(crate) fn written_layout(&self) -> &Layout {
        (self.written.get(&self.active))
            .or_else(|| self.written.get(DEFAULT))
            .unwrap_or(&self.fallback)
    }

    /// Turns the mods `off` off, and every other mod on.
    pub(crate) fn set_off(&mut self, off: &BTreeSet<String>) {
        if self.off != *off {
            self.off.clone_from(off);
            self.redraw();
        }
    }

    fn redraw(&mut self) {
        self.layouts = (self.written.iter())
            .map(|(name, layout)| (name.clone(), layout.without_mods(&self.off)))
            .collect();
    }
}
