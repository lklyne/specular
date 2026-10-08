//! The asset source: the Kit's default icons, and the few more the sidebar
//! and the right panel use from the Kit's full catalog.

use std::borrow::Cow;

use gpui_kit::assets::{Assets, IconName, icon_assets};
use gpui_kit::component::IconNamed;
use gpui_kit::{AssetSource, SharedString};

icon_assets!(
    ExtraIcons,
    [
        StickyNote,
        PenLine,
        MessageSquare,
        SquareDashed,
        SquareDashedMousePointer,
        Code,
        Archive,
        Image,
        Zap,
        ListEnd,
        FolderCode,
        Keyboard
    ]
);

/// An icon of the Kit's catalog that is not in its default set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellIcon {
    /// A sticky note or a text.
    StickyNote,
    /// A drawing.
    PenLine,
    /// A comment.
    MessageSquare,
    /// A comment on a region or on the selection.
    SquareDashed,
    /// A turn about the selected items.
    SquareDashedMousePointer,
    /// A turn about a page element.
    Code,
    /// Closing a thread: it is archived, not deleted.
    Archive,
    /// A queued message that is only images.
    Image,
    /// Auto-fix is on for the origin.
    Zap,
    /// Comments for the origin wait in the queue.
    ListEnd,
    /// A connected repo.
    FolderCode,
    /// The shortcuts pane.
    Keyboard,
}

impl IconNamed for ShellIcon {
    fn path(self) -> SharedString {
        match self {
            Self::StickyNote => IconName::StickyNote.path(),
            Self::PenLine => IconName::PenLine.path(),
            Self::MessageSquare => IconName::MessageSquare.path(),
            Self::SquareDashed => IconName::SquareDashed.path(),
            Self::SquareDashedMousePointer => IconName::SquareDashedMousePointer.path(),
            Self::Code => IconName::Code.path(),
            Self::Archive => IconName::Archive.path(),
            Self::Image => IconName::Image.path(),
            Self::Zap => IconName::Zap.path(),
            Self::ListEnd => IconName::ListEnd.path(),
            Self::FolderCode => IconName::FolderCode.path(),
            Self::Keyboard => IconName::Keyboard.path(),
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
