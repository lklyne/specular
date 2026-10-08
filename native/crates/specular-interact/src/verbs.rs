//! What the selection can be told to do without the pointer: delete,
//! duplicate, nudge and reorder. Each is one undo step.

use glam::DVec2;
use specular_doc::{Command, Document, EdgeId, EntityId, ItemId, Rect};

use crate::live::{self, Start};
use crate::scroll_follow::Scrolls;
use crate::stack_order::{self, Move};
use crate::{App, Effect, anchor, clone, geometry, grid, scope, update, zoom};

/// The gap left between placed items, in canvas units.
const PLACEMENT_GAP: f64 = 80.0;
/// How far past the occupied canvas the search for a free spot goes.
const SCAN_MARGIN: f64 = 2000.0;

/// Removes the selection: the selected edges, the selected entities with
/// everything inside their groups, and every edge that would be left with a
/// missing end. What is hooked to a deleted page stays, freed where it is
/// seen.
pub(crate) fn delete(app: &mut App, effects: &mut Vec<Effect>) {
    let selection = &app.session.selection;
    let entities: Vec<EntityId> = selection.entities().cloned().collect();
    let edges: Vec<EdgeId> = (selection.items().iter())
        .filter_map(|item| match item {
            ItemId::Edge(id) => Some(id.clone()),
            ItemId::Entity(_) => None,
        })
        .collect();
    let commands = deletion(&app.document, &Scrolls::of(app), &entities, &edges);
    if !commands.is_empty() {
        update::document_step(app, Command::Batch(commands), effects);
    }
}

/// The commands that remove `edges` and `entities`, with everything inside
/// the entities' groups, and every edge that would be left with a missing
/// end. Ids that name nothing are skipped.
///
/// What is hooked to a deleted page is not removed: an entity goes
/// canvas-bound where it is stored and a comment loses its binding, in the
/// same step, as Electron's `clearPageAnchorsForPage` does.
pub fn delete_commands(
    document: &Document,
    entities: &[EntityId],
    edges: &[EdgeId],
) -> Vec<Command> {
    deletion(document, &Scrolls::default(), entities, edges)
}

/// [`delete_commands`] knowing how far each page has carried what follows
/// it, so that a freed entity stays where it is seen.
fn deletion(
    document: &Document,
    scrolls: &Scrolls,
    entities: &[EntityId],
    edges: &[EdgeId],
) -> Vec<Command> {
    let operands = scope::contained(document, entities);
    let pages: Vec<EntityId> = (operands.iter())
        .filter(|id| document.entity(id).and_then(crate::app::page_of).is_some())
        .cloned()
        .collect();
    let freed = anchor::freed(document, scrolls, &pages, &operands);
    let mut doomed: Vec<EdgeId> = Vec::new();
    let named = edges.iter().filter(|id| document.edge(id).is_some());
    let touching = (operands.iter())
        .flat_map(|id| document.edges_touching(id))
        .map(|edge| &edge.id);
    for id in named.chain(touching) {
        if !doomed.contains(id) {
            doomed.push(id.clone());
        }
    }
    freed
        .into_iter()
        .chain(doomed.into_iter().map(Command::RemoveEdge))
        .chain(operands.into_iter().map(Command::RemoveEntity))
        .collect()
}

/// The commands that move `entities` by `delta` canvas units, with
/// everything inside their groups and hooked to their pages. A drawing's
/// points travel with it.
pub fn move_commands(document: &Document, entities: &[EntityId], delta: DVec2) -> Vec<Command> {
    translate_commands(document, &scope::operands(document, entities), delta)
}

/// The commands that move exactly `entities` by `delta` canvas units.
pub(crate) fn translate_commands(
    document: &Document,
    entities: &[EntityId],
    delta: DVec2,
) -> Vec<Command> {
    (live::starts(document, entities).iter())
        .flat_map(|start| moved(start, delta))
        .collect()
}

/// Copies the selection into free space beside it, selects the copies and
/// brings them into view: on a crowded canvas the free space can be a long
/// way off, and a duplicate that lands off screen looks like nothing
/// happened.
pub(crate) fn duplicate(app: &mut App, effects: &mut Vec<Effect>) {
    let scope = app.selection_scope();
    // The copies are made of what is seen, so they are placed from there.
    let Some(bounds) = scope.shown_bounds.or(scope.bounds) else {
        return;
    };
    let delta = free_spot(app, bounds) - geometry::origin(bounds);
    if let Some(copies) = clone::copies(app, &scope, delta) {
        // What was copied without its page is hooked to the page it lands
        // on, or to none (ADR 0031).
        let scrolls = Scrolls::of(app);
        let command = anchor::placed_copies(&mut app.document, &scrolls, copies.command);
        update::document_step(app, command, effects);
        app.session.selection.set(copies.members);
        zoom::reveal(app, bounds.translated(delta.x, delta.y));
    }
}

