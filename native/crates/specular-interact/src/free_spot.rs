//! Where a new thing goes when nothing says: a free spot on the canvas,
//! beside the selection or scanning from the canvas's top-left.
//!
//! The math is the Electron app's (`workspace-placement.ts`), with every
//! entity counted as occupied. The API's placement routes answer from it,
//! and so does a new page tab.

use specular_doc::Rect;

use crate::App;

/// The canvas grid.
const GRID: f64 = 20.0;
/// The room kept round occupied space.
pub const GUTTER: f64 = 80.0;
/// How far past the occupied canvas the scan for a free spot goes.
const SCAN_REACH: f64 = 2000.0;

/// `value` on the grid, as the Electron app's placement rounds it.
pub fn snap(value: f64) -> f64 {
    (value / GRID).round() * GRID
}

/// A free spot and how it was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// The spot's left edge.
    pub x: f64,
    /// The spot's top edge.
    pub y: f64,
    /// Whether it is not the spot first tried.
    pub fallback: bool,
    /// How it was found, in the Electron app's words.
    pub reason: &'static str,
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// Whether a `width` by `height` box at `(x, y)` is [`GUTTER`] clear of
/// every entity.
fn is_free(occupied: &[Rect], x: f64, y: f64, width: f64, height: f64) -> bool {
    let candidate = Rect::new(x, y, width, height);
    !occupied.iter().any(|rect| {
        let grown = Rect::new(
            rect.x - GUTTER,
            rect.y - GUTTER,
            rect.width + GUTTER * 2.0,
            rect.height + GUTTER * 2.0,
        );
        overlaps(candidate, grown)
    })
}

/// The first free spot scanning right from the start, then row by row.
fn scan(occupied: &[Rect], width: f64, height: f64, start: (f64, f64)) -> Spot {
    let reach = |far: fn(&Rect) -> f64| {
        (occupied.iter())
            .map(|rect| far(rect) + GUTTER)
            .fold(SCAN_REACH, f64::max)
    };
    let limit_x = reach(|rect| rect.x + rect.width) + width + SCAN_REACH;
    let limit_y = reach(|rect| rect.y + rect.height) + height + SCAN_REACH;
    let (start_x, start_y) = (snap(start.0), snap(start.1));
    let mut y = start_y;
    let mut first_row = true;
    while y <= limit_y {
        let mut x = if first_row { start_x } else { GUTTER };
        first_row = false;
        while x <= limit_x {
            if is_free(occupied, x, y, width, height) {
                return Spot {
                    x,
                    y,
                    fallback: (x, y) != (start_x, start_y),
                    reason: "scan_fit",
                };
            }
            x += GRID;
        }
        y += GRID;
    }
    // Past everything there is always room.
    Spot {
        x: snap(limit_x),
        y: start_y,
        fallback: true,
        reason: "scan_exhausted",
    }
}

/// A free spot for a `width` by `height` box: beside the selection when
/// `beside_selection` and there is one, else the first free spot from the
/// canvas's top-left.
pub fn place(app: &App, width: f64, height: f64, beside_selection: bool) -> Spot {
    let (width, height) = (snap(width), snap(height));
    let occupied: Vec<Rect> = (app.document().entities())
        .map(|entity| entity.rect)
        .collect();
    let anchor = beside_selection
        .then(|| app.selection_scope().bounds)
        .flatten();
    let Some(anchor) = anchor else {
        return scan(&occupied, width, height, (GUTTER, GUTTER));
    };
    let (x, y) = (snap(anchor.x + anchor.width + GUTTER), snap(anchor.y));
    if is_free(&occupied, x, y, width, height) {
        return Spot {
            x,
            y,
            fallback: false,
            reason: "selection_anchor",
        };
    }
    scan(&occupied, width, height, (x, y))
}
