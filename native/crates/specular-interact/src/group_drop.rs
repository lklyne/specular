//! Group membership by drop: which group a released drag lands in, and the
//! commands that reparent what was dragged (ADR 0014's contiguity included).

use glam::DVec2;
use specular_doc::{Command, Document, EntityId, ItemId, Rect};

use crate::scope::is_group;
use crate::stack_order::{self, document_runs};

/// Whether `rect` holds `point`, edges included, as the drop target's test
/// does.
fn holds(rect: Rect, point: DVec2) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

/// The group a drag released at `point` lands in: the smallest group whose
/// rect holds the point, the front-most of equal ones. `excluded` are the
/// groups that travel with the drag, which are never a target.
pub(crate) fn drop_target(
    document: &Document,
    rects: &[(EntityId, Rect)],
    point: DVec2,
    excluded: &[EntityId],
) -> Option<EntityId> {
    let stack = |id: &EntityId| document.stack_index(&ItemId::Entity(id.clone()));
    rects
        .iter()
        .filter(|(id, rect)| !excluded.contains(id) && holds(*rect, point))
        .min_by(|(a, a_rect), (b, b_rect)| {
            let area = |rect: &Rect| rect.width * rect.height;
            // The later in the stack, the more in front.
            (area(a_rect).total_cmp(&area(b_rect))).then_with(|| stack(b).cmp(&stack(a)))
        })
        .map(|(id, _)| id.clone())
}

/// Every group's rect, in stack order: where the drop targets are for the
/// length of a drag, whatever the groups do meanwhile.
pub(crate) fn group_rects(document: &Document) -> Vec<(EntityId, Rect)> {
    document
        .entities()
        .filter(|entity| is_group(entity))
        .map(|group| (group.id.clone(), group.rect))
        .collect()
}

/// `ids` without any whose ancestor is in `ids` too: the ones a reparent
/// acts on, because the rest travel with them.
pub(crate) fn roots(document: &Document, ids: &[EntityId]) -> Vec<EntityId> {
    ids.iter()
        .filter(|id| {
            let mut ancestors = document.ancestors(id);
            !ancestors.any(|group| ids.contains(&group.id))
        })
        .cloned()
        .collect()
}

/// The commands that make each of `members` (its roots only) a child of
/// `target`, or of no group. A member already there stays, and a group is
/// never put into itself or something inside it. Empty when nothing
/// changes; otherwise it ends with the stack order that keeps every
/// group's run contiguous, if that moves anything.
pub(crate) fn reparent(
    document: &Document,
    members: &[EntityId],
    target: Option<&EntityId>,
) -> Vec<Command> {
    if target.is_some_and(|group| {
        document
            .entity(group)
            .is_none_or(|entity| !is_group(entity))
    }) {
        return Vec::new();
    }
    let mut commands: Vec<Command> = roots(document, members)
        .into_iter()
        .filter(|id| {
            let Some(entity) = document.entity(id) else {
                return false;
            };
            let inside_itself = target.is_some_and(|group| {
                group == id || document.ancestors(group).any(|above| above.id == *id)
            });
            !inside_itself && entity.parent.as_ref() != target
        })
        .map(|id| Command::SetParent {
            id,
            parent: target.cloned(),
        })
        .collect();
    if commands.is_empty() {
        return commands;
    }
    commands.extend(contiguity_after(document, &commands));
    commands
}

/// The stack order that gathers every group's run once `commands` have been
/// applied to `document`, or `None` when it is contiguous already.
pub(crate) fn contiguity_after(document: &Document, commands: &[Command]) -> Option<Command> {
    let mut trial = document.clone();
    trial.apply(Command::Batch(commands.to_vec())).ok()?;
    let next = stack_order::enforce_contiguity(trial.order(), &document_runs(&trial));
    (next != trial.order()).then_some(Command::SetOrder(next))
}

#[cfg(test)]
mod tests {
    use specular_doc::{Entity, Group, Kind};

    use super::*;

    fn group(id: &str, x: f64, y: f64, width: f64, height: f64) -> Entity {
        Entity::new(
            id,
            Rect::new(x, y, width, height),
            Kind::Group(Group::default()),
        )
    }

    fn document_of(groups: impl IntoIterator<Item = Entity>) -> Document {
        let mut document = Document::new();
        for (at, entity) in groups.into_iter().enumerate() {
            document
                .apply(Command::InsertEntity {
                    entity: Box::new(entity),
                    at,
                })
                .expect("inserts");
        }
        document
    }

    fn target(document: &Document, x: f64, y: f64, excluded: &[&str]) -> Option<String> {
        let excluded: Vec<EntityId> = excluded.iter().map(|id| EntityId::from(*id)).collect();
        drop_target(
            document,
            &group_rects(document),
            DVec2::new(x, y),
            &excluded,
        )
        .map(|id| id.as_str().to_owned())
    }

    #[test]
    fn the_dragged_group_is_no_target_and_a_border_counts_as_inside() {
        let document = document_of([
            group("outer", 100.0, 100.0, 500.0, 500.0),
            group("inner", 200.0, 200.0, 120.0, 120.0),
        ]);
        // (pointer, excluded, target)
        let rows = [
            ((250.0, 250.0), vec![], Some("inner")),
            ((250.0, 250.0), vec!["inner"], Some("outer")),
            ((600.0, 600.0), vec![], Some("outer")),
            ((50.0, 50.0), vec![], None),
        ];
        for ((x, y), excluded, want) in rows {
            assert_eq!(
                target(&document, x, y, &excluded).as_deref(),
                want,
                "({x}, {y}) without {excluded:?}"
            );
        }
    }

    #[test]
    fn equal_groups_go_to_the_one_in_front() {
        let document = document_of([
            group("a", 0.0, 0.0, 100.0, 100.0),
            group("b", 0.0, 0.0, 100.0, 100.0),
        ]);
        assert_eq!(target(&document, 10.0, 10.0, &[]).as_deref(), Some("b"));
    }
}
