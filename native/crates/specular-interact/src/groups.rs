//! The group verbs: wrap the selection in a group, take a group apart, and
//! step into one. Each is one undo step, and the groups above whatever it
//! changes are refitted in it (`group_fit`).

use specular_doc::{Command, Document, Entity, EntityId, Group, ItemId, Kind, LayoutMode, Rect};

use crate::group_fit::PADDING as GROUP_PADDING;
use crate::{App, Effect, group_drop, update};

/// What a group is called until it is renamed.
pub(crate) const DEFAULT_LABEL: &str = "Group";

/// The selection with every group all of whose members are in it standing
/// for them, then without anything inside another selected item: what
/// grouping wraps. Groups come last, as they are added.
fn roots_to_group(document: &Document, selected: &[EntityId]) -> Vec<EntityId> {
    let mut ids: Vec<EntityId> = selected.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for group in document.entities().filter(|e| crate::scope::is_group(e)) {
            let children: Vec<&EntityId> = document.children(&group.id).map(|c| &c.id).collect();
            if children.is_empty() || !children.iter().all(|child| ids.contains(child)) {
                continue;
            }
            ids.retain(|id| !children.contains(&id));
            if !ids.contains(&group.id) {
                ids.push(group.id.clone());
            }
            changed = true;
        }
    }
    group_drop::roots(document, &ids)
}

/// Wraps the selected items in a new group with 24 units of room round
/// them, and selects it. The group goes where the frontmost of its run
/// was, and the run is gathered there. Needs two items selected, and does
/// nothing otherwise.
pub(crate) fn group(app: &mut App, effects: &mut Vec<Effect>) {
    let selected: Vec<EntityId> = (app.session.selection.entities())
        .filter(|id| app.document.entity(id).is_some())
        .cloned()
        .collect();
    if selected.len() < 2 {
        return;
    }
    let document = &app.document;
    let roots = roots_to_group(document, &selected);
    let Some(content) = (roots.iter())
        .filter_map(|id| document.entity(id))
        .map(|entity| entity.rect)
        .reduce(crate::geometry::union)
    else {
        return;
    };
    let parent = roots
        .first()
        .and_then(|id| document.entity(id)?.parent.clone());
    // Just in front of the run the new group will lead.
    let at = roots
        .iter()
        .flat_map(|id| {
            let mut family = vec![ItemId::Entity(id.clone())];
            family.extend(descendants(document, id));
            family
        })
        .filter_map(|item| document.stack_index(&item))
        .max()
        .map_or(document.stack_len(), |frontmost| frontmost + 1);

    let id = EntityId::from(app.fresh_id().as_str());
    let rect = Rect::new(
        content.x - GROUP_PADDING,
        content.y - GROUP_PADDING,
        content.width + GROUP_PADDING * 2.0,
        content.height + GROUP_PADDING * 2.0,
    );
    let entity = Entity {
        label: Some(DEFAULT_LABEL.to_owned()),
        parent,
        ..Entity::new(
            id.clone(),
            rect,
            Kind::Group(Group {
                layout_mode: Some(LayoutMode::Freeform),
                managed_layout: Some(false),
                ..Group::default()
            }),
        )
    };
    let mut commands = vec![Command::InsertEntity {
        entity: Box::new(entity),
        at,
    }];
    commands.extend(roots.into_iter().map(|root| Command::SetParent {
        id: root,
        parent: Some(id.clone()),
    }));
    commands.extend(group_drop::contiguity_after(&app.document, &commands));
    update::document_step(app, Command::Batch(commands), effects);
    app.session.selection.set([ItemId::Entity(id)]);
}

/// Every item inside `group`, nested groups' too.
fn descendants(document: &Document, group: &EntityId) -> Vec<ItemId> {
    let mut found: Vec<ItemId> = Vec::new();
    // A stack, so a parent cycle in a hand-edited file ends at the first
    // repeat.
    let mut pending = vec![group.clone()];
    while let Some(next) = pending.pop() {
        for child in document.children(&next) {
            let item = ItemId::Entity(child.id.clone());
            if !found.contains(&item) {
                found.push(item);
                pending.push(child.id.clone());
            }
        }
    }
    found
}

/// The group that is the whole selection, if it is.
pub(crate) fn lone_group(app: &App) -> Option<&Entity> {
    let entity = app
        .document
        .entity(app.session.selection.single_entity()?)?;
    crate::scope::is_group(entity).then_some(entity)
}

/// Takes the selected group apart: its members go to the group's own
/// group, or to none, and they become the selection. The edges the group
/// had go with it. Does nothing unless a group is the whole selection.
pub(crate) fn ungroup(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(group) = lone_group(app) else {
        return;
    };
    let document = &app.document;
    let freed: Vec<EntityId> = document.children(&group.id).map(|c| c.id.clone()).collect();
    let mut commands: Vec<Command> = freed
        .iter()
        .map(|id| Command::SetParent {
            id: id.clone(),
            parent: group.parent.clone(),
        })
        .collect();
    commands.extend(
        document
            .edges_touching(&group.id)
            .map(|edge| Command::RemoveEdge(edge.id.clone())),
    );
    commands.push(Command::RemoveEntity(group.id.clone()));
    update::document_step(app, Command::Batch(commands), effects);
    app.session
        .selection
        .set(freed.into_iter().map(ItemId::Entity));
}

/// Selects what is directly inside `group`, one level down. Returns
/// `false`, and changes nothing, for a group with nothing in it.
pub(crate) fn enter(app: &mut App, group: &EntityId) -> bool {
    let children: Vec<ItemId> = (app.document.children(group))
        .map(|child| ItemId::Entity(child.id.clone()))
        .collect();
    if children.is_empty() {
        return false;
    }
    app.session.selection.set(children);
    app.session.entered_group = Some(group.clone());
    true
}

/// Forgets the entered group once the selection is not wholly inside it, or
/// it is gone.
pub(crate) fn keep_entered_valid(app: &mut App) {
    let Some(group) = &app.session.entered_group else {
        return;
    };
    let document = &app.document;
    let selection = &app.session.selection;
    let inside = |id: &EntityId| document.ancestors(id).any(|above| above.id == *group);
    let valid = document.entity(group).is_some()
        && !selection.is_empty()
        && selection.items().iter().all(|item| match item {
            ItemId::Entity(id) => inside(id),
            ItemId::Edge(_) => false,
        });
    if !valid {
        app.session.entered_group = None;
    }
}

/// Escape with a group entered: the group becomes the selection and the
/// entered group is the one above it, if any. Returns whether there was one
/// to step out of.
pub(crate) fn step_out(app: &mut App) -> bool {
    let Some(group) = app.session.entered_group.take() else {
        return false;
    };
    app.session.entered_group = app
        .document
        .entity(&group)
        .and_then(|entity| entity.parent.clone());
    app.session.selection.set([ItemId::Entity(group)]);
    true
}
