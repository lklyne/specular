//! Rows, columns and grids of boxes, and the `layout` directive that asks
//! for one.

use serde_json::Value;

/// How a batch is arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Row,
    Column,
    Grid,
}

impl Mode {
    pub(super) fn parse(name: &str) -> Option<Self> {
        match name {
            "row" => Some(Self::Row),
            "column" => Some(Self::Column),
            "grid" => Some(Self::Grid),
            _ => None,
        }
    }
}

/// A box to place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Size {
    pub(super) width: f64,
    pub(super) height: f64,
}

fn widest(sizes: &[Size]) -> f64 {
    sizes.iter().map(|size| size.width).fold(0.0, f64::max)
}

fn tallest(sizes: &[Size]) -> f64 {
    sizes.iter().map(|size| size.height).fold(0.0, f64::max)
}

/// The columns a grid of `count` boxes has: as asked, else as near square
/// as it goes.
fn grid_cols(count: usize, cols: Option<usize>) -> usize {
    let square = (count as f64).sqrt().ceil() as usize;
    cols.filter(|cols| *cols > 0).unwrap_or(square).max(1)
}

/// The box the whole arrangement fills. A grid's cells are all the size of
/// the largest item.
pub(super) fn bounds(
    sizes: &[Size],
    mode: Mode,
    col_gap: f64,
    row_gap: f64,
    cols: Option<usize>,
) -> Size {
    let gaps = sizes.len().saturating_sub(1) as f64;
    match mode {
        Mode::Row => Size {
            width: sizes.iter().map(|size| size.width).sum::<f64>() + gaps * col_gap,
            height: tallest(sizes),
        },
        Mode::Column => Size {
            width: widest(sizes),
            height: sizes.iter().map(|size| size.height).sum::<f64>() + gaps * row_gap,
        },
        Mode::Grid => {
            let cols = grid_cols(sizes.len(), cols);
            let rows = sizes.len().div_ceil(cols);
            Size {
                width: cols as f64 * widest(sizes) + (cols - 1) as f64 * col_gap,
                height: rows as f64 * tallest(sizes) + rows.saturating_sub(1) as f64 * row_gap,
            }
        }
    }
}

/// Each box's top-left with the arrangement's at `origin`.
pub(super) fn positions(
    sizes: &[Size],
    mode: Mode,
    col_gap: f64,
    row_gap: f64,
    origin: (f64, f64),
    cols: Option<usize>,
) -> Vec<(f64, f64)> {
    let (mut x, mut y) = origin;
    match mode {
        Mode::Row => (sizes.iter())
            .map(|size| {
                let at = (x, origin.1);
                x += size.width + col_gap;
                at
            })
            .collect(),
        Mode::Column => (sizes.iter())
            .map(|size| {
                let at = (origin.0, y);
                y += size.height + row_gap;
                at
            })
            .collect(),
        Mode::Grid => {
            let cols = grid_cols(sizes.len(), cols);
            let (cell_width, cell_height) = (widest(sizes), tallest(sizes));
            (0..sizes.len())
                .map(|index| {
                    (
                        origin.0 + (index % cols) as f64 * (cell_width + col_gap),
                        origin.1 + (index / cols) as f64 * (cell_height + row_gap),
                    )
                })
                .collect()
        }
    }
}

/// The spacing scale: names for gaps that sit on the canvas grid.
const TOKENS: [(&str, f64); 5] = [
    ("xs", 20.0),
    ("s", 40.0),
    ("m", 60.0),
    ("l", 100.0),
    ("xl", 160.0),
];

/// A gap given as pixels or a token, or `fallback` when it is neither.
pub(super) fn spacing(value: &Value, fallback: f64) -> f64 {
    let token = (value.as_str())
        .and_then(|name| TOKENS.iter().find(|(token, _)| *token == name))
        .map(|(_, pixels)| *pixels);
    value.as_f64().or(token).unwrap_or(fallback)
}

fn is_spacing(value: &Value) -> bool {
    value.is_number() || (value.as_str()).is_some_and(|name| TOKENS.iter().any(|(t, _)| *t == name))
}

/// Checks a `layout` directive and returns its mode, or the first problem
/// in the words the CLI's own check uses.
pub(super) fn validate(directive: &Value) -> Result<Mode, String> {
    if !directive.is_object() {
        return Err("layout: expected an object".to_owned());
    }
    let kind = &directive["kind"];
    let mode = kind
        .as_str()
        .and_then(Mode::parse)
        .ok_or_else(|| format!("layout.kind: expected 'row' | 'column' | 'grid', got {kind}"))?;
    for key in ["gap", "rowGap", "colGap"] {
        let value = &directive[key];
        if !value.is_null() && !is_spacing(value) {
            return Err(format!(
                "layout.{key}: expected number or one of xs|s|m|l|xl, got {value}"
            ));
        }
    }
    let cols = &directive["cols"];
    if !cols.is_null() && cols.as_u64().is_none_or(|cols| cols < 1) {
        return Err(format!(
            "layout.cols: expected positive integer, got {cols}"
        ));
    }
    for key in ["originX", "originY"] {
        let value = &directive[key];
        if !value.is_null() && !value.is_number() {
            return Err(format!("layout.{key}: expected number, got {value}"));
        }
    }
    let near = &directive["near"];
    if !near.is_null() && !near.is_string() {
        return Err(format!(
            "layout.near: expected entity id string, got {near}"
        ));
    }
    if directive["originX"].is_null() != directive["originY"].is_null() {
        return Err("layout: originX and originY must be specified together".to_owned());
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const BOXES: [Size; 3] = [
        Size {
            width: 100.0,
            height: 40.0,
        },
        Size {
            width: 60.0,
            height: 80.0,
        },
        Size {
            width: 20.0,
            height: 20.0,
        },
    ];

    #[test]
    fn a_row_runs_left_to_right_with_the_gap_between() {
        let at = positions(&BOXES, Mode::Row, 20.0, 20.0, (10.0, 5.0), None);
        assert_eq!(at, [(10.0, 5.0), (130.0, 5.0), (210.0, 5.0)]);
        let size = bounds(&BOXES, Mode::Row, 20.0, 20.0, None);
        assert_eq!((size.width, size.height), (220.0, 80.0));
    }

    #[test]
    fn a_grid_has_cells_the_size_of_its_largest_item() {
        let at = positions(&BOXES, Mode::Grid, 20.0, 10.0, (0.0, 0.0), None);
        assert_eq!(at, [(0.0, 0.0), (120.0, 0.0), (0.0, 90.0)]);
        let size = bounds(&BOXES, Mode::Grid, 20.0, 10.0, None);
        assert_eq!((size.width, size.height), (220.0, 170.0));
    }

    #[test]
    fn a_directive_is_refused_in_the_cli_s_words() {
        let refused = |directive| validate(&directive).unwrap_err();
        assert_eq!(
            refused(json!({ "kind": "pile" })),
            "layout.kind: expected 'row' | 'column' | 'grid', got \"pile\""
        );
        assert_eq!(
            refused(json!({ "kind": "row", "gap": "huge" })),
            "layout.gap: expected number or one of xs|s|m|l|xl, got \"huge\""
        );
        assert_eq!(
            refused(json!({ "kind": "grid", "originX": 4 })),
            "layout: originX and originY must be specified together"
        );
        assert_eq!(
            validate(&json!({ "kind": "grid", "gap": "m", "cols": 3 })),
            Ok(Mode::Grid)
        );
        assert_eq!(spacing(&json!("m"), 80.0), 60.0);
    }
}
