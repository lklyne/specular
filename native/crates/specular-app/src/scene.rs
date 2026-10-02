//! What to put on the canvas at startup.

use std::path::Path;

use anyhow::Context as _;
use specular_core::document::PageNode;
use specular_core::{CanvasDocument, CanvasRect};

/// Pages in the demo grid when neither a `.canvas` nor `--pages` is given
/// (ADR 0038 measured 9/20/40).
pub(crate) const DEMO_PAGE_COUNT: usize = 9;
const DEMO_URL: &str = "https://example.com/";

/// Loads pages from a `.canvas` file, or builds a demo grid of `demo_pages`.
pub(crate) fn load_pages(path: Option<&Path>, demo_pages: usize) -> anyhow::Result<Vec<PageNode>> {
    let Some(path) = path else {
        return Ok(demo_grid(demo_pages));
    };
    let json =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let document = CanvasDocument::from_json_canvas(&json)
        .with_context(|| format!("parsing {}", path.display()))?;
    document
        .pages()
        .with_context(|| format!("reading pages from {}", path.display()))
}

/// `count` laptop-sized pages in rows of five, like a multi-page board.
fn demo_grid(count: usize) -> Vec<PageNode> {
    const COLUMNS: usize = 5;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_grid_has_requested_page_count() {
        assert_eq!(load_pages(None, 20).unwrap().len(), 20);
    }

    #[test]
    fn bench_fixtures_load_with_their_page_counts() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let counts: Vec<usize> = ["static-9", "static-20", "static-40", "animated-20", "input"]
            .iter()
            .map(|name| {
                load_pages(Some(&dir.join(format!("{name}.canvas"))), 0)
                    .unwrap()
                    .len()
            })
            .collect();
        assert_eq!(counts, [9, 20, 40, 20, 1]);
    }

    #[test]
    fn demo_grid_pages_do_not_overlap() {
        let pages = demo_grid(12);
        let overlapping = pages
            .iter()
            .enumerate()
            .any(|(i, a)| pages[i + 1..].iter().any(|b| a.rect.intersects(b.rect)));
        assert!(!overlapping);
    }
}
