//! [`Format`]: the markdown formatting a key or a menu item toggles on the
//! selection of the text being edited, and which texts take which.

use super::buffer::{Target, TextEdit};
use super::format::{self, ListKind, Wrap};
use crate::App;

/// One kind of markdown formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// `**strong**`.
    Bold,
    /// `*emphasis*`.
    Italic,
    /// `` `code` ``.
    Code,
    /// `~~struck~~`.
    Strike,
    /// A heading of this level, 1 to 6, on the lines the selection spans.
    /// 0 makes them body text.
    Heading(u8),
    /// `- item` on the lines the selection spans.
    BulletList,
    /// `1. item` on the lines the selection spans.
    NumberedList,
    /// `- [ ] item` on the lines the selection spans.
    TaskList,
}

impl Format {
    /// Whether a text of this kind takes the format. A text or a sticky has
    /// the formats the Electron sticky has, a Document has them all, and a
    /// shape's label has none.
    const fn applies_to(self, target: Target) -> bool {
        match target {
            Target::Label | Target::Title | Target::EdgeLabel | Target::Comment => false,
            Target::Note => true,
            Target::Text => matches!(
                self,
                Self::Bold | Self::Italic | Self::Strike | Self::BulletList
            ),
        }
    }
}

impl TextEdit {
    /// Whether the text being edited takes `format`.
    pub(crate) const fn takes(&self, format: Format) -> bool {
        format.applies_to(self.target)
    }

    /// Whether the popup of the item being edited stays up during the edit:
    /// the item's text and a Document's source have formatting to show, and
    /// an edge's label belongs to a popup the edit does not cover. A shape's
    /// label and an item's title are edited in place with the popup down.
    pub(crate) const fn keeps_popup(&self) -> bool {
        match self.target {
            Target::Text | Target::Note | Target::EdgeLabel => true,
            Target::Label | Target::Title | Target::Comment => false,
        }
    }
}

/// Toggles `format` on the selection of the text being edited.
pub(crate) fn run(app: &mut App, format: Format) {
    let Some(edit) = &mut app.session.editing else {
        return;
    };
    if edit.composition.is_some() || !format.applies_to(edit.target) {
        return;
    }
    let changed = match format {
        Format::Bold => format::toggle_wrap(edit, Wrap::Bold),
        Format::Italic => format::toggle_wrap(edit, Wrap::Italic),
        Format::Code => format::toggle_wrap(edit, Wrap::Code),
        Format::Strike => format::toggle_wrap(edit, Wrap::Strike),
        Format::Heading(level) => format::set_heading(edit, level),
        Format::BulletList => format::toggle_list(edit, ListKind::Bullet),
        Format::NumberedList => format::toggle_list(edit, ListKind::Numbered),
        Format::TaskList => format::toggle_list(edit, ListKind::Task),
    };
    if changed {
        super::refit(app);
    }
}
