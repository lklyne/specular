//! What to put on the canvas at startup.

use std::path::Path;

use anyhow::Context as _;
use specular_core::document::PageNode;
use specular_core::{CanvasDocument, CanvasRect};

/// Pages in the demo grid when no `.canvas` is given (ADR 0038 measured 9/20/40).
const DEMO_PAGE_COUNT: usize = 9;
const DEMO_URL: &str = "https://example.com/";

/// Loads pages from a `.canvas` file, or builds the demo grid.
pub(crate) fn load_pages(path: Option<&Path>) -> anyhow::Result<Vec<PageNode>> {
    let Some(path) = path else {
        return Ok(demo_grid(DEMO_PAGE_COUNT));
    };
    let json =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let document = CanvasDocument::from_json_canvas(&json)
        .with_context(|| format!("parsing {}", path.display()))?;
    Ok(document.pages())
}

fn demo_grid(count: usize) -> Vec<PageNode> {
    const COLUMNS: usize = 3;
    const WIDTH: f32 = 1280.0;
    const HEIGHT: f32 = 800.0;
    const GAP: f32 = 80.0;
    (0..count)
        .map(|index| {
            let column = (index % COLUMNS) as f32;
            let row = (index / COLUMNS) as f32;
            PageNode {
                node_id: format!("demo-{index}"),
                url: DEMO_URL.to_owned(),
                rect: CanvasRect::new(column * (WIDTH + GAP), row * (HEIGHT + GAP), WIDTH, HEIGHT),
            }
        })
        .collect()
}
