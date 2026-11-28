//! The window's regions, which of them are drawn, and the room they take
//! from the canvas (ADR 0055).
//!
//! The window has five regions over or beside the canvas: the tab row, the
//! tools, the dock, the sidebar and the right panel. A [`Layout`] (a file
//! the shell reads) says whether each is drawn, and [`Toggles`] are what a
//! person flipped at run time with `ui.toggle`, kept apart from the file.
//! [`insets`] turns the regions drawn and what the app shows now into the
//! edges the canvas is seen between. It is the one answer both runners
//! read: the window positions its regions by it, and a headless run, which
//! draws none, lays the viewport out by the same numbers.

mod layout;
mod toggle;

pub use self::layout::{Layout, LayoutError, LayoutFile, RegionNode};
pub use self::toggle::{LayoutAction, RESET_KEYS, Toggles};
pub(crate) use self::toggle::{is_reset, run as run_layout_action};

use crate::{App, ToolsPlace};

/// The first row of the chrome: the window's title bar strip, where the
/// tabs go. Tall enough to centre the traffic lights a window puts at
/// (14, 13).
pub const TAB_ROW: f32 = 38.0;
/// The bar under the tab row that holds the dock while a tab shows its item.
pub const DOCK_ROW: f32 = 36.0;
/// The chrome across the top of a window that draws it: the tab row alone.
/// Its tools are a ribbon and its dock a bar over the canvas.
pub const CHROME_HEIGHT: f32 = TAB_ROW;
/// The width of the tools' ribbon while it is docked: a square button and
/// the room either side of it.
pub const TOOLS_DOCK_WIDTH: f32 = 36.0;
/// The width the sidebar covers when it is shown: `LEFT_SIDEBAR_WIDTH` in
/// `runtime-constants.ts`.
pub const SIDEBAR_WIDTH: f32 = 256.0;

/// One region of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    /// The tab row across the top, which is the title bar.
    Tabs,
    /// The tools: a ribbon down the left edge, or floating over the canvas.
    Toolbar,
    /// The bar of the selection's or the tool's controls.
    Dock,
    /// The left sidebar.
    Sidebar,
    /// The right panel: the agent threads and the composer.
    Chat,
}

impl Region {
    /// Every region, in the layout file's order.
    pub const ALL: [Self; 5] = [
        Self::Tabs,
        Self::Toolbar,
        Self::Dock,
        Self::Sidebar,
        Self::Chat,
    ];

    /// The name the layout file and `ui.toggle` call it by.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Tabs => "tabs",
            Self::Toolbar => "toolbar",
            Self::Dock => "dock",
            Self::Sidebar => "sidebar",
            Self::Chat => "chat",
        }
    }

    /// The region called `name`.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.name() == name)
    }
}

/// Something the window draws over everything else, at a place of its
/// own, which a layout can fill with a mod as it fills a region. Unlike a
/// region, an overlay takes no room from the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    /// The context menu a right press on the canvas opens, at the press.
    Menu,
    /// The Share button at the end of the tab row, and its popover.
    Share,
    /// The notices, in the canvas's lower right corner.
    Toast,
    /// The command palette.
    Palette,
    /// The first-run view, in place of the canvas while no space is open.
    Welcome,
}

impl Overlay {
    /// Every overlay, in the layout file's order.
    pub const ALL: [Self; 5] = [
        Self::Menu,
        Self::Share,
        Self::Toast,
        Self::Palette,
        Self::Welcome,
    ];

    /// The name the layout file and `ui.toggle` call it by.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Share => "share",
            Self::Toast => "toast",
            Self::Palette => "palette",
            Self::Welcome => "welcome",
        }
    }

    /// The overlay called `name`.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|overlay| overlay.name() == name)
    }
}

/// Which regions the layout and the toggles let be drawn. A region let be
/// drawn may still have nothing to show: the sidebar while it is closed,
/// the dock with nothing selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag a region, which is what a renderer reads"
)]
pub struct Regions {
    /// The tab row.
    pub tabs: bool,
    /// The tools.
    pub toolbar: bool,
    /// The dock.
    pub dock: bool,
    /// The sidebar.
    pub sidebar: bool,
    /// The right panel.
    pub chat: bool,
}

