//! The popup of text and sticky notes (`StickyNotePopover.tsx`).

use specular_doc::{Entity, Kind, TextStyle};

use super::super::build::{button, color_dropdown, font_dropdown, groups, size_dropdown};
use super::super::{Align, Control, ControlId, Face, Icon, PaintRole, Palette, PopupModel};
use crate::property::read;
use crate::{Action, Format, Property};

pub(super) fn popup(app: &crate::App, entities: &[&Entity]) -> PopupModel {
    // Storage carries the slot and each text resolves it against its own
    // surface, so a mixed selection takes one pick. The swatches follow the
    // all-plain case: ink in the vivid palette. A sticky among them makes
    // them fills.
    let all_plain = entities.iter().all(|entity| match &entity.kind {
        Kind::Text(text) => text.resolved_style() == TextStyle::Plain,
        Kind::Shape(_) | Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => false,
    });
    let noun = match (entities.len(), all_plain) {
        (1, true) => "text".to_owned(),
        (1, false) => "sticky note".to_owned(),
        (count, _) => format!("{count} text items"),
    };
    let (palette, role) = if all_plain {
        (Palette::Vivid, PaintRole::Ink)
    } else {
        (Palette::Soft, PaintRole::Fill)
    };
    let color = read::color(app);
    let controls = groups(vec![
        vec![
            size_dropdown(
                ControlId::new("text.size"),
                "Set text size",
                read::text_size(app),
                |size| Action::SetProperty(Property::TextSize(size)),
            ),
            font_dropdown(
                ControlId::new("text.font"),
                "Set text font",
                read::text_font(app),
                |font| Action::SetProperty(Property::TextFont(font)),
            ),
        ],
        vec![color_dropdown(
            ControlId::new("text.color"),
            &noun,
            palette,
            role,
            color.as_ref(),
            None,
            |color| Action::SetProperty(Property::Color(color)),
        )],
        formats(app),
    ]);
    PopupModel {
        anchor: super::over(entities, Align::Center),
        controls,
    }
}

/// The formatting buttons of the text being edited: the ones the Electron
/// popup has, for the formats that text takes. Outside an edit there are none.
pub(super) fn formats(app: &crate::App) -> Vec<Control> {
    let Some(edit) = &app.session.editing else {
        return Vec::new();
    };
    let all = [
        (Format::Bold, "bold", "Bold", Icon::Bold),
        (
            Format::Strike,
            "strikethrough",
            "Strikethrough",
            Icon::Strikethrough,
        ),
        (
            Format::BulletList,
            "bullets",
            "Bullet list",
            Icon::BulletList,
        ),
    ];
    (all.into_iter())
        .filter(|(format, ..)| edit.takes(*format))
        .map(|(format, name, label, icon)| {
            button(
                ControlId::new("format").child(name),
                label,
                Face::icon(icon),
                Action::Format(format),
            )
        })
        .collect()
}
