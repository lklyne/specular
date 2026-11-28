//! The window's layouts (ADR 0055, ADR 0059): the tree of the one in
//! force, which of its parts are drawn, and the room they take from the
//! canvas.
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
mod legacy;
mod look;
mod node;
mod solve;
mod state;
mod toggle;

pub use self::layout::{Layout, LayoutError, LayoutFile, RegionNode, SHIPPED, SHIPPED_THEMES};
pub use self::look::{HiddenWord, Look, Resolved, ThemeChoice, ThemeFile, TrafficLights};
pub use self::node::{Border, Content, Material, Node, NodeKind, Padding, Size};
pub use self::solve::{Frame, Gate, Placed, solve};
pub(crate) use self::state::ChromeState;
pub use self::state::{LayoutsRead, Shown, gate, place};
pub use self::toggle::{LayoutAction, RESET_KEYS, Toggles};
pub(crate) use self::toggle::{is_reset, run as run_layout_action};

use crate::App;

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
/// own, which a layout can fill with a mod as a block holds one. Unlike a
/// block, an overlay takes no room from the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    /// The bar of the selection's or the tool's controls, over the
    /// canvas's top edge.
    Dock,
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
    pub const ALL: [Self; 6] = [
        Self::Dock,
        Self::Menu,
        Self::Share,
        Self::Toast,
        Self::Palette,
        Self::Welcome,
    ];

    /// The name the layout file and `ui.toggle` call it by.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dock => "dock",
            Self::Menu => "menu",
            Self::Share => "share",
            Self::Toast => "toast",
            Self::Palette => "palette",
            Self::Welcome => "welcome",
        }
    }

    /// Its place in [`Overlay::ALL`].
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Dock => 0,
            Self::Menu => 1,
            Self::Share => 2,
            Self::Toast => 3,
            Self::Palette => 4,
            Self::Welcome => 5,
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
/// pixels: what the blocks drawn over it cover.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    /// From the left edge.
    pub left: f32,
    /// From the top edge.
    pub top: f32,
    /// From the right edge. What lies right of the canvas sits beside the
    /// viewport, not over it, so nothing covers this edge.
    pub right: f32,
    /// From the bottom edge, which nothing covers either.
    pub bottom: f32,
}

impl App {
    /// Which regions the layout and the toggles let be drawn.
    pub fn regions(&self) -> Regions {
        self.chrome.regions()
    }

    /// How far in from each edge the canvas is seen. Everything that
    /// centres, fits, clips or clamps to the viewport reads this one answer.
    pub fn insets(&self) -> Insets {
        let shown = Shown::of(self);
        let window = solve::Frame::sized(self.session.viewport.x, self.session.viewport.y);
        let mut insets = self.chrome.insets(window, shown);
        if shown.covers && shown.item_dock && self.regions().dock {
            insets.top += DOCK_ROW;
        }
        insets
    }

    /// Whether the sidebar is shown up to the window's top edge, so it, not
    /// the tab row, holds the traffic lights and its own toggle.
    pub fn sidebar_holds_title(&self) -> bool {
        let shown = Shown::of(self);
        let size = self.session.viewport;
        let (width, height) = if size.x > 0.0 && size.y > 0.0 {
            (size.x, size.y)
        } else {
            (1200.0, 800.0)
        };
        let window = solve::Frame::sized(width, height);
        (self.chrome.solve(window, shown).into_iter()).any(|placed| {
            placed.node.is(Region::Sidebar.name())
                && placed.frame.y <= 0.0
                && placed.frame.width > 0.0
                && placed.frame.height > 0.0
        })
    }

    /// The layout in force.
    pub fn layout(&self) -> &Layout {
        self.chrome.layout()
    }

    /// The name of the layout in force.
    pub fn layout_name(&self) -> &str {
        &self.chrome.active
    }

    /// Every layout there is, by name.
    pub fn layouts(&self) -> impl Iterator<Item = (&str, &Layout)> {
        self.chrome
            .layouts
            .iter()
            .map(|(name, layout)| (name.as_str(), layout))
    }

    /// What was toggled over the layout in force at run time.
    pub fn layout_toggles(&self) -> &Toggles {
        self.chrome.toggles()
    }

    /// Whether the node, overlay or control `id` is hidden now.
    pub fn is_hidden(&self, id: &str) -> bool {
        self.chrome.hidden(id)
    }

    /// The look in force: the layout's theme and colours over the theme
    /// files read.
    pub fn look(&self) -> Resolved {
        (self.layout().look).resolve(self.theme.appearance(), &self.chrome.themes)
    }
}
