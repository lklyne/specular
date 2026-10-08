//! Page anchoring (ADR 0031): placement decides whether an entity is hooked
//! to a page. One whose centre lands on a page's body is, and from then on
//! it moves with that page.

use specular_doc::{Command, Document, Entity, EntityId, Kind, PageAnchor};

use crate::app::page_of;
use crate::geometry;

/// `url` in the form an anchor records and compares: trimmed, with its hash
/// stripped. `None` for a page with no URL yet.
pub(crate) fn canonical_page_url(url: &str) -> Option<String> {
    let document = url.trim().split('#').next().unwrap_or_default();
    (!document.is_empty()).then(|| document.to_owned())
}

/// Whether placement can hook an entity of this kind to a page.
pub const fn anchors_to_pages(kind: &Kind) -> bool {
    match kind {
        Kind::Text(_) | Kind::Drawing(_) | Kind::Shape(_) => true,
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) => false,
    }
}

/// The anchor `entity` gets from where it sits: the frontmost page whose body
/// holds the entity's centre, or `None` on empty canvas. An entity in a
/// group never anchors, because the group already owns its movement.
///
/// The anchor records no scroll offset, so the entity is pinned to the page
/// and does not follow its scroll.
pub fn page_anchor_for(document: &Document, entity: &Entity) -> Option<PageAnchor> {
    if entity.parent.is_some() || !anchors_to_pages(&entity.kind) {
        return None;
    }
    let rect = entity.rect;
    let centre = geometry::origin(rect) + geometry::size(rect) / 2.0;
    document.entities().rev().find_map(|candidate| {
        let page = page_of(candidate)?;
        geometry::contains(candidate.rect, centre).then(|| PageAnchor {
            page_url: canonical_page_url(&page.url),
            ..PageAnchor::new(candidate.id.clone())
        })
    })
}

/// The commands that re-resolve the anchor of each of `ids` from where it
/// sits now: hooked to the page under its centre, or freed when none is.
/// Nothing is said about an entity whose answer is its current page, so the
/// anchor keeps what it recorded. An entity that is not anchorable, or whose
/// anchor page is in `travelling`, is left alone: it moved with its page, so
/// the page still owns it.
pub(crate) fn reanchor(
    document: &Document,
    ids: &[EntityId],
    travelling: &[EntityId],
) -> Vec<Command> {
    ids.iter()
        .filter_map(|id| document.entity(id))
        .filter(|entity| anchors_to_pages(&entity.kind))
        .filter(|entity| {
            (entity.anchor.as_ref()).is_none_or(|anchor| !travelling.contains(&anchor.page_id))
        })
        .filter_map(|entity| {
            let next = page_anchor_for(document, entity);
            let same = match (&entity.anchor, &next) {
                (None, None) => true,
                (Some(old), Some(new)) => {
                    old.page_id == new.page_id && old.page_url == new.page_url
                }
                (Some(_), None) | (None, Some(_)) => false,
            };
            (!same).then(|| Command::SetAnchor {
                id: entity.id.clone(),
                anchor: next.map(Box::new),
            })
        })
        .collect()
}

/// What `read` makes of the document with `command` applied, which is
/// taken back before this returns.
fn looking_after(
    document: &mut Document,
    command: &Command,
    read: impl FnOnce(&Document) -> Vec<Command>,
) -> Vec<Command> {
    let Ok(undo) = document.apply(command.clone()) else {
        return Vec::new();
    };
    let found = read(document);
    if let Err(error) = document.apply(undo) {
        tracing::warn!("trial step not taken back: {error}");
    }
    found
}

/// `step` followed by the re-anchoring of `ids` from where `step` leaves
/// them. `travelling` are the entities `step` moves as one, as in
/// [`reanchor`].
pub(crate) fn then_reanchor(
    document: &mut Document,
    step: Command,
    ids: &[EntityId],
    travelling: &[EntityId],
) -> Command {
    let more = looking_after(document, &step, |after| reanchor(after, ids, travelling));
    follow(step, more)
}

/// `insert` followed by the page anchors the inserted copies get from where
/// they land, as placement decides (ADR 0031). A copy that came with its page
/// keeps the anchor to that page's copy; every other copy is hooked to the
/// page under its centre, or free.
pub(crate) fn placed_copies(document: &mut Document, insert: Command) -> Command {
    let copies: Vec<EntityId> = match &insert {
        Command::Batch(commands) => commands
            .iter()
            .filter_map(|command| match command {
                Command::InsertEntity { entity, .. } if entity.anchor.is_none() => {
                    Some(entity.id.clone())
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    then_reanchor(document, insert, &copies, &[])
}

fn follow(step: Command, more: Vec<Command>) -> Command {
    if more.is_empty() {
        return step;
    }
    let mut commands = vec![step];
    commands.extend(more);
    Command::Batch(commands)
}
