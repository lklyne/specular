//! What the selection can be told to do without the pointer: delete,
//! duplicate and nudge. Each is one undo step.

use glam::DVec2;
use specular_doc::{Command, EdgeId, ItemId, Rect};

use crate::live::{self, Start};
use crate::{App, Effect, clone, geometry, grid, update};

/// The gap left between placed items, in canvas units.
const PLACEMENT_GAP: f64 = 80.0;
/// How far past the occupied canvas the search for a free spot goes.
const SCAN_MARGIN: f64 = 2000.0;

/// Removes the selection: the selected edges, the selected entities with
/// everything inside their groups and hooked to their pages, and every edge
/// that would be left with a missing end.
pub(crate) fn delete(app: &mut App, effects: &mut Vec<Effect>) {
    let scope = app.selection_scope();
    let document = &app.document;
    let mut edges: Vec<EdgeId> = Vec::new();
    let selected = app
        .session
        .selection
        .items()
        .iter()
        .filter_map(|item| match item {
            ItemId::Edge(id) => document.edge(id).map(|edge| &edge.id),
            ItemId::Entity(_) => None,
        });
    let touching = (scope.operands.iter())
        .flat_map(|id| document.edges_touching(id))
        .map(|edge| &edge.id);
    for id in selected.chain(touching) {
        if !edges.contains(id) {
            edges.push(id.clone());
        }
    }
    let commands: Vec<Command> = (edges.into_iter().map(Command::RemoveEdge))
        .chain(scope.operands.into_iter().map(Command::RemoveEntity))
        .collect();
    if !commands.is_empty() {
        update::document_step(app, Command::Batch(commands), effects);
    }
}

/// Copies the selection into free space beside it and selects the copies.
pub(crate) fn duplicate(app: &mut App, effects: &mut Vec<Effect>) {
    let scope = app.selection_scope();
    let Some(bounds) = scope.bounds else {
        return;
    };
    let delta = free_spot(app, bounds) - geometry::origin(bounds);
    if let Some(copies) = clone::copies(app, &scope, delta) {
        update::document_step(app, copies.command, effects);
        app.session.selection.set(copies.members);
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
        update::document_step(app, live::batch(commands), effects);
    }
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
