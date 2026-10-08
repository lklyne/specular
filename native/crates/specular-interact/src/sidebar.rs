//! The left sidebar, as data: the Canvases list and the Notes and Pages
//! sections of the active canvas.
//!
//! Like [`menus`](crate::menus), this is a pure reading of the [`App`].
//! Every row and control carries the [`Action`] a click on it sends, and
//! the state a renderer would need to keep, whether the sidebar is shown,
//! which sections are folded and which rows are open, is in the session
//! ([`SidebarView`]) and reported here, so whatever draws the sidebar holds
//! none of it.

mod labels;
mod rows;
mod state;

use specular_doc::EntityId;

use self::rows::{Tree, leaves};
pub use self::state::{SIDEBAR_WIDTH, SidebarAction, SidebarSection, SidebarView};
use crate::panel::{ControlId, Field, FieldSubmit, FieldWidth, Icon};
use crate::{Action, App, CanvasAction, CanvasId};

/// Everything the sidebar shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarModel {
    /// Whether the sidebar is shown at all.
    pub visible: bool,
    /// What shows or hides it.
    pub toggle: Action,
    /// The head of the Canvases list.
    pub canvases_head: SectionHead,
    /// The canvases of the space, in its order.
    pub canvases: Vec<CanvasRow>,
    /// What the add button next to the Canvases head sends.
    pub add_canvas: Action,
    /// The head of the Notes section.
    pub notes_head: SectionHead,
    /// The Notes section: everything on the active canvas that is not a
    /// page, front of the stack first.
    pub notes: Vec<SidebarRow>,
    /// The head of the Pages section.
    pub pages_head: SectionHead,
    /// The Pages section: the active canvas's pages, front of the stack
    /// first, each with what is hooked to it.
    pub pages: Vec<SidebarRow>,
}

impl SidebarModel {
    /// Whether the canvas has nothing for either section to list.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.pages.is_empty()
    }
}

/// The head of a section: the title that folds it.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionHead {
    /// Its name.
    pub id: ControlId,
    /// Which section it heads.
    pub section: SidebarSection,
    /// The title. A folded Canvases list is titled by the active canvas.
    pub title: String,
    /// Whether the section is folded away.
    pub folded: bool,
    /// What a click on the head sends: fold or unfold.
    pub toggle: Action,
}

/// One canvas in the Canvases list.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasRow {
    /// The row's name.
    pub control: ControlId,
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
    /// The field that renames it in place. Its value is the name, and an
    /// empty or unchanged name asks for nothing.
    pub rename: Field,
    /// What removing it sends. Removing the last canvas leaves a fresh one.
    pub delete: Action,
}

/// One row of the Notes or Pages section.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    /// The row's name, which is unique among the rows shown: a group with
    /// notes and pages has a row in each section.
    pub id: ControlId,
    /// What the row stands for.
    pub target: RowTarget,
    /// What sort of row it is.
    pub kind: RowKind,
    /// The text.
    pub label: String,
    /// The glyph before the text.
    pub glyph: Icon,
    /// The text at the row's far end: a group's count, a page's size, a
    /// comment's number of messages.
    pub trailing: Option<String>,
    /// Whether what it stands for is selected, or for a comment, focused.
    pub selected: bool,
    /// Whether it is drawn faded: it is hooked to a document its page has
    /// navigated away from.
    pub dimmed: bool,
    /// Whether the rows inside it are shown, for a row that has any.
    pub expanded: Option<bool>,
    /// What opens or closes it, for a row that has rows inside it.
    pub toggle: Option<Action>,
    /// The rows inside it: a group's members, or what is hooked to a page.
    pub children: Vec<Self>,
    /// What a click sends: select what the row stands for and bring it into
    /// view.
    pub action: Action,
}

/// What a sidebar row stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowTarget {
    /// An entity of the active canvas.
    Entity(EntityId),
    /// A comment bound to a page.
    Comment(specular_doc::AnnotationId),
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
    Shape(specular_doc::ShapeKind),
    /// An open comment on a page.
    Comment {
        /// The comment and its replies.
        messages: usize,
    },
}

