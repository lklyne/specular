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
