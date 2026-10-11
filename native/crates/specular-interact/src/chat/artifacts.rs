//! What a turn made or edited, as the cards under the agent's answer.
//!
//! The stream names the files the agent's own tools wrote, and
//! [`note_changes`] adds what its API calls did to the canvas. A card is
//! resolved against the canvas each time the panel is read, so one whose
//! item was deleted since is gone and a file placed on the canvas later
//! becomes a card that can be clicked.

use specular_agent::{Artifact, ArtifactTarget, ThreadId};
use specular_doc::{Document, Entity, ItemId, Kind, TextStyle};

use super::model::ArtifactCard;
use crate::labels::{file_icon, file_label, first_line, page_icon, page_label, shape_label};
use crate::notes::is_note_file;
use crate::panel::Icon;
use crate::{Action, App};

/// The cards for `artifacts`, in order, one for each thing: a file that an
/// item of the canvas shows is that item's card.
pub(crate) fn cards(app: &App, artifacts: &[Artifact]) -> Vec<ArtifactCard> {
    let mut seen: Vec<&Entity> = Vec::new();
    let mut cards = Vec::new();
    for artifact in artifacts {
        let entity = match &artifact.target {
            ArtifactTarget::Item(id) => {
                let Some(entity) = app.document.entities().find(|it| it.id.as_str() == id) else {
                    continue;
                };
                Some(entity)
            }
            ArtifactTarget::File(path) => showing_file(app, path),
        };
        match (entity, &artifact.target) {
            (Some(entity), _) => {
                if seen.iter().any(|held| held.id == entity.id) {
                    continue;
                }
                seen.push(entity);
                cards.push(item_card(app, entity, artifact.created));
            }
            (None, ArtifactTarget::File(path)) => cards.push(ArtifactCard {
                label: file_label(path),
                detail: "File".to_owned(),
                glyph: file_icon(path),
                created: artifact.created,
                focus: None,
            }),
            (None, ArtifactTarget::Item(_)) => {}
        }
    }
    cards
}

/// The item of the active canvas that shows the file at `path`, which the
/// agent's tool named in full or from the space folder.
fn showing_file<'a>(app: &'a App, path: &str) -> Option<&'a Entity> {
    let folder = app
        .space
        .folder()
        .map(|folder| folder.trim_end_matches('/'));
    let relative = folder
        .and_then(|folder| path.strip_prefix(folder))
        .map_or(path, |rest| rest.trim_start_matches('/'));
    app.document.entities().find(|entity| match &entity.kind {
        Kind::File(file) => file.file == relative,
        Kind::Page(_) | Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => false,
    })
}

fn item_card(app: &App, entity: &Entity, created: bool) -> ArtifactCard {
    let named = (entity.label.as_deref().map(str::trim)).filter(|label| !label.is_empty());
    let (label, detail, glyph) = match &entity.kind {
        Kind::Page(page) => (
            page_label(app, entity, page),
            "Page",
            page_icon(entity.rect.width),
        ),
        Kind::Text(text) => (
            (named.or_else(|| first_line(&text.text)))
                .unwrap_or("Text")
                .to_owned(),
            match text.resolved_style() {
                TextStyle::Sticky => "Sticky note",
                TextStyle::Plain => "Text",
            },
            Icon::StickyNote,
        ),
        Kind::File(file) => (
            file_label(&file.file),
            if is_note_file(&file.file) {
                "Document"
            } else {
                "File"
            },
            file_icon(&file.file),
        ),
        Kind::Group(_) => (named.unwrap_or("Group").to_owned(), "Group", Icon::Folder),
        Kind::Drawing(_) => (
            named.unwrap_or("Drawing").to_owned(),
            "Drawing",
            Icon::PenLine,
        ),
        Kind::Shape(shape) => (
            (named.or_else(|| first_line(&shape.text)))
                .unwrap_or_else(|| shape_label(shape.shape))
                .to_owned(),
            "Shape",
            Icon::Shape(shape.shape),
        ),
    };
    let item = ItemId::Entity(entity.id.clone());
    ArtifactCard {
        label,
        detail: detail.to_owned(),
        glyph,
        created,
        focus: Some(Action::Reveal {
            select: vec![item.clone()],
            focus: item,
        }),
    }
}

/// Notes on the run of `thread` what its API call did to the active
/// document, which was `before`: the entities it added, and the ones whose
/// content it changed. A move or a resize is not worth a card.
pub(crate) fn note_changes(app: &mut App, thread: &ThreadId, before: &Document) {
    let changes: Vec<Artifact> = (app.document.entities())
        .filter_map(|entity| {
            let created = match before.entity(&entity.id) {
                None => true,
                Some(old) if old.kind != entity.kind => false,
                Some(_) => return None,
            };
            Some(Artifact {
                target: ArtifactTarget::Item(entity.id.as_str().to_owned()),
                created,
            })
        })
        .collect();
    for change in changes {
        app.threads.note_artifact(thread, change);
    }
}
