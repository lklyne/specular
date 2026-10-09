//! Copy, cut and paste on the canvas.
//!
//! Copy puts the selection on the system clipboard as text: a prefix, then
//! the copied entities and the edges between them as a small `.canvas`
//! document. Paste asks the shell for the clipboard and decides from what
//! comes back, in the order `src/main/clipboard-paste.ts` does: copied
//! entities, then an image, then a URL, then any other text.

use glam::DVec2;
use serde_json::json;
use specular_doc::{
    Command, Document, Entity, EntityId, ItemId, Kind, Page, PageSource, Rect, TextStyle,
};

use crate::asset::{self, AssetBytes};
use crate::clone::{self, Orphan, Source};
use crate::scroll_follow::{self, Scrolls};
use crate::{App, Effect, anchor, edit, geometry, grid, place, update, url, verbs};

/// What copied entities start with on the clipboard.
const CLIPBOARD_PREFIX: &str = "specular:canvas:";
/// The viewport a pasted URL's page gets: the Desktop preset.
const PASTED_PAGE_SIZE: DVec2 = DVec2::new(1440.0, 900.0);
const PASTED_PAGE_PRESET: u32 = 7;
const PASTED_PAGE_DEVICE: &str = "desktop";

/// What the shell read from the system clipboard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardContent {
    /// The clipboard as text, if it has any.
    pub text: Option<String>,
    /// The clipboard as an image, if it holds one.
    pub image: Option<ClipboardImage>,
}

/// An image from the clipboard, encoded as a PNG file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// The PNG file's bytes.
    pub png: AssetBytes,
}

/// What a paste makes of a clipboard.
#[derive(Debug, Clone, PartialEq)]
pub enum Paste {
    /// Entities copied from a canvas, and the edges between them.
    Items(Box<Document>),
    /// An image, which becomes a file in `assets/` and a file entity.
    Image(ClipboardImage),
    /// A page showing this URL.
    Page(String),
    /// A sticky note with this text.
    Text(String),
    /// Nothing the canvas can show.
    Nothing,
}

impl Paste {
    /// Decides what pasting `content` makes.
    pub fn of(content: ClipboardContent) -> Self {
        let text = content.text.unwrap_or_default();
        if let Some(document) = copied_items(&text) {
            return Self::Items(Box::new(document));
        }
        if let Some(image) = content.image {
            return Self::Image(image);
        }
        let trimmed = text.trim();
        if trimmed.is_empty() {
            Self::Nothing
        } else if !trimmed.contains('\n') && url::looks_like_url(trimmed) {
            Self::Page(url::normalize_user_url(trimmed))
        } else {
            Self::Text(trimmed.to_owned())
        }
    }
}

/// The entities `text` carries, if it is what [`copy`] wrote.
fn copied_items(text: &str) -> Option<Document> {
    let json = text.strip_prefix(CLIPBOARD_PREFIX)?;
    let document = Document::from_canvas_str(json).ok()?;
    let has_entities = document.entities().next().is_some();
    has_entities.then_some(document)
}

/// Puts the selection on the clipboard: its entities, what is inside its
/// groups and hooked to its pages, and the edges between any two of them.
/// An entity that has moved with its page is written where it is seen, and
/// its anchor names only the page and the document: where the copy lands
/// decides the rest.
pub(crate) fn copy(app: &App, effects: &mut Vec<Effect>) {
    let scope = app.selection_scope();
    let scrolls = Scrolls::of(app);
    let mut copied = Document::new();
    for item in app.document.order() {
        let command = match item {
            ItemId::Entity(id) if scope.holds(id) => {
                app.document.entity(id).map(|entity| Command::InsertEntity {
                    entity: Box::new(as_seen(&scrolls, entity)),
                    at: copied.stack_len(),
                })
            }
            ItemId::Edge(id) => (app.document.edge(id))
                .filter(|edge| scope.holds(&edge.from) && scope.holds(&edge.to))
                .map(|edge| Command::InsertEdge {
                    edge: Box::new(edge.clone()),
                    at: copied.stack_len(),
                }),
            ItemId::Entity(_) => None,
        };
        if let Some(command) = command
            && let Err(error) = copied.apply(command)
        {
            tracing::warn!("not copied: {error}");
        }
    }
    if copied.stack_len() == 0 {
        return;
    }
    match copied.to_canvas_value() {
        Ok(json) => effects.push(Effect::WriteClipboard(format!("{CLIPBOARD_PREFIX}{json}"))),
        Err(error) => tracing::warn!("not copied: {error}"),
    }
}

/// `entity` as the clipboard holds it: where it is seen, bound to its page
/// and that page's document and nothing finer.
fn as_seen(scrolls: &Scrolls, entity: &Entity) -> Entity {
    let seen = scroll_follow::fold(scrolls, entity).unwrap_or_else(|| entity.clone());
    let anchor = seen.anchor.as_ref().map(|anchor| specular_doc::PageAnchor {
        page_url: anchor.page_url.clone(),
        ..specular_doc::PageAnchor::new(anchor.page_id.clone())
    });
    Entity { anchor, ..seen }
}

