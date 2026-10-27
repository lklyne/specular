//! The synthetic page's grid of cells: what a point or a rect hits, and what an inspect reads.

use glam::Vec2;

use crate::geometry::{CssRect, CssSize, PixelRect};
use crate::source::{CapturedElement, ElementPlace, InspectedNode, PageElement};

/// The size of one cell of the grid a synthetic page answers
/// [`query_element`](PageSource::query_element) from, in CSS pixels.
const CELL: (u32, u32) = (160, 48);

/// The element a synthetic page laid out at `viewport` has under `point`:
/// the cell of a 160x48 grid holding it, cut off at the viewport's edge.
/// `None` outside the viewport.
///
/// The frames show no such grid. It stands in for a DOM so that what is
/// built on an element answer runs with no browser, and gives the same
/// answer every time.
pub(crate) fn synthetic_element_at(viewport: CssSize, point: Vec2) -> Option<PageElement> {
    let inside = point.x >= 0.0
        && point.y >= 0.0
        && point.x < viewport.width as f32
        && point.y < viewport.height as f32;
    if !inside {
        return None;
    }
    let (column, row) = (point.x as u32 / CELL.0, point.y as u32 / CELL.1);
    let (x, y) = (column * CELL.0, row * CELL.1);
    Some(PageElement {
        selector: cell_selector(column, row),
        element_path: Some("body > div.cell".to_owned()),
        bounding_box: PixelRect::new(
            x as i32,
            y as i32,
            CELL.0.min(viewport.width - x),
            CELL.1.min(viewport.height - y),
        ),
    })
}

/// The selector of the grid cell at `column` and `row`.
fn cell_selector(column: u32, row: u32) -> String {
    format!("div.cell[data-col=\"{column}\"][data-row=\"{row}\"]")
}

/// The element a synthetic page offers an item centred on the document point
/// `point` to follow: the grid cell holding it, or the body for a point
/// outside the document, which is one viewport wide and three tall.
pub(crate) fn synthetic_capture(viewport: CssSize, point: Vec2) -> CapturedElement {
    let inside = point.x >= 0.0
        && point.y >= 0.0
        && point.x < viewport.width as f32
        && point.y < 3.0 * viewport.height as f32;
    let selector = if inside {
        cell_selector(point.x as u32 / CELL.0, point.y as u32 / CELL.1)
    } else {
        "body".to_owned()
    };
    let place = synthetic_place(&selector).unwrap_or(ElementPlace {
        doc: Vec2::ZERO,
        viewport_positioned: false,
    });
    CapturedElement { selector, place }
}

/// Where the element `selector` names sits in a synthetic page's document:
/// a grid cell's corner, the origin for the body, and `None` for a selector
/// the grid does not write. The grid never reflows, so a place never moves.
pub(crate) fn synthetic_place(selector: &str) -> Option<ElementPlace> {
    let at = |doc| ElementPlace {
        doc,
        viewport_positioned: false,
    };
    if selector == "body" {
        return Some(at(Vec2::ZERO));
    }
    let rest = selector.strip_prefix("div.cell[data-col=\"")?;
    let (column, rest) = rest.split_once("\"][data-row=\"")?;
    let row = rest.strip_suffix("\"]")?;
    let (column, row): (u32, u32) = (column.parse().ok()?, row.parse().ok()?);
    Some(at(Vec2::new(
        (column * CELL.0) as f32,
        (row * CELL.1) as f32,
    )))
}

/// The element a synthetic page scrolled by `scroll` has under the viewport
/// point `point`. The grid is laid out in the document, so its cells move up
/// as the page scrolls down, and the box is where the cell sits in the
/// viewport now.
pub(super) fn element_scrolled(
    viewport: CssSize,
    scroll: Vec2,
    point: Vec2,
) -> Option<PageElement> {
    synthetic_element_at(viewport, point)?;
    let at = point + scroll;
    let (column, row) = (at.x as u32 / CELL.0, at.y as u32 / CELL.1);
    let (x, y) = ((column * CELL.0) as f32, (row * CELL.1) as f32);
    Some(PageElement {
        selector: cell_selector(column, row),
        element_path: Some("body > div.cell".to_owned()),
        bounding_box: PixelRect::new(
            (x - scroll.x).round() as i32,
            (y - scroll.y).round() as i32,
            CELL.0.min(viewport.width.saturating_sub(x as u32)),
            CELL.1,
        ),
    })
}

/// The cell of a synthetic page's grid under the viewport point `point`, as
/// the inspect tool reads it: a `div.cell` named for its column and row, in
/// the styles every cell has.
pub(super) fn inspected_scrolled(
    viewport: CssSize,
    scroll: Vec2,
    point: Vec2,
) -> Option<InspectedNode> {
    let element = element_scrolled(viewport, scroll, point)?;
    let at = point + scroll;
    let (column, row) = (at.x as u32 / CELL.0, at.y as u32 / CELL.1);
    let styles = [
        ("display", "block"),
        ("position", "static"),
        ("font-family", "Inter, sans-serif"),
        ("font-size", "14px"),
        ("font-weight", "400"),
        ("color", "rgb(17, 24, 39)"),
        ("background", "rgb(255, 255, 255)"),
        ("padding", "8px"),
        ("margin", "0px"),
    ];
    Some(InspectedNode {
        node_id: format!("cell-{column}-{row}"),
        tag_name: "div".to_owned(),
        name: format!("div \"Cell {column},{row}\""),
        selector: element.selector,
        id_attribute: Some(format!("cell-{column}-{row}")),
        classes: vec!["cell".to_owned()],
        styles: styles
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect(),
        bounding_box: element.bounding_box,
    })
}

/// The cells of a synthetic page's grid that lie wholly inside `rect`, each
/// as its box, row by row. Both are in the viewport CSS pixels of a page
/// laid out at `viewport` and scrolled by `scroll`. A cell the rect only
/// clips is not grabbed.
pub(crate) fn synthetic_elements_in(
    viewport: CssSize,
    scroll: Vec2,
    rect: CssRect,
) -> Vec<CssRect> {
    let (left, top) = (rect.x + scroll.x, rect.y + scroll.y);
    let (right, bottom) = (left + rect.width, top + rect.height);
    let cells = |from: f32, to: f32, cell: u32, limit: f32| {
        let first = (from.max(0.0) / cell as f32).ceil() as u32;
        let last = (to.min(limit) / cell as f32).floor().max(0.0) as u32;
        first..last
    };
    let document_height = 3.0 * viewport.height as f32;
    let columns = cells(left, right, CELL.0, viewport.width as f32);
    cells(top, bottom, CELL.1, document_height)
        .flat_map(|row| columns.clone().map(move |column| (column, row)))
        .map(|(column, row)| {
            CssRect::new(
                (column * CELL.0) as f32 - scroll.x,
                (row * CELL.1) as f32 - scroll.y,
                CELL.0 as f32,
                CELL.1 as f32,
            )
        })
        .collect()
}