/// The sidebar for `app` as it is now.
pub fn sidebar(app: &App) -> SidebarModel {
    let view = &app.session.sidebar;
    let canvases: Vec<CanvasRow> = (app.space.canvases().iter())
        .map(|canvas| {
            let id = ControlId::new("sidebar.canvas").child(canvas.id.as_str());
            CanvasRow {
                control: id.clone(),
                id: canvas.id.clone(),
                label: canvas.name.clone(),
                active: canvas.is_active(),
                entity_count: (app.canvas_document(&canvas.id))
                    .map_or(0, |document| document.entities().count()),
                action: Action::Canvas(CanvasAction::Switch(canvas.id.clone())),
                rename: Field {
                    id: id.child("name"),
                    label: "Canvas name".into(),
                    caption: None,
                    value: canvas.name.clone(),
                    placeholder: None,
                    width: FieldWidth::Wide,
                    submit: FieldSubmit::CanvasName(canvas.id.clone()),
                },
                delete: Action::Canvas(CanvasAction::Delete(Some(canvas.id.clone()))),
            }
        })
        .collect();
    let head = |section, name: &'static str, title: &str| SectionHead {
        id: ControlId::new("sidebar.head").child(name),
        section,
        title: title.to_owned(),
        folded: view.is_folded(section),
        toggle: Action::Sidebar(SidebarAction::Section(section)),
    };
    let mut canvases_head = head(SidebarSection::Canvases, "canvases", "Canvases");
    if canvases_head.folded
        && let Some(active) = canvases.iter().find(|canvas| canvas.active)
    {
        canvases_head.title.clone_from(&active.label);
    }
    let rows = Tree::new(app).top_rows();
    SidebarModel {
        visible: view.shown(),
        toggle: Action::Sidebar(SidebarAction::Toggle),
        canvases_head,
        canvases,
        add_canvas: Action::Canvas(CanvasAction::New),
        notes_head: head(SidebarSection::Notes, "notes", "Notes"),
        notes: partition(&rows, SidebarSection::Notes, view),
        pages_head: head(SidebarSection::Pages, "pages", "Pages"),
        pages: partition(&rows, SidebarSection::Pages, view),
    }
}

/// The rows of `rows` that belong in `section`, named and opened as that
/// section has them. A group is kept where it has members, with only those
/// members, so a group of notes and pages has a row in each section.
fn partition(rows: &[SidebarRow], section: SidebarSection, view: &SidebarView) -> Vec<SidebarRow> {
    let mut kept = Vec::new();
    for row in rows {
        match row.kind {
            RowKind::Group { .. } => {
                let children = partition(&row.children, section, view);
                if !children.is_empty() {
                    let entity_count = leaves(&children);
                    kept.push(opened(
                        SidebarRow {
                            kind: RowKind::Group { entity_count },
                            trailing: Some(entity_count.to_string()),
                            children,
                            ..row.clone()
                        },
                        section,
                        view,
                    ));
                }
            }
            RowKind::Page => {
                if section == SidebarSection::Pages {
                    kept.push(opened(row.clone(), section, view));
                }
            }
            RowKind::Text
            | RowKind::File
            | RowKind::Drawing { .. }
            | RowKind::Shape(_)
            | RowKind::Comment { .. } => {
                if section == SidebarSection::Notes {
                    kept.push(opened(row.clone(), section, view));
                }
            }
        }
    }
    kept
}

/// `row` and what is inside it named for `section`, with the state of its
/// fold. A group starts closed and a page open (`useState(false)` and
/// `useState(true)` in `SidebarCanvasTree.tsx`).
fn opened(mut row: SidebarRow, section: SidebarSection, view: &SidebarView) -> SidebarRow {
    let name = match &row.target {
        RowTarget::Entity(id) => id.as_str().to_owned(),
        RowTarget::Comment(id) => format!("comment.{}", id.as_str()),
    };
    let section_name = match section {
        SidebarSection::Notes => "notes",
        SidebarSection::Pages => "pages",
        SidebarSection::Canvases => "canvases",
    };
    row.id = ControlId::new("sidebar").child(section_name).child(name);
    let starts_open = match row.kind {
        RowKind::Group { .. } => Some(false),
        RowKind::Page => Some(true),
        RowKind::Text
        | RowKind::File
        | RowKind::Drawing { .. }
        | RowKind::Shape(_)
        | RowKind::Comment { .. } => None,
    };
    if let (Some(starts_open), RowTarget::Entity(entity)) = (starts_open, &row.target) {
        let open = view.is_open(section, entity, starts_open);
        let has_children = !row.children.is_empty();
        row.expanded = has_children.then_some(open);
        row.toggle = has_children.then(|| {
            Action::Sidebar(SidebarAction::Row {
                section,
                entity: entity.clone(),
            })
        });
        if matches!(row.kind, RowKind::Group { .. }) {
            row.glyph = if open { Icon::FolderOpen } else { Icon::Folder };
        }
    }
    row.children = (row.children.iter().cloned())
        .map(|child| opened(child, section, view))
        .collect();
    row
}

/// Starts typing a new name in the sidebar's row for `canvas`. With the
/// sidebar hidden there is no row to type in.
pub(crate) fn begin_rename(
    app: &mut App,
    canvas: &crate::CanvasId,
    effects: &mut Vec<crate::Effect>,
) {
    if !app.session.sidebar.shown() {
        return;
    }
    let field = crate::sidebar(app)
        .canvases
        .into_iter()
        .find(|row| row.id == *canvas)
        .map(|row| row.rename.id);
    if let Some(field) = field {
        crate::edit::begin_field(app, &field, effects);
    }
}
