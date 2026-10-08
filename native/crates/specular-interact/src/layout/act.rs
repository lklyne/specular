//! The auto-layout verbs: make the selection a managed row or column, turn
//! a group's layout on, round or off, and set its gap. Each is one undo
//! step with the reflow inside it.

use specular_doc::{Command, EntityId, ItemId, LayoutMode};

use super::{Axis, group_of, line_axis, manage, normal_gap, set_fields};
use crate::{App, Effect, groups, update};

/// What a group made for a layout is called until it is renamed.
pub(crate) const LABEL: &str = "Auto-layout";

/// Makes the selection an auto-layout group and selects it: a lone group is
/// managed as it is, and two or more items are wrapped in a new one first.
pub(crate) fn make(app: &mut App, effects: &mut Vec<Effect>) {
    let selected: Vec<EntityId> = app.session.selection.entities().cloned().collect();
    let id = EntityId::from(app.fresh_id().as_str());
    let Some((group, command)) = make_command(app, &selected, &id, LABEL, None) else {
        return;
    };
    update::document_step(app, command, effects);
    app.session.selection.set([ItemId::Entity(group)]);
}

/// The step that makes `entities` an auto-layout group, and the group: a
/// lone group is managed as it is, and two or more are wrapped in a new
/// group `id` called `label` first. `None` when there is nothing to manage.
pub fn make_command(
    app: &App,
    entities: &[EntityId],
    id: &EntityId,
    label: &str,
    gap: Option<f64>,
) -> Option<(EntityId, Command)> {
    let document = &app.document;
    if let [only] = entities
        && document.entity(only).and_then(group_of).is_some()
    {
        return Some((only.clone(), manage(document, only, None, gap)?));
    }
    if entities.len() < 2 {
        return None;
    }
    let wrap = groups::group_command(document, entities, id, label.to_owned())?;
    let mut wrapped = document.clone();
    wrapped.apply(wrap.clone()).ok()?;
    let managed = manage(&wrapped, id, None, gap)?;
    Some((id.clone(), Command::Batch(vec![wrap, managed])))
}

/// The group that is the whole selection.
fn selected_group(app: &App) -> Option<EntityId> {
    groups::lone_group(app).map(|group| group.id.clone())
}

/// Packs the selected group along `axis`, or with `None` lets its members
/// sit where they are. A group that was not managed takes its sequence from
/// where its members are along the axis.
pub(crate) fn set_axis(app: &mut App, axis: Option<Axis>, effects: &mut Vec<Effect>) {
    let Some(group) = selected_group(app) else {
        return;
    };
    let document = &app.document;
    let managed = document
        .entity(&group)
        .and_then(group_of)
        .and_then(line_axis);
    let command = match (axis, managed) {
        (None, _) => set_fields(document, &group, |fields| {
            fields.managed_layout = Some(false);
            fields.layout_mode = Some(LayoutMode::Freeform);
        }),
        (Some(axis), None) => manage(document, &group, Some(axis), None),
        (Some(axis), Some(_)) => set_fields(document, &group, |fields| {
            fields.layout_mode = Some(mode_of(axis));
        }),
    };
    if let Some(command) = command {
        update::document_step(app, command, effects);
    }
}

/// Sets the gap the selected managed group packs with.
pub(crate) fn set_gap(app: &mut App, gap: f64, effects: &mut Vec<Effect>) {
    let Some(group) = selected_group(app) else {
        return;
    };
    if let Some(command) = gap_command(app, &group, gap) {
        update::document_step(app, command, effects);
    }
}

/// The step that sets the managed group `group`'s gap, or `None` when it
/// has that gap, is not managed, or `gap` is not a number.
pub fn gap_command(app: &App, group: &EntityId, gap: f64) -> Option<Command> {
    let document = &app.document;
    line_axis(group_of(document.entity(group)?)?)?;
    let gap = normal_gap(gap)?;
    set_fields(document, group, |fields| fields.layout_gap = Some(gap))
}

/// The step that moves `child` to slot `to` of the managed group `group`,
/// or `None` when that changes nothing.
pub fn reorder_command(
    app: &App,
    group: &EntityId,
    child: &EntityId,
    to: usize,
) -> Option<Command> {
    super::reordered(&app.document, group, child, to)
}

pub(crate) const fn mode_of(axis: Axis) -> LayoutMode {
    match axis {
        Axis::X => LayoutMode::Row,
        Axis::Y => LayoutMode::Column,
    }
}

impl App {
    /// The axis the group `id` packs its members along and the gap it packs
    /// with, when it manages them.
    pub fn group_layout(&self, id: &EntityId) -> Option<(Axis, f64)> {
        let group = group_of(self.document.entity(id)?)?;
        Some((line_axis(group)?, super::gap_of(group)))
    }
}
