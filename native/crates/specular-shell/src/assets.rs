//! The asset source: the Kit's default icons, and the few more the sidebar
//! uses from the Kit's full catalog.

use std::borrow::Cow;

use gpui_kit::assets::{Assets, IconName, icon_assets};
use gpui_kit::component::IconNamed;
use gpui_kit::{AssetSource, SharedString};

icon_assets!(ExtraIcons, [StickyNote, PenLine, MessageSquare]);

/// An icon of the Kit's catalog that is not in its default set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellIcon {
    /// A sticky note or a text.
    StickyNote,
    /// A drawing.
    PenLine,
    /// A comment.
    MessageSquare,
}

impl IconNamed for ShellIcon {
    fn path(self) -> SharedString {
        match self {
            Self::StickyNote => IconName::StickyNote.path(),
            Self::PenLine => IconName::PenLine.path(),
            Self::MessageSquare => IconName::MessageSquare.path(),
        }
    }
}

/// The Kit's default assets, then the extra icons.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShellAssets;

impl AssetSource for ShellAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if let Ok(Some(found)) = ExtraIcons.load(path) {
            return Ok(Some(found));
        }
        Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        Assets.list(path)
    }
}