/// Copies the selection and removes it, as one undo step.
pub(crate) fn cut(app: &mut App, effects: &mut Vec<Effect>) {
    let before = effects.len();
    copy(app, effects);
    if effects.len() > before {
        verbs::delete(app, effects);
    }
}

/// Asks the shell for the clipboard. The answer is an
/// [`Event::Clipboard`](crate::Event::Clipboard).
pub(crate) fn request(app: &App, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_none() {
        effects.push(Effect::ReadClipboard);
    }
}

/// The shell read the clipboard: pastes what it holds at the pointer.
pub(crate) fn on_read(app: &mut App, content: ClipboardContent, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_some() {
        return;
    }
    if app.session.editing.is_some() {
        if let Some(text) = &content.text {
            edit::paste(app, text);
        }
        return;
    }
    let at = paste_point(app);
    match Paste::of(content) {
        Paste::Items(copied) => paste_items(app, &copied, at, effects),
        Paste::Image(image) => {
            let id = EntityId::new(app.fresh_id());
            let file = asset::asset_file(&id, "png");
            effects.push(Effect::WriteAsset {
                file: file.clone(),
                bytes: image.png,
            });
            let size = DVec2::new(f64::from(image.width), f64::from(image.height));
            let entity = asset::file_entity(id, file, geometry::rect(at, size));
            asset::insert_selected(app, vec![entity], effects);
        }
        Paste::Page(url) => {
            let id = EntityId::new(app.fresh_id());
            asset::insert_selected(app, vec![page(id, at, url)], effects);
        }
        Paste::Text(text) => {
            let id = EntityId::new(app.fresh_id());
            let mut entity = place::text(app, id, at, TextStyle::Sticky);
            if let Kind::Text(sticky) = &mut entity.kind {
                sticky.text = text;
            }
            asset::insert_selected(app, vec![entity], effects);
        }
        Paste::Nothing => {}
    }
}

/// Where a paste lands: the grid point nearest the pointer, or nearest the
/// middle of the viewport the sidebar leaves free when the pointer is
/// outside the window.
pub(crate) fn paste_point(app: &App) -> DVec2 {
    let session = &app.session;
    let screen = session
        .pointer
        .unwrap_or_else(|| crate::viewport::centre(app));
    let world = session.camera.screen_to_world(screen).as_dvec2();
    DVec2::new(grid::snap(world.x), grid::snap(world.y))
}

/// Adds everything in `copied` with fresh ids, its top-left corner at `at`,
/// and selects what was not inside a copied group. What came without its
/// page is hooked to the page it lands on, or to none (ADR 0031).
fn paste_items(app: &mut App, copied: &Document, at: DVec2, effects: &mut Vec<Effect>) {
    let Some(bounds) = bounds(copied) else {
        return;
    };
    let delta = at - geometry::origin(bounds);
    let all = |_: &EntityId| true;
    let ids = clone::fresh_ids(app, |_| clone::needed(copied, all));
    let stack = app.document.stack_len();
    let source = Source {
        document: copied,
        scrolls: &Scrolls::default(),
    };
    let (commands, renamed) = clone::insertions(&source, all, ids, stack, delta, Orphan::Leaves);
    let members: Vec<ItemId> = (copied.entities())
        .filter(|entity| {
            (entity.parent.as_ref()).is_none_or(|parent| !renamed.contains_key(parent))
        })
        .filter_map(|entity| renamed.get(&entity.id).cloned())
        .map(ItemId::Entity)
        .collect();
    let scrolls = Scrolls::of(app);
    let command = anchor::placed_copies(&mut app.document, &scrolls, Command::Batch(commands));
    update::document_step(app, command, effects);
    app.session.selection.set(members);
}

fn bounds(document: &Document) -> Option<Rect> {
    (document.entities())
        .map(|entity| entity.rect)
        .reduce(geometry::union)
}

/// The page a pasted URL makes: Electron's `paste_url` page at the Desktop
/// preset.
fn page(id: EntityId, at: DVec2, url: String) -> Entity {
    let metadata = json!({
        "createdFrom": "paste_url",
        "deviceOrientation": "landscape",
        "showDeviceFrame": true,
        "deviceId": PASTED_PAGE_DEVICE,
    });
    let page = Page {
        url,
        preset_index: Some(PASTED_PAGE_PRESET),
        source: Some(PageSource::Manual),
        metadata: metadata.as_object().cloned(),
        ..Page::default()
    };
    // `at` is where the device's corner goes, and the screen sits in from it.
    let mut rect = geometry::rect(at, PASTED_PAGE_SIZE);
    if let Some(shell) = page.shell() {
        rect.x += shell.insets.left;
        rect.y += shell.insets.top;
    }
    Entity::new(id, rect, Kind::Page(page))
}
