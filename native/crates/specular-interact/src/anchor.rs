//! Page anchoring (ADR 0031): placement decides whether an entity is hooked
//! to a page. One whose centre lands on a page's body is, and from then on
//! it moves with that page.

use specular_doc::{Command, Document, Entity, EntityId, Kind, PageAnchor};

use crate::app::page_of;
use crate::geometry;
use crate::scroll_follow::{self, Scrolls};

/// `url` in the form an anchor records and compares: trimmed, with its hash
/// stripped. `None` for a page with no URL yet.
pub(crate) fn canonical_page_url(url: &str) -> Option<String> {
    let document = url.trim().split('#').next().unwrap_or_default();
    (!document.is_empty()).then(|| document.to_owned())
}

/// Whether a recorded URL still names the page's current document. False
/// only when both sides carry a URL and they disagree once trimmed and
/// stripped of their hash. A missing recorded URL always matches, as does a
/// page with no URL yet.
pub fn matches_page_url(recorded: Option<&str>, current: Option<&str>) -> bool {
    match (
        recorded.and_then(canonical_page_url),
        current.and_then(canonical_page_url),
    ) {
        (Some(recorded), Some(current)) => recorded == current,
        (None, _) | (_, None) => true,
    }
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
/// The anchor records the page's scroll, so the entity tracks the document
/// under it from here on (see [`scroll_follow`](crate::scroll_follow)).
pub(crate) fn page_anchor_for(
    document: &Document,
    scrolls: &Scrolls,
    entity: &Entity,
) -> Option<PageAnchor> {
    if entity.parent.is_some() || !anchors_to_pages(&entity.kind) {
        return None;
    }
    let rect = entity.rect;
    let centre = geometry::origin(rect) + geometry::size(rect) / 2.0;
    document.entities().rev().find_map(|candidate| {
        let page = page_of(candidate)?;
        let live = scrolls.live(&candidate.id);
        geometry::contains(candidate.rect, centre).then(|| PageAnchor {
            page_url: canonical_page_url(&page.url),
            scroll_x: Some(live.x),
            scroll_y: Some(live.y),
            ..PageAnchor::new(candidate.id.clone())
        })
    })
}

/// The commands that re-resolve the anchor of each of `ids` from where it
/// sits now: hooked to the page under its centre, or freed when none is.
/// What the page has scrolled since the anchor was written is folded into the
/// entity's stored position first, and the anchor restamped, so the stored
/// position is where the entity is seen. Nothing is said about an entity
/// whose answer is its current page and that has not moved with the scroll.
/// An entity that is not anchorable, or whose anchor page is in
/// `travelling`, is left alone: it moved with its page, so the page still
/// owns it.
pub(crate) fn reanchor(
    document: &Document,
    scrolls: &Scrolls,
    ids: &[EntityId],
    travelling: &[EntityId],
) -> Vec<Command> {
    ids.iter()
        .filter_map(|id| document.entity(id))
        .filter(|entity| anchors_to_pages(&entity.kind))
        .filter(|entity| {
            (entity.anchor.as_ref()).is_none_or(|anchor| !travelling.contains(&anchor.page_id))
        })
        .flat_map(|entity| {
            let folded = scroll_follow::fold(scrolls, entity);
            let mut commands = folded
                .as_ref()
                .map_or_else(Vec::new, |folded| fold_commands(entity, folded));
            let seen = folded.as_ref().unwrap_or(entity);
            let next = page_anchor_for(document, scrolls, seen);
            let anchor = match (&entity.anchor, next) {
                (None, None) => None,
                (Some(old), Some(new))
                    if old.page_id == new.page_id && old.page_url == new.page_url =>
                {
                    // Same page: the anchor keeps what it recorded, but a
                    // folded shift restamps the scroll it was folded at.
                    folded
                        .is_some()
                        .then(|| Some(scroll_follow::restamped(scrolls, old)))
                }
                (_, new) => Some(new),
            };
            if let Some(anchor) = anchor {
                commands.push(Command::SetAnchor {
                    id: entity.id.clone(),
                    anchor: anchor.map(Box::new),
                });
            }
            commands
        })
        .collect()
}

/// The commands that store `entity` where `folded` has it: its rect and, for
/// a drawing, its points.
fn fold_commands(entity: &Entity, folded: &Entity) -> Vec<Command> {
    let mut commands = vec![Command::SetRect {
        id: entity.id.clone(),
        rect: folded.rect,
    }];
    if folded.kind != entity.kind {
        commands.push(Command::SetKind {
            id: entity.id.clone(),
            kind: Box::new(folded.kind.clone()),
        });
    }
    commands
}

/// The commands that fold what each of `ids` has moved with its page into
/// its stored position and restamp its anchor there: the scroll rebase. The
/// anchor is not re-resolved, so the entity stays on the page it is on.
/// Nothing is said about an entity that has not moved.
pub(crate) fn rebase(document: &Document, scrolls: &Scrolls, ids: &[EntityId]) -> Vec<Command> {
    ids.iter()
        .filter_map(|id| document.entity(id))
        .filter_map(|entity| Some((entity, scroll_follow::fold(scrolls, entity)?)))
        .flat_map(|(entity, folded)| {
            let mut commands = fold_commands(entity, &folded);
            if let Some(anchor) = &entity.anchor {
                commands.push(Command::SetAnchor {
                    id: entity.id.clone(),
                    anchor: Some(Box::new(scroll_follow::restamped(scrolls, anchor))),
                });
            }
            commands
        })
        .collect()
}

/// The commands that free what is hooked to `pages`, which are being
/// deleted: an entity stays where it is seen and goes canvas-bound, and a
/// comment loses its binding to the page. Entities among `removed` go with
/// the pages and are left alone.
pub(crate) fn freed(
    document: &Document,
    scrolls: &Scrolls,
    pages: &[EntityId],
    removed: &[EntityId],
) -> Vec<Command> {
    let on_gone = |anchor: &Option<PageAnchor>| {
        (anchor.as_ref()).is_some_and(|anchor| pages.contains(&anchor.page_id))
    };
    let entities = (document.entities())
        .filter(|entity| on_gone(&entity.anchor) && !removed.contains(&entity.id))
        .flat_map(|entity| {
            let folded = scroll_follow::fold(scrolls, entity);
            let mut commands = folded
                .as_ref()
                .map_or_else(Vec::new, |folded| fold_commands(entity, folded));
            commands.push(Command::SetAnchor {
                id: entity.id.clone(),
                anchor: None,
            });
            commands
        });
    let comments = (document.annotations().iter())
        .filter(|annotation| on_gone(&annotation.page_anchor))
        .map(|annotation| {
            Command::ReplaceAnnotation(Box::new(specular_doc::Annotation {
                page_anchor: None,
                ..annotation.clone()
            }))
        });
    entities.chain(comments).collect()
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
    scrolls: &Scrolls,
    step: Command,
    ids: &[EntityId],
    travelling: &[EntityId],
) -> Command {
    let more = looking_after(document, &step, |after| {
        reanchor(after, scrolls, ids, travelling)
    });
    follow(step, more)
}

/// `insert` followed by the page anchors the inserted copies get from where
/// they land, as placement decides (ADR 0031). A copy that came with its page
/// keeps the anchor to that page's copy; every other copy is hooked to the
/// page under its centre, or free.
pub(crate) fn placed_copies(
    document: &mut Document,
    scrolls: &Scrolls,
    insert: Command,
) -> Command {
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
    then_reanchor(document, scrolls, insert, &copies, &[])
}

fn follow(step: Command, more: Vec<Command>) -> Command {
    if more.is_empty() {
        return step;
    }
    let mut commands = vec![step];
    commands.extend(more);
    Command::Batch(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_match_unless_both_are_set_and_differ_before_the_fragment() {
        let rows = [
            (None, Some("https://example.com/a"), true),
            (Some("https://example.com/a"), Some(""), true),
            (Some("https://example.com/a"), None, true),
            (Some("  "), Some("https://example.com/a"), true),
            (None, None, true),
            (
                Some("https://example.com/a#x"),
                Some("https://example.com/a#y"),
                true,
            ),
            (
                Some("https://example.com/a"),
                Some(" https://example.com/a#top "),
                true,
            ),
            (
                Some("https://example.com/a?tab=1"),
                Some("https://example.com/a?tab=2"),
                false,
            ),
            (
                Some("https://example.com/a"),
                Some("https://example.com/b"),
                false,
            ),
        ];
        for (recorded, shown, want) in rows {
            assert_eq!(
                matches_page_url(recorded, shown),
                want,
                "{recorded:?} vs {shown:?}"
            );
        }
    }
}
