//! The left sidebar, as data: the Canvases list and the Notes and Pages
//! sections of the active canvas.
//!
//! Like [`menus`](crate::menus), this is a pure reading of the [`App`].
//! Every row carries the [`Action`] a click on it sends, so whatever draws
//! the sidebar holds no state of its own.

use std::collections::HashMap;

use specular_doc::{Annotation, AnnotationId, Entity, EntityId, ItemId, Kind, Page, ShapeKind};

use crate::anchor::matches_page_url;
use crate::app::page_of;
use crate::comment::is_open;
use crate::{Action, App, CanvasAction, CanvasId};

/// How many characters of a comment's text name its row.
const COMMENT_LABEL_CHARS: usize = 60;

/// Everything the sidebar shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarModel {
    /// The canvases of the space, in its order.
    pub canvases: Vec<CanvasRow>,
    /// The Notes section: everything on the active canvas that is not a
    /// page, front of the stack first.
    pub notes: Vec<SidebarRow>,
    /// The Pages section: the active canvas's pages, front of the stack
    /// first, each with what is hooked to it.
    pub pages: Vec<SidebarRow>,
}

/// One canvas in the Canvases list.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRow {
    /// The canvas.
    pub id: CanvasId,
    /// Its name.
    pub label: String,
    /// Whether it is the one the window shows.
    pub active: bool,
    /// How many entities it holds.
    pub entity_count: usize,
    /// What a click sends: a switch to it.
    pub action: Action,
}

/// One row of the Notes or Pages section.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    /// What the row stands for.
    pub target: RowTarget,
    /// What sort of row it is.
    pub kind: RowKind,
    /// The text.
    pub label: String,
    /// Whether what it stands for is selected, or for a comment, focused.
    pub selected: bool,
    /// Whether it is drawn faded: it is hooked to a document its page has
    /// navigated away from.
    pub dimmed: bool,
    /// The rows inside it: a group's members, or what is hooked to a page.
    pub children: Vec<Self>,
    /// What a click sends.
    pub action: Action,
}

/// What a sidebar row stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowTarget {
    /// An entity of the active canvas.
    Entity(EntityId),
    /// A comment bound to a page.
    Comment(AnnotationId),
}

/// What sort of thing a sidebar row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A group. A group with members in both sections has a row in each.
    Group {
        /// How many items it holds in this section, at any depth.
        entity_count: usize,
    },
    /// A live web page.
    Page,
    /// Plain text or a sticky note.
    Text,
    /// A file: a Document, an image.
    File,
    /// A freehand drawing.
    Drawing {
        /// How many strokes it has.
        strokes: usize,
    },
    /// A shape.
    Shape(ShapeKind),
    /// An open comment on a page.
    Comment {
        /// The comment and its replies.
        messages: usize,
    },
}

/// The two sections a row can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Notes,
    Pages,
}

/// The sidebar for `app` as it is now.
pub fn sidebar(app: &App) -> SidebarModel {
    let canvases = (app.space.canvases().iter())
        .map(|canvas| CanvasRow {
            id: canvas.id.clone(),
            label: canvas.name.clone(),
            active: canvas.is_active(),
            entity_count: (app.canvas_document(&canvas.id))
                .map_or(0, |document| document.entities().count()),
            action: Action::Canvas(CanvasAction::Switch(canvas.id.clone())),
        })
        .collect();
    let tree = Tree::new(app);
    let rows = tree.rows_under(None);
    SidebarModel {
        canvases,
        notes: partition(&rows, Section::Notes),
        pages: partition(&rows, Section::Pages),
    }
}

/// The active canvas, read for building rows.
struct Tree<'a> {
    app: &'a App,
    /// Each entity's place in the stack. A higher rank is nearer the front.
    ranks: HashMap<&'a EntityId, usize>,
}

impl<'a> Tree<'a> {
    fn new(app: &'a App) -> Self {
        let ranks = (app.document.order().iter().enumerate())
            .filter_map(|(rank, item)| match item {
                ItemId::Entity(id) => Some((id, rank)),
                ItemId::Edge(_) => None,
            })
            .collect();
        Self { app, ranks }
    }