/// Moves the selection by exactly `delta` canvas units. No grid: a nudge
/// from off the grid stays off it.
pub(crate) fn nudge(app: &mut App, delta: DVec2, effects: &mut Vec<Effect>) {
    let scope = app.selection_scope();
    let starts = live::starts(&app.document, &scope.operands);
    let commands: Vec<Command> = starts
        .iter()
        .flat_map(|start| moved(start, delta))
        .collect();
    if !commands.is_empty() {
        // Only what was nudged directly re-resolves; what is hooked to a
        // nudged page travels with it.
        let step = live::batch(commands);
        let scrolls = Scrolls::of(app);
        let step = anchor::then_reanchor(
            &mut app.document,
            &scrolls,
            step,
            &scope.members,
            &scope.operands,
        );
        update::document_step(app, step, effects);
    }
}

/// Moves the selection in the stack order, entities and edges alike, and
/// keeps every group's members in one run (ADR 0014). Does nothing, and
/// records no step, when the order would not change.
pub(crate) fn reorder(app: &mut App, how: Move, effects: &mut Vec<Effect>) {
    let document = &app.document;
    let order = document.order();
    let next = stack_order::enforce_contiguity(
        &stack_order::apply(order, &stacked_selection(app), how),
        &stack_order::document_runs(document),
    );
    if next != order {
        update::document_step(app, Command::SetOrder(next), effects);
    }
}

/// The selection as stack items: each selected group stands for its whole
/// run, so the group moves as the unit ADR 0014 says it is. A selected
/// member alone is just itself and moves inside its group's run.
fn stacked_selection(app: &App) -> Vec<ItemId> {
    let mut items: Vec<ItemId> = Vec::new();
    // A stack, so a parent cycle in a hand-edited file ends at the first
    // repeat.
    let mut pending: Vec<ItemId> = app.session.selection.items().to_vec();
    while let Some(item) = pending.pop() {
        if items.contains(&item) {
            continue;
        }
        if let ItemId::Entity(id) = &item {
            pending
                .extend((app.document.children(id)).map(|child| ItemId::Entity(child.id.clone())));
        }
        items.push(item);
    }
    items
}

fn moved(start: &Start, delta: DVec2) -> Vec<Command> {
    let (rect, kind) = start.moved(delta);
    let mut commands = Vec::new();
    if rect != start.rect {
        commands.push(Command::SetRect {
            id: start.id.clone(),
            rect,
        });
    }
    if let Some(kind) = kind
        && start.kind.as_ref() != Some(&kind)
    {
        commands.push(Command::SetKind {
            id: start.id.clone(),
            kind: Box::new(kind),
        });
    }
    commands
}

/// Where a copy of something filling `source` goes: on the grid to its
/// right, or below it, or the first free spot found scanning on from there.
/// A spot is free when it is [`PLACEMENT_GAP`] clear of every entity.
fn free_spot(app: &App, source: Rect) -> DVec2 {
    let size = DVec2::new(grid::snap(source.width), grid::snap(source.height));
    let occupied: Vec<Rect> = app.document.entities().map(|entity| entity.rect).collect();
    let free = |at: DVec2| {
        let candidate = geometry::rect(at, size);
        !occupied.iter().any(|rect| overlaps(candidate, *rect))
    };
    let right = DVec2::new(
        grid::snap(source.x + source.width + PLACEMENT_GAP),
        grid::snap(source.y),
    );
    let below = DVec2::new(
        grid::snap(source.x),
        grid::snap(source.y + source.height + PLACEMENT_GAP),
    );
    if free(right) {
        return right;
    }
    if free(below) {
        return below;
    }
    let reach = occupied
        .iter()
        .fold(DVec2::splat(SCAN_MARGIN), |reach, rect| {
            reach.max(geometry::origin(*rect) + geometry::size(*rect) + PLACEMENT_GAP)
        });
    let limit = reach + size + SCAN_MARGIN;
    let mut at = right;
    while at.y <= limit.y {
        while at.x <= limit.x {
            if free(at) {
                return at;
            }
            at.x += grid::GRID_SIZE;
        }
        at = DVec2::new(PLACEMENT_GAP, at.y + grid::GRID_SIZE);
    }
    right
}

/// Whether `candidate` comes within [`PLACEMENT_GAP`] of `rect`.
fn overlaps(candidate: Rect, rect: Rect) -> bool {
    candidate.x < rect.x + rect.width + PLACEMENT_GAP
        && candidate.x + candidate.width > rect.x - PLACEMENT_GAP
        && candidate.y < rect.y + rect.height + PLACEMENT_GAP
        && candidate.y + candidate.height > rect.y - PLACEMENT_GAP
}
