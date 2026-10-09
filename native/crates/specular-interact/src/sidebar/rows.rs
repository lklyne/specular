//! The rows of the Notes and Pages sections, read off the active canvas.

use std::collections::HashMap;

use specular_doc::{Annotation, AnnotationId, Entity, EntityId, ItemId, Kind, Page};

use super::{RowKind, RowTarget, SidebarRow};
use crate::anchor::matches_page_url;
use crate::app::page_of;
use crate::comment::is_open;
use crate::labels::{
    comment_label, dimensions, file_icon, file_label, first_line, page_icon, page_label,
    shape_label,
};
use crate::panel::{ControlId, Icon};
use crate::{Action, App};

/// The active canvas, indexed for building rows: one pass over the entities
/// and the comments, so a canvas of thousands of rows is not read once per
/// group and per page.
pub(super) struct Tree<'a> {
    app: &'a App,
    /// The entities that are in a group, by the group, front of the stack
    /// first.
    members: HashMap<&'a EntityId, Vec<&'a Entity>>,
    /// The entities hooked to a page that is on the canvas, by the page,
    /// front of the stack first.
    hooked: HashMap<&'a EntityId, Vec<&'a Entity>>,
    /// The entities that are in no group and hooked to no page.
    top: Vec<&'a Entity>,
    /// The open comments bound to a page, by the page, newest first.
    comments: HashMap<&'a EntityId, Vec<&'a Annotation>>,
}

impl<'a> Tree<'a> {
    pub(super) fn new(app: &'a App) -> Self {
        let document = &app.document;
        let ranks: HashMap<&EntityId, usize> = (document.order().iter().enumerate())
            .filter_map(|(rank, item)| match item {
                ItemId::Entity(id) => Some((id, rank)),
                ItemId::Edge(_) => None,
            })
            .collect();
        let mut entities: Vec<&Entity> = document.entities().collect();
        entities.sort_by_key(|entity| std::cmp::Reverse(ranks.get(&entity.id).copied()));

        let mut members: HashMap<&EntityId, Vec<&Entity>> = HashMap::new();
        let mut hooked: HashMap<&EntityId, Vec<&Entity>> = HashMap::new();
        let mut top = Vec::new();
        for entity in entities {
            // A parent that names nothing leaves the entity at the top
            // level, where it can be found.
            let parent =
                (entity.parent.as_ref()).filter(|parent| document.entity(parent).is_some());
            if let Some(parent) = parent {
                members.entry(parent).or_default().push(entity);
                continue;
            }
            match Self::hook(app, entity) {
                Some(page) => hooked.entry(page).or_default().push(entity),
                None => top.push(entity),
            }
        }

        let mut comments: HashMap<&EntityId, Vec<&Annotation>> = HashMap::new();
        for annotation in document.annotations() {
            if !is_open(annotation.status) {
                continue;
            }
            if let Some(binding) = &annotation.page_anchor {
                comments
                    .entry(&binding.page_id)
                    .or_default()
                    .push(annotation);
            }
        }
        for list in comments.values_mut() {
            // ISO 8601 stamps sort as text.
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        }
        Self {
            app,
            members,
            hooked,
            top,
            comments,
        }
    }

