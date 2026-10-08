//! Emoji inside a run: which stretches of the text they are, and the size
//! they are set at.
//!
//! The shaper sets a colour bitmap glyph at 0.8 of the font size with a
//! one-em advance, so emoji came out a fifth smaller than the text beside
//! them. macOS draws the same glyphs a full em wide from 24 px up and
//! larger below that, levelling off at 1.25 em from 16 px down, with about
//! a pixel of tracking. Emoji are cut out of the run and set to that curve,
//! so they are the size a browser draws them and the caret stops agree.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

/// The font emoji are asked for by name, so a character that also has a
/// text glyph draws in colour once a variation selector asks for it.
pub(crate) const FAMILY: &str = "Apple Color Emoji";

/// The ink of a colour glyph as a fraction of the size it is shaped at.
const NATURAL_INK: f32 = 0.8;
/// At and below this size an emoji is [`SMALL_INK`] ems wide.
const SMALL_SIZE: f32 = 16.0;
const SMALL_INK: f32 = 1.25;
/// At and above this size an emoji is one em wide.
const LARGE_SIZE: f32 = 24.0;
/// The tracking after an emoji, in px, which is gone by this size.
const TRACKING: f32 = 1.0;
const TRACKING_END: f32 = 28.0;

/// How to set an emoji in text of one size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Setting {
    /// The font size to shape the emoji at.
    pub(crate) size: f32,
    /// Letter spacing, in ems of [`size`](Self::size), that brings the
    /// advance to the emoji's width plus its tracking.
    pub(crate) spacing: f32,
}

/// The setting for emoji in text of `size`.
pub(crate) fn setting(size: f32) -> Setting {
    let ink = ink(size);
    let shaped = ink / NATURAL_INK;
    let tracking = TRACKING * ((TRACKING_END - size) / (TRACKING_END - LARGE_SIZE)).clamp(0.0, 1.0);
    Setting {
        size: shaped,
        // The shaped advance is one em of the shaped size.
        spacing: (ink + tracking) / shaped - 1.0,
    }
}

/// The width an emoji is drawn at, in px, in text of `size`.
fn ink(size: f32) -> f32 {
    if size <= SMALL_SIZE {
        size * SMALL_INK
    } else if size < LARGE_SIZE {
        // From 20 px wide at 16 to 24 px wide at 24.
        SMALL_SIZE * SMALL_INK + (size - SMALL_SIZE) / 2.0
    } else {
        size
    }
}

/// The byte ranges of `text` that are emoji, in order. Neighbouring emoji
/// are one range.
pub(crate) fn ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for (start, cluster) in text.grapheme_indices(true) {
        if !is_emoji(cluster) {
            continue;
        }
        let end = start + cluster.len();
        match ranges.last_mut() {
            Some(last) if last.end == start => last.end = end,
            _ => ranges.push(start..end),
        }
    }
    ranges
}

/// Whether a grapheme cluster is drawn as an emoji: it has a character that
/// is an emoji by default, or a variation selector asking for one.
fn is_emoji(cluster: &str) -> bool {
    cluster
        .chars()
        .any(|c| c == '\u{FE0F}' || matches!(u32::from(c), 0x1F000..=0x1FAFF) || presents(c))
}

/// The characters below U+1F000 that are emoji with no variation selector
/// (`Emoji_Presentation` in the Unicode emoji data).
fn presents(c: char) -> bool {
    matches!(
        u32::from(c),
        0x231A..=0x231B
            | 0x23E9..=0x23EC
            | 0x23F0
            | 0x23F3
            | 0x25FD..=0x25FE
            | 0x2614..=0x2615
            | 0x2648..=0x2653
            | 0x267F
            | 0x2693
            | 0x26A1
            | 0x26AA..=0x26AB
            | 0x26BD..=0x26BE
            | 0x26C4..=0x26C5
            | 0x26CE
            | 0x26D4
            | 0x26EA
            | 0x26F2..=0x26F3
            | 0x26F5
            | 0x26FA
            | 0x26FD
            | 0x2705
            | 0x270A..=0x270B
            | 0x2728
            | 0x274C
            | 0x274E
            | 0x2753..=0x2755
            | 0x2757
            | 0x2795..=0x2797
            | 0x27B0
            | 0x27BF
            | 0x2B1B..=0x2B1C
            | 0x2B50
            | 0x2B55
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_are_found_as_whole_clusters_and_neighbours_join() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let text = format!("a\u{1F389}\u{1F680} b{family}c");
        let found = ranges(&text);
        assert_eq!(found, [1..9, 11..11 + family.len()]);
    }

    #[test]
    fn a_variation_selector_makes_an_emoji_of_a_text_character() {
        assert_eq!(ranges("\u{2764}"), []);
        let whole = |text: &str| ranges(text).first() == Some(&(0..text.len()));
        assert!(whole("\u{2764}\u{FE0F}"));
        // A star is an emoji with no selector.
        assert!(whole("\u{2B50}"));
    }

    /// The width of the glyph's ink and its advance, in px.
    fn drawn(size: f32) -> (f32, f32) {
        let set = setting(size);
        (set.size * NATURAL_INK, set.size * (1.0 + set.spacing))
    }

    #[test]
    fn the_size_follows_what_macos_draws() {
        let close = |a: f32, b: f32| (a - b).abs() < 0.01;
        // Measured with CoreText: ink and advance at each size.
        for (size, ink, advance) in [
            (12.0, 15.0, 16.0),
            (16.0, 20.0, 21.0),
            (20.0, 22.0, 23.0),
            (24.0, 24.0, 25.0),
            (28.0, 28.0, 28.0),
            (40.0, 40.0, 40.0),
        ] {
            let (drawn_ink, drawn_advance) = drawn(size);
            assert!(close(drawn_ink, ink), "{size}: ink {drawn_ink}");
            assert!(
                close(drawn_advance, advance),
                "{size}: advance {drawn_advance}"
            );
        }
    }
}