impl Regions {
    /// Whether `region` may be drawn.
    pub const fn shows(self, region: Region) -> bool {
        match region {
            Region::Tabs => self.tabs,
            Region::Toolbar => self.toolbar,
            Region::Dock => self.dock,
            Region::Sidebar => self.sidebar,
            Region::Chat => self.chat,
        }
    }
}

/// How far in from each edge of the viewport the canvas is seen, in logical
/// pixels: what the regions drawn over it cover.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    /// From the left edge.
    pub left: f32,
    /// From the top edge.
    pub top: f32,
    /// From the right edge. The right panel sits beside the viewport, not
    /// over it, so nothing covers this edge today.
    pub right: f32,
    /// From the bottom edge.
    pub bottom: f32,
}

/// What the app shows now that decides how much a region covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the independent facts the insets are made of"
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
    /// tab row. On the canvas the bar comes and goes over the canvas and
    /// covers nothing.
    pub item_dock: bool,
}

impl Shown {
    /// What `app` shows now.
    pub fn of(app: &App) -> Self {
        Self {
            covers: app.session.panel.chrome_covers,
            sidebar: app.session.sidebar.shown(),
            tools_docked: app.settings.tools == ToolsPlace::Docked,
            item_dock: app.lens().is_some(),
        }
    }
}

/// The insets of `regions` when the app shows `shown`.
pub fn insets(regions: Regions, shown: Shown) -> Insets {
    let room = |drawn: bool, size: f32| if drawn { size } else { 0.0 };
    let chrome = shown.covers;
    Insets {
        left: room(regions.sidebar && shown.sidebar, SIDEBAR_WIDTH)
            + room(
                chrome && regions.toolbar && shown.tools_docked,
                TOOLS_DOCK_WIDTH,
            ),
        top: room(chrome && regions.tabs, TAB_ROW)
            + room(chrome && regions.dock && shown.item_dock, DOCK_ROW),
        right: 0.0,
        bottom: 0.0,
    }
}

/// The layout as the file has it, and what was toggled over it.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ChromeState {
    /// The last good layout file, or the shipped one.
    pub(crate) layout: Layout,
    /// What was toggled at run time.
    pub(crate) toggles: Toggles,
}

impl ChromeState {
    /// Whether `id`, a region's name or a control's, is hidden: as it was
    /// toggled, else as the layout has it.
    pub(crate) fn hidden(&self, id: &str) -> bool {
        self.toggles
            .get(id)
            .unwrap_or_else(|| self.layout.hides(id))
    }

    /// Which regions may be drawn.
    pub(crate) fn regions(&self) -> Regions {
        let drawn = |region: Region| !self.hidden(region.name());
        Regions {
            tabs: drawn(Region::Tabs),
            toolbar: drawn(Region::Toolbar),
            dock: drawn(Region::Dock),
            sidebar: drawn(Region::Sidebar),
            chat: drawn(Region::Chat),
        }
    }
}

impl App {
    /// Which regions the layout and the toggles let be drawn.
    pub fn regions(&self) -> Regions {
        self.chrome.regions()
    }

    /// How far in from each edge the canvas is seen. Everything that
    /// centres, fits, clips or clamps to the viewport reads this one answer.
    pub fn insets(&self) -> Insets {
        insets(self.regions(), Shown::of(self))
    }

    /// The layout in force: the last good file, or the shipped one.
    pub fn layout(&self) -> &Layout {
        &self.chrome.layout
    }

    /// What was toggled over the layout at run time.
    pub fn layout_toggles(&self) -> &Toggles {
        &self.chrome.toggles
    }

    /// Whether the region or control `id` is hidden now.
    pub fn is_hidden(&self, id: &str) -> bool {
        self.chrome.hidden(id)
    }
}
