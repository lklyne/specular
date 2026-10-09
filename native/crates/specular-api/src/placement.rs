//! Where new things go: `/layout/find-placement`, `/layout/batch-placement`
//! and `/layout/apply-directive`. The CLI asks these before every create
//! that names no position, then sends the positions in its patch. They read
//! the canvas and change nothing.
//!
//! The math is the Electron app's (`workspace-placement.ts`,
//! `layout-math.ts`). The free spot itself is found by
//! `specular_interact::free_spot`, which a new page tab also asks.

pub(crate) mod arrange;
mod layout;

use serde_json::{Value, json};
use specular_doc::EntityId;
use specular_interact::App;
use specular_interact::free_spot::{self, place, snap};

use self::layout::{Mode, Size};
use crate::{Response, Step};

/// The room kept round occupied space, and the default gap between items.
const GUTTER: f64 = free_spot::GUTTER;

fn beside_selection(body: &Value) -> bool {
    body.get("anchor").and_then(Value::as_str) != Some("empty_region")
}

fn positions_json(positions: &[(f64, f64)], insets: &[(f64, f64)]) -> Vec<Value> {
    (positions.iter().zip(insets))
        .map(|((x, y), (inset_x, inset_y))| {
            json!({ "canvasX": x + inset_x, "canvasY": y + inset_y })
        })
        .collect()
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

/// A free spot for a `width` by `height` box, honouring the `anchor` of
/// `body`, as `/layout/find-placement` answers it.
pub(crate) fn locate(app: &App, width: f64, height: f64, body: &Value) -> Value {
    let spot = place(app, width, height, beside_selection(body));
    json!({
        "canvasX": spot.x,
        "canvasY": spot.y,
        "fallbackUsed": spot.fallback,
        "reason": spot.reason,
    })
}

/// `POST /layout/find-placement`.
pub(crate) fn find(app: &App, body: &Value) -> Step {
    let width = number(body, "width").unwrap_or(800.0);
    let height = number(body, "height").unwrap_or(600.0);
    Step::Answer(locate(app, width, height, body))
}

fn items(body: &Value) -> Result<&[Value], Response> {
    (body.get("items").and_then(Value::as_array))
        .map(Vec::as_slice)
        .ok_or_else(|| Response::bad_request("items: expected an array"))
}

/// `POST /layout/batch-placement`: one free spot for the whole batch, with
/// the items laid out in it.
pub(crate) fn batch(app: &App, body: &Value) -> Result<Step, Response> {
    let items = items(body)?;
    let mode = match body.get("layout").and_then(Value::as_str) {
        None => Mode::Row,
        Some(name) => Mode::parse(name)
            .ok_or_else(|| Response::bad_request(format!("layout: unknown layout '{name}'")))?,
    };
    let gap = snap(number(body, "gap").unwrap_or(GUTTER));
    let sizes: Vec<Size> = (items.iter())
        .map(|item| Size {
            width: snap(number(item, "width").unwrap_or(0.0)),
            height: snap(number(item, "height").unwrap_or(0.0)),
        })
        .collect();
    let insets: Vec<(f64, f64)> = (items.iter())
        .map(|item| {
            (
                number(item, "insetX").unwrap_or(0.0),
                number(item, "insetY").unwrap_or(0.0),
            )
        })
        .collect();
    if sizes.is_empty() {
        return Ok(Step::Answer(json!({ "positions": [] })));
    }
    let bounds = layout::bounds(&sizes, mode, gap, gap, None);
    let spot = place(app, bounds.width, bounds.height, beside_selection(body));
    let positions = layout::positions(&sizes, mode, gap, gap, (spot.x, spot.y), None);
    Ok(Step::Answer(
        json!({ "positions": positions_json(&positions, &insets) }),
    ))
}

/// `POST /layout/apply-directive`: positions for new items and for existing
/// ones being rearranged, from a `{kind, gap, cols, originX, originY,
/// near}` directive.
pub(crate) fn directive(app: &App, body: &Value) -> Result<Step, Response> {
    let directive = &body["layout"];
    let mode = layout::validate(directive).map_err(Response::bad_request)?;
    let gap = layout::spacing(&directive["gap"], GUTTER);
    let col_gap = layout::spacing(&directive["colGap"], gap);
    let row_gap = layout::spacing(&directive["rowGap"], gap);
    let cols = (directive["cols"].as_u64()).and_then(|cols| usize::try_from(cols).ok());

    let document = app.document();
    let mut sizes = Vec::new();
    let mut insets = Vec::new();
    let mut kinds = Vec::new();
    let mut existing: Option<(f64, f64)> = None;
    for (index, item) in items(body)?.iter().enumerate() {
        if let Some(id) = item.get("id").and_then(Value::as_str) {
            let entity = document.entity(&EntityId::from(id)).ok_or_else(|| {
                Response::bad_request(format!(
                    "applyLayoutDirective: unknown entity id \"{id}\" at index {index}"
                ))
            })?;
            let rect = entity.rect;
            sizes.push(Size {
                width: number(item, "width").unwrap_or(rect.width),
                height: number(item, "height").unwrap_or(rect.height),
            });
            insets.push((0.0, 0.0));
            kinds.push(json!(entity.kind.name()));
            existing =
                Some(existing.map_or((rect.x, rect.y), |(x, y)| (x.min(rect.x), y.min(rect.y))));
            continue;
        }
        let (Some(width), Some(height)) = (number(item, "width"), number(item, "height")) else {
            return Err(Response::bad_request(format!(
                "applyLayoutDirective: item at index {index} has no id and no width/height"
            )));
        };
        sizes.push(Size { width, height });
        insets.push((
            number(item, "insetX").unwrap_or(0.0),
            number(item, "insetY").unwrap_or(0.0),
        ));
        kinds.push(Value::Null);
    }
    let Some(first_inset) = insets.first().copied() else {
        return Ok(Step::Answer(json!({ "positions": [], "kinds": [] })));
    };

    let given = number(directive, "originX").zip(number(directive, "originY"));
    let origin = if let Some((x, y)) = given {
        // The origin names where the first item's own corner goes.
        (x - first_inset.0, y - first_inset.1)
    } else if let Some(near) = directive["near"].as_str() {
        let near = (document.entity(&EntityId::from(near)))
            .map(|entity| entity.rect)
            .ok_or_else(|| {
                Response::bad_request(format!(
                    "applyLayoutDirective: near entity \"{near}\" not found"
                ))
            })?;
        match mode {
            Mode::Column => (near.x, near.y + near.height + row_gap),
            Mode::Row | Mode::Grid => (near.x + near.width + col_gap, near.y),
        }
    } else if let Some(corner) = existing {
        corner
    } else {
        let bounds = layout::bounds(&sizes, mode, col_gap, row_gap, cols);
        let spot = place(app, bounds.width, bounds.height, true);
        (spot.x, spot.y)
    };
    let positions = layout::positions(&sizes, mode, col_gap, row_gap, origin, cols);
    Ok(Step::Answer(json!({
        "positions": positions_json(&positions, &insets),
        "kinds": kinds,
    })))
}