    /// The page `entity` nests under: the one it is anchored to, when it is
    /// in no group and that page is on the canvas.
    fn hook(app: &'a App, entity: &'a Entity) -> Option<&'a EntityId> {
        if matches!(entity.kind, Kind::Group(_) | Kind::Page(_)) {
            return None;
        }
        let page = &entity.anchor.as_ref()?.page_id;
        (app.document.entity(page)).and_then(page_of).map(|_| page)
    }

    /// The rows at the top level of the canvas.
    pub(super) fn top_rows(&self) -> Vec<SidebarRow> {
        self.top.iter().map(|entity| self.row(entity)).collect()
    }

    fn group_rows(&self, group: &EntityId) -> Vec<SidebarRow> {
        (self.members.get(group).into_iter().flatten())
            .map(|entity| self.row(entity))
            .collect()
    }

    fn row(&self, entity: &Entity) -> SidebarRow {
        let (kind, glyph, children) = match &entity.kind {
            Kind::Group(_) => {
                let children = self.group_rows(&entity.id);
                let entity_count = leaves(&children);
                (RowKind::Group { entity_count }, Icon::Folder, children)
            }
            Kind::Page(page) => (
                RowKind::Page,
                page_icon(entity.rect.width),
                self.page_children(&entity.id, page),
            ),
            Kind::Text(_) => (RowKind::Text, Icon::StickyNote, Vec::new()),
            Kind::File(file) => (RowKind::File, file_icon(&file.file), Vec::new()),
            Kind::Drawing(drawing) => {
                let strokes = drawing.strokes.len();
                (RowKind::Drawing { strokes }, Icon::PenLine, Vec::new())
            }
            Kind::Shape(shape) => (
                RowKind::Shape(shape.shape),
                Icon::Shape(shape.shape),
                Vec::new(),
            ),
        };
        let trailing = match &entity.kind {
            Kind::Group(_) => Some(leaves(&children).to_string()),
            Kind::Page(_) => Some(dimensions(entity.rect.width, entity.rect.height)),
            Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
        };
        let item = ItemId::Entity(entity.id.clone());
        SidebarRow {
            id: ControlId::new("sidebar"),
            target: RowTarget::Entity(entity.id.clone()),
            kind,
            label: self.label(entity),
            glyph,
            trailing,
            selected: self.app.session.selection.contains(&item),
            dimmed: false,
            expanded: None,
            toggle: None,
            children,
            action: Action::Reveal {
                select: vec![item.clone()],
                focus: item,
            },
        }
    }

    /// What belongs to a page: the items hooked to it, front first, then
    /// the open comments bound to it, newest first. A row is dimmed when
    /// the page no longer shows the document it was placed on.
    fn page_children(&self, id: &EntityId, page: &Page) -> Vec<SidebarRow> {
        let hooked = (self.hooked.get(id).into_iter().flatten()).map(|entity| {
            let recorded = entity.anchor.as_ref().and_then(|a| a.page_url.as_deref());
            SidebarRow {
                dimmed: !matches_page_url(recorded, Some(&page.url)),
                ..self.row(entity)
            }
        });
        let comments = (self.comments.get(id).into_iter().flatten()).map(|annotation| {
            let recorded = (annotation.page_anchor.as_ref()).and_then(|a| a.page_url.as_deref());
            let messages = 1 + annotation.replies.len();
            SidebarRow {
                id: ControlId::new("sidebar"),
                target: RowTarget::Comment(annotation.id.clone()),
                kind: RowKind::Comment { messages },
                label: comment_label(annotation),
                glyph: Icon::MessageSquare,
                trailing: (messages > 1).then(|| messages.to_string()),
                selected: self.focused(&annotation.id),
                dimmed: !matches_page_url(recorded, Some(&page.url)),
                expanded: None,
                toggle: None,
                children: Vec::new(),
                action: Action::RevealComment(annotation.id.clone()),
            }
        });
        hooked.chain(comments).collect()
    }

    fn focused(&self, id: &AnnotationId) -> bool {
        self.app.session.focused_comment.as_ref() == Some(id)
    }

    fn label(&self, entity: &Entity) -> String {
        let named = (entity.label.as_deref().map(str::trim)).filter(|label| !label.is_empty());
        match &entity.kind {
            Kind::Page(page) => page_label(self.app, entity, page),
            Kind::Text(text) => (named.or_else(|| first_line(&text.text)))
                .unwrap_or("Text")
                .to_owned(),
            Kind::File(file) => file_label(&file.file),
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

/// How many rows under `rows` are not groups, at any depth of group.
pub(super) fn leaves(rows: &[SidebarRow]) -> usize {
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