    /// The entities of the canvas, front of the stack first.
    fn front_first(&self) -> Vec<&'a Entity> {
        let mut entities: Vec<&Entity> = self.app.document.entities().collect();
        entities.sort_by_key(|entity| std::cmp::Reverse(self.ranks.get(&entity.id).copied()));
        entities
    }

    /// The rows for the members of `group`, or with `None` the rows at the
    /// top level. An item hooked to a page is under that page, not here.
    fn rows_under(&self, group: Option<&EntityId>) -> Vec<SidebarRow> {
        (self.front_first().into_iter())
            .filter(|entity| self.parent(entity) == group)
            .filter(|entity| self.hooked_page(entity).is_none())
            .map(|entity| self.row(entity))
            .collect()
    }

    /// The group `entity` is in, if the canvas has it. A parent that names
    /// nothing leaves the entity at the top level, where it can be found.
    fn parent(&self, entity: &'a Entity) -> Option<&'a EntityId> {
        (entity.parent.as_ref()).filter(|parent| self.app.document.entity(parent).is_some())
    }

    /// The page `entity` nests under: the one it is anchored to, when it is
    /// in no group and that page is on the canvas.
    fn hooked_page(&self, entity: &'a Entity) -> Option<&'a EntityId> {
        if self.parent(entity).is_some() || matches!(entity.kind, Kind::Group(_) | Kind::Page(_)) {
            return None;
        }
        let page = &entity.anchor.as_ref()?.page_id;
        (self.app.document.entity(page))
            .and_then(page_of)
            .map(|_| page)
    }

    fn row(&self, entity: &'a Entity) -> SidebarRow {
        let (kind, children) = match &entity.kind {
            Kind::Group(_) => {
                let children = self.rows_under(Some(&entity.id));
                let entity_count = leaves(&children);
                (RowKind::Group { entity_count }, children)
            }
            Kind::Page(page) => (RowKind::Page, self.page_children(&entity.id, page)),
            Kind::Text(_) => (RowKind::Text, Vec::new()),
            Kind::File(_) => (RowKind::File, Vec::new()),
            Kind::Drawing(drawing) => {
                let strokes = drawing.strokes.len();
                (RowKind::Drawing { strokes }, Vec::new())
            }
            Kind::Shape(shape) => (RowKind::Shape(shape.shape), Vec::new()),
        };
        let item = ItemId::Entity(entity.id.clone());
        SidebarRow {
            target: RowTarget::Entity(entity.id.clone()),
            kind,
            label: self.label(entity),
            selected: self.app.session.selection.contains(&item),
            dimmed: false,
            children,
            action: Action::Select(vec![item]),
        }
    }

    /// What belongs to a page: the items hooked to it, front first, then
    /// the open comments bound to it, newest first. A row is dimmed when
    /// the page no longer shows the document it was placed on.
    fn page_children(&self, id: &'a EntityId, page: &Page) -> Vec<SidebarRow> {
        let hooked = (self.front_first().into_iter())
            .filter(|entity| self.hooked_page(entity) == Some(id))
            .map(|entity| {
                let recorded = entity.anchor.as_ref().and_then(|a| a.page_url.as_deref());
                SidebarRow {
                    dimmed: !matches_page_url(recorded, Some(&page.url)),
                    ..self.row(entity)
                }
            });
        let mut comments: Vec<&Annotation> = (self.app.document.annotations().iter())
            .filter(|annotation| is_open(annotation.status))
            .filter(|annotation| {
                (annotation.page_anchor.as_ref()).is_some_and(|binding| binding.page_id == *id)
            })
            .collect();
        // ISO 8601 stamps sort as text.
        comments.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let comments = comments.into_iter().map(|annotation| {
            let recorded = (annotation.page_anchor.as_ref()).and_then(|a| a.page_url.as_deref());
            SidebarRow {
                target: RowTarget::Comment(annotation.id.clone()),
                kind: RowKind::Comment {
                    messages: 1 + annotation.replies.len(),
                },
                label: comment_label(annotation),
                selected: self.app.session.focused_comment.as_ref() == Some(&annotation.id),
                dimmed: !matches_page_url(recorded, Some(&page.url)),
                children: Vec::new(),
                action: Action::FocusComment(Some(annotation.id.clone())),
            }
        });
        hooked.chain(comments).collect()
    }

    fn label(&self, entity: &Entity) -> String {
        let named = (entity.label.as_deref().map(str::trim)).filter(|label| !label.is_empty());
        match &entity.kind {
            Kind::Page(page) => {
                let state = self.app.page_state(&entity.id);
                let title = state
                    .map(|state| state.title.trim())
                    .filter(|title| !title.is_empty());
                (title.or(named).map(str::to_owned))
                    .or_else(|| host_label(&page.url))
                    .unwrap_or_else(|| "Page".to_owned())
            }
            Kind::Text(text) => (named.or_else(|| first_line(&text.text)))
                .unwrap_or("Text")
                .to_owned(),
            Kind::File(file) => {
                let name = file.file.rsplit('/').next().unwrap_or(&file.file);
                let stem = name.len().checked_sub(3).filter(|&at| {
                    name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(".md")
                });
                stem.map_or(name, |at| &name[..at]).to_owned()
            }
            Kind::Group(_) => named.unwrap_or("Group").to_owned(),
            Kind::Drawing(drawing) => named.map_or_else(
                || {
                    let strokes = drawing.strokes.len();
                    let plural = if strokes == 1 { "" } else { "s" };
                    format!("Drawing ({strokes} stroke{plural})")
                },
                str::to_owned,
            ),
            Kind::Shape(shape) => (named.or_else(|| first_line(&shape.text)))
                .unwrap_or_else(|| shape_label(shape.shape))
                .to_owned(),
        }
    }
}

/// The rows of `rows` that belong in `section`. A group is kept where it
/// has members, with only those members, so a group of notes and pages has
/// a row in each section.
fn partition(rows: &[SidebarRow], section: Section) -> Vec<SidebarRow> {
    let mut kept = Vec::new();
    for row in rows {
        match row.kind {
            RowKind::Group { .. } => {
                let children = partition(&row.children, section);
                if !children.is_empty() {
                    kept.push(SidebarRow {
                        kind: RowKind::Group {
                            entity_count: leaves(&children),
                        },
                        children,
                        ..row.clone()
                    });
                }
            }
            RowKind::Page => {
                if section == Section::Pages {
                    kept.push(row.clone());
                }
            }
            RowKind::Text
            | RowKind::File
            | RowKind::Drawing { .. }
            | RowKind::Shape(_)
            | RowKind::Comment { .. } => {
                if section == Section::Notes {
                    kept.push(row.clone());
                }
            }
        }
    }
    kept
}

/// How many rows under `rows` are not groups, at any depth of group.
fn leaves(rows: &[SidebarRow]) -> usize {
    (rows.iter())
        .map(|row| match row.kind {
            RowKind::Group { .. } => leaves(&row.children),
            RowKind::Page
            | RowKind::Text
            | RowKind::File
            | RowKind::Drawing { .. }
            | RowKind::Shape(_)
            | RowKind::Comment { .. } => 1,
        })
        .sum()
}

/// A comment's row text: the element it is on, else the start of what it
/// says.
fn comment_label(annotation: &Annotation) -> String {
    let element = (annotation.element_name.as_deref().map(str::trim)).filter(|n| !n.is_empty());
    if let Some(element) = element {
        return element.to_owned();
    }
    let text = annotation.text.trim();
    if text.is_empty() {
        return "Comment".to_owned();
    }
    if text.chars().count() <= COMMENT_LABEL_CHARS {
        return text.to_owned();
    }
    let start: String = text.chars().take(COMMENT_LABEL_CHARS - 1).collect();
    format!("{start}\u{2026}")
}

fn first_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

/// The host of `url` without a leading `www.`, or `None` when it has none.
fn host_label(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?.split(':').next()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    (!host.is_empty()).then(|| host.to_owned())
}

const fn shape_label(shape: ShapeKind) -> &'static str {
    match shape {
        ShapeKind::Rectangle => "Rectangle",
        ShapeKind::Rounded => "Rounded rectangle",
        ShapeKind::Ellipse => "Ellipse",
        ShapeKind::Diamond => "Diamond",
        ShapeKind::Triangle => "Triangle",
        ShapeKind::Hexagon => "Hexagon",
        ShapeKind::Pill => "Pill",
        ShapeKind::Parallelogram => "Parallelogram",
        ShapeKind::Chevron => "Chevron",
        ShapeKind::Cylinder => "Cylinder",
    }
}
