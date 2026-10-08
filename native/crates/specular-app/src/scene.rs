//! What to put on the canvas at startup.

use std::path::Path;

use anyhow::Context as _;
use specular_doc::{
    Annotation, AnnotationId, Command, Document, Entity, Kind, Page, PageAnchor, Rect, RegionAnchor,
};
use specular_interact::{PagePlacement, region_annotation};

/// Pages in the demo grid when neither a `.canvas` nor `--pages` is given
/// (ADR 0038 measured 9/20/40).
pub(crate) const DEMO_PAGE_COUNT: usize = 9;
const DEMO_URL: &str = "https://example.com/";

/// The document to open: a `.canvas` file, or a demo grid of `demo_pages`,
/// plus `annotations` seeded comment regions.
pub(crate) fn load_document(
    path: Option<&Path>,
    demo_pages: usize,
    annotations: usize,
) -> anyhow::Result<Document> {
    let mut document = match path {
        Some(path) => {
            let json = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            Document::from_canvas_str(&json)
                .with_context(|| format!("parsing {}", path.display()))?
        }
        None => document_of(demo_grid(demo_pages))?,
    };
    let seeds = seed_annotations(annotations, &document);
    let first = document.annotations().len();
    let commands = seeds
        .into_iter()
        .enumerate()
        .map(|(index, annotation)| Command::InsertAnnotation {
            annotation: Box::new(annotation),
            at: first + index,
        })
        .collect();
    document
        .apply(Command::Batch(commands))
        .context("seeding annotations")?;
    Ok(document)
}

/// A document holding `entities` back-to-front.
pub(crate) fn document_of(entities: Vec<Entity>) -> anyhow::Result<Document> {
    let mut document = Document::new();
    let commands = entities
        .into_iter()
        .enumerate()
        .map(|(at, entity)| Command::InsertEntity {
            entity: Box::new(entity),
            at,
        })
        .collect();
    document
        .apply(Command::Batch(commands))
        .context("building the document")?;
    Ok(document)
}

/// A page entity showing `url`.
pub(crate) fn page_entity(id: &str, url: &str, rect: Rect) -> Entity {
    let page = Page {
        url: url.to_owned(),
        ..Page::default()
    };
    Entity::new(id, rect, Kind::Page(page))
}

/// `count` laptop-sized pages in rows of five, like a multi-page board.
fn demo_grid(count: usize) -> Vec<Entity> {
    const COLUMNS: usize = 5;
    const WIDTH: f64 = 1280.0;
    const HEIGHT: f64 = 800.0;
    const GAP: f64 = 80.0;
    (0..count)
        .map(|index| {
            let column = (index % COLUMNS) as f64;
            let row = (index / COLUMNS) as f64;
            let rect = Rect::new(column * (WIDTH + GAP), row * (HEIGHT + GAP), WIDTH, HEIGHT);
            page_entity(&format!("demo-{index}"), DEMO_URL, rect)
        })
        .collect()
}

/// `count` comment regions spread over the pages of `document`, round-robin,
/// each in its page's document. Position and size vary by index arithmetic
/// alone so a benchmark draws the same UI every run.
fn seed_annotations(count: usize, document: &Document) -> Vec<Annotation> {
    let pages: Vec<(&Entity, &Page)> = document
        .entities()
        .filter_map(|entity| match &entity.kind {
            Kind::Page(page) => Some((entity, page)),
            Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {
                None
            }
        })
        .collect();
    if pages.is_empty() {
        return Vec::new();
    }
    (0..count)
        .map(|index| {
            let (entity, page) = pages[index % pages.len()];
            let viewport = PagePlacement::viewport_for(entity.rect);
            let (width, height) = (f64::from(viewport.width), f64::from(viewport.height));
            // Offsets top out at 0.77 / 0.68 and sizes at 0.20 / 0.14 of the
            // viewport, so every region stays inside its page.
            let doc_rect = Rect::new(
                (0.05 + 0.08 * ((index * 7) % 10) as f64) * width,
                (0.05 + 0.07 * ((index * 3 + 1) % 10) as f64) * height,
                (0.12 + 0.02 * (index % 5) as f64) * width,
                (0.08 + 0.02 * ((index / 2) % 4) as f64) * height,
            );
            let page_anchor = PageAnchor {
                page_url: Some(page.url.clone()),
                ..PageAnchor::new(entity.id.clone())
            };
            region_annotation(
                AnnotationId::new(format!("seed-{index}")),
                "1970-01-01T00:00:00.000Z".to_owned(),
                RegionAnchor::Document { doc_rect },
                Some(page_anchor),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use specular_interact::{App, Event, region_on_canvas, update};

    use super::*;

    /// The canvas rect of every region seeded over a demo grid, with the id
    /// of its page.
    fn seeded(pages: usize, count: usize) -> Vec<(String, Rect)> {
        let mut app = App::new(0);
        let document = load_document(None, pages, count).unwrap();
        update(&mut app, Event::DocumentOpened(Box::new(document)));
        app.document()
            .annotations()
            .iter()
            .map(|note| {
                let page = note.page_anchor.as_ref().unwrap().page_id.to_string();
                (page, region_on_canvas(&app, note).unwrap())
            })
            .collect()
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    #[test]
    fn bench_fixtures_load_with_their_page_counts() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let counts: Vec<usize> = ["static-9", "static-20", "static-40", "animated-20", "input"]
            .iter()
            .map(|name| {
                let path = dir.join(format!("{name}.canvas"));
                let mut app = App::new(0);
                let document = load_document(Some(&path), 0, 0).unwrap();
                update(&mut app, Event::DocumentOpened(Box::new(document)));
                app.pages().count()
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
            .any(|(i, a)| pages[i + 1..].iter().any(|b| overlaps(a.rect, b.rect)));
        assert!(!overlapping);
    }

    #[test]
    fn pages_keep_their_order_as_the_stack_order() {
        let document = load_document(None, 3, 0).unwrap();
        assert_eq!(document.entities().count(), 3);
        let ids: Vec<_> = document.entities().map(|page| page.id.as_str()).collect();
        assert_eq!(ids, ["demo-0", "demo-1", "demo-2"]);
    }

    #[test]
    fn seed_spreads_round_robin_over_pages() {
        let owners: Vec<_> = seeded(2, 5).into_iter().map(|(page, _)| page).collect();
        assert_eq!(owners, ["demo-0", "demo-1", "demo-0", "demo-1", "demo-0"]);
    }

    #[test]
    fn seeded_regions_stay_inside_their_page() {
        let page = demo_grid(1)[0].rect;
        let outside = seeded(1, 200).iter().any(|(_, region)| {
            region.x < page.x
                || region.y < page.y
                || region.x + region.width > page.x + page.width
                || region.y + region.height > page.y + page.height
        });
        assert!(!outside);
    }

    #[test]
    fn seed_is_deterministic_and_varied() {
        let (a, b) = (seeded(1, 6), seeded(1, 6));
        assert_eq!(a, b);
        assert_ne!(a[0], a[1]);
    }
}
