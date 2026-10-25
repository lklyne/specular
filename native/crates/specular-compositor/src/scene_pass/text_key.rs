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
