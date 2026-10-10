//! One row of the command palette: the label with the characters the query
//! matched in a heavier weight, a check mark when the command is on, and its
//! key at the right.

use std::ops::Range;

use gpui_kit::component::{Icon, IconName, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    FontWeight, HighlightStyle, IntoElement, ParentElement as _, SharedString, Styled as _,
    StyledText, div, px,
};
use specular_interact::PaletteItem;

use crate::theme;

/// The byte ranges of `label` that hold the characters at `positions`
/// (ascending character indices), adjacent ones joined.
fn ranges(label: &str, positions: &[usize]) -> Vec<Range<usize>> {
    let mut found: Vec<Range<usize>> = Vec::new();
    let mut at = positions.iter().copied().peekable();
    for (index, (start, character)) in label.char_indices().enumerate() {
        if at.next_if_eq(&index).is_none() {
            continue;
        }
        let end = start + character.len_utf8();
        match found.last_mut() {
            Some(last) if last.end == start => last.end = end,
            _ => found.push(start..end),
        }
    }
    found
}

/// The row for `item`, with `positions` of its label emphasized.
pub(super) fn row(item: &PaletteItem, positions: &[usize]) -> impl IntoElement + use<> {
    let label = SharedString::from(item.label.to_string());
    let emphasis = HighlightStyle {
        font_weight: Some(FontWeight::SEMIBOLD),
        ..HighlightStyle::default()
    };
    let highlights = ranges(&label, positions)
        .into_iter()
        .map(|range| (range, emphasis));
    let keys = item.chord.map(|chord| SharedString::from(chord.text()));
    let muted = theme::tinted(theme::text_muted());
    h_flex()
        .w_full()
        .gap_6()
        .items_center()
        .justify_between()
        .text_size(px(13.0))
        .when(!item.enabled, |row| row.text_color(muted))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(StyledText::new(label.clone()).with_highlights(highlights)),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .when(item.checked == Some(true), |keys| {
                    keys.child(Icon::new(IconName::Check).small())
                })
                .children(keys.map(|keys| {
                    div()
                        .text_color(muted)
                        .font_family(specular_compositor::MONO_FAMILY)
                        .child(keys)
                })),
        )
}

#[cfg(test)]
mod tests {
    use super::ranges;

    #[test]
    fn matched_characters_join_into_byte_ranges() {
        assert_eq!(ranges("Bold", &[0, 1, 3]), vec![0..2, 3..4]);
        assert_eq!(ranges("Bold", &[]), Vec::<std::ops::Range<usize>>::new());
        // Positions count characters, ranges count bytes.
        assert_eq!(ranges("é Bold", &[2, 3]), vec![3..5]);
    }
}
