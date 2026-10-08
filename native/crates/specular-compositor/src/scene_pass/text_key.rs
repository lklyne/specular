//! A key for each item of a text batch: everything its glyph quads depend
//! on, in the item's own space. Two frames whose keys match lay out the
//! same quads, wherever the camera is.

use std::hash::{Hash, Hasher};

use rustc_hash::FxHasher;
use specular_scene::{ColumnDraw, Rect, RowRule, RuleHeight, TextRun};

use super::text_areas::{TextDraw, TextItem};
use super::text_layout::shaping_hash;

/// The key of one item of a batch.
pub(crate) fn member_key(draw: &TextDraw<'_>) -> u64 {
    let mut hasher = FxHasher::default();
    draw.opacity.to_bits().hash(&mut hasher);
    hash_rect(draw.clip, &mut hasher);
    match draw.text {
        TextItem::Run(run) => {
            0_u8.hash(&mut hasher);
            hash_run(run, &mut hasher);
        }
        TextItem::Column(column) => {
            1_u8.hash(&mut hasher);
            hash_column(column, &mut hasher);
        }
    }
    hasher.finish()
}

fn hash_rect(rect: Option<Rect>, hasher: &mut FxHasher) {
    rect.map(|rect| [rect.x, rect.y, rect.width, rect.height].map(f32::to_bits))
        .hash(hasher);
}

/// A run's shaping, and where and in what colour it is placed.
fn hash_run(run: &TextRun, hasher: &mut FxHasher) {
    shaping_hash(run).hash(hasher);
    [run.origin.x, run.origin.y].map(f32::to_bits).hash(hasher);
    run.box_height.map(f32::to_bits).hash(hasher);
    run.vertical_align.hash(hasher);
    run.color.hash(hasher);
}

fn hash_column(column: &ColumnDraw, hasher: &mut FxHasher) {
    let ColumnDraw {
        origin,
        width,
        height,
        scroll,
        rows,
        owner,
    } = column;
    [origin.x, origin.y, *width, *height, *scroll]
        .map(f32::to_bits)
        .hash(hasher);
    owner.hash(hasher);
    rows.len().hash(hasher);
    for row in rows {
        [row.gap, row.min_height, row.bottom_padding]
            .map(f32::to_bits)
            .hash(hasher);
        row.cells.len().hash(hasher);
        for cell in &row.cells {
            hash_run(cell, hasher);
        }
        row.rules.len().hash(hasher);
        for rule in &row.rules {
            hash_rule(rule, hasher);
        }
    }
}

fn hash_rule(rule: &RowRule, hasher: &mut FxHasher) {
    [rule.x, rule.width].map(f32::to_bits).hash(hasher);
    rule.color.hash(hasher);
    match rule.height {
        RuleHeight::Row => 0_u32.hash(hasher),
        RuleHeight::RowAndGap => 1_u32.hash(hasher),
        RuleHeight::Middle(height) => (2_u32, height.to_bits()).hash(hasher),
        RuleHeight::Bottom(height) => (3_u32, height.to_bits()).hash(hasher),
    }
}

#[cfg(test)]
mod tests {
    use specular_scene::{Color, Point, Row};

    use super::*;

    fn run(text: &str) -> TextRun {
        TextRun::new(text, Point::new(10.0, 20.0), 14.0, Color::BLACK)
    }

    fn key_of(run: &TextRun, clip: Option<Rect>, opacity: f32) -> u64 {
        member_key(&TextDraw {
            text: TextItem::Run(run),
            clip,
            opacity,
        })
    }

    #[test]
    fn text_place_colour_clip_and_opacity_each_change_the_key() {
        let base = key_of(&run("note"), None, 1.0);
        let moved = TextRun {
            origin: Point::new(11.0, 20.0),
            ..run("note")
        };
        let recoloured = TextRun {
            color: Color::WHITE,
            ..run("note")
        };
        let clip = Some(Rect::new(0.0, 0.0, 50.0, 50.0));
        for (name, other) in [
            ("text", key_of(&run("notes"), None, 1.0)),
            ("place", key_of(&moved, None, 1.0)),
            ("colour", key_of(&recoloured, None, 1.0)),
            ("clip", key_of(&run("note"), clip, 1.0)),
            ("opacity", key_of(&run("note"), None, 0.5)),
        ] {
            assert_ne!(other, base, "{name}");
        }
    }

    #[test]
    fn a_column_s_key_follows_its_scroll_and_its_cells() {
        let column = |scroll: f32, text: &str| ColumnDraw {
            origin: Point::new(0.0, 0.0),
            width: 200.0,
            height: 300.0,
            scroll,
            rows: vec![Row {
                cells: vec![run(text)],
                ..Row::default()
            }],
            owner: None,
        };
        let key = |column: &ColumnDraw| {
            member_key(&TextDraw {
                text: TextItem::Column(column),
                clip: None,
                opacity: 1.0,
            })
        };
        let base = key(&column(0.0, "a"));
        assert_eq!(base, key(&column(0.0, "a")));
        assert_ne!(base, key(&column(8.0, "a")));
        assert_ne!(base, key(&column(0.0, "b")));
    }
}
