//! Dragging a line's handles: a reorder dot carries its box to another
//! slot, and a gap strip opens or closes the spacing. Both show the line as
//! it would be while the drag goes, and become one undo step at the
//! release.
//!
//! A managed group's line commits to the group: the sequence, or the gap,
//! and the layout follows in the step. A loose selection's commits only
//! where its boxes now are.

use glam::DVec2;
use specular_doc::EntityId;

use super::handles::{self, LayoutHandle};
use super::row::Row;
use super::{act, reordered};
use crate::live::{self, Start};
use crate::{App, Effect, Gesture, grid, scope, update};

/// What a line drag is changing.
#[derive(Debug, Clone, PartialEq)]
enum Change {
    /// A box on its way to a slot.
    Reorder {
        moving: EntityId,
        /// The slot it would take if released now.
        slot: usize,
    },
    /// The gap, from what it was at the press.
    Gap { start: f64, gap: f64 },
}

/// A box of the line with everything that travels with it, as the drag
/// found them.
#[derive(Debug, Clone, PartialEq)]
struct Part {
    id: EntityId,
    starts: Vec<Start>,
}

/// A reorder dot or a gap strip being dragged.
#[derive(Debug, Clone, PartialEq)]
pub struct LineDrag {
    /// The managed group whose line it is, or `None` for a loose
    /// selection's.
    group: Option<EntityId>,
    /// The line as the press found it. Slots are read from this and never
    /// from the preview, which would feed back into itself.
    row: Row,
    parts: Vec<Part>,
    followers: Vec<Start>,
    /// The canvas point the press landed on.
    origin: DVec2,
    /// How far the pointer has travelled from it.
    travel: DVec2,
    change: Change,
}

/// The box a reorder is carrying, floating where the pointer has taken it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReorderGhost {
    /// The box.
    pub entity: EntityId,
    /// How far from the slot the document has it in the ghost floats, in
    /// canvas units.
    pub delta: DVec2,
}

impl LineDrag {
    /// The box being reordered, when that is what the drag does.
    pub(crate) fn moving(&self) -> Option<&EntityId> {
        match &self.change {
            Change::Reorder { moving, .. } => Some(moving),
            Change::Gap { .. } => None,
        }
    }

    /// Where each box of the line sits now.
    fn targets(&self) -> Vec<(EntityId, DVec2)> {
        match &self.change {
            Change::Reorder { moving, slot } => self.row.reordered(moving, *slot),
            Change::Gap { gap, .. } => self.row.regapped(*gap),
        }
    }

    fn all_starts(&self) -> Vec<Start> {
        (self.parts.iter())
            .flat_map(|part| part.starts.iter().cloned())
            .collect()
    }
}

impl App {
    pub(crate) fn line_drag(&self) -> Option<&LineDrag> {
        match &self.session.gesture {
            Some(Gesture::Line(drag)) => Some(drag),
            Some(
                Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::Marquee { .. }
                | Gesture::Comment(_)
                | Gesture::Place(_)
                | Gesture::Draw(_)
                | Gesture::TextSelect(_)
                | Gesture::EdgeDrag(_),
            )
            | None => None,
        }
    }

    /// The box a reorder drag is carrying and where it floats, for drawing
    /// it under the pointer while its slot shows where it would land.
    pub fn reorder_ghost(&self) -> Option<ReorderGhost> {
        let drag = self.line_drag()?;
        let moving = drag.moving()?;
        let start = (drag.row.boxes.iter()).find(|(id, _)| id == moving)?.1;
        let now = self.document.entity(moving)?.rect;
        let floating = DVec2::new(start.x, start.y) + drag.travel;
        Some(ReorderGhost {
            entity: moving.clone(),
            delta: floating - DVec2::new(now.x, now.y),
        })
    }
}

/// A press on `handle` at the canvas point `world`.
pub(crate) fn begin(app: &App, handle: &LayoutHandle, world: DVec2) -> Option<LineDrag> {
    let lines = handles::lines(app);
    let (group, row, change) = match handle {
        LayoutHandle::Reorder { entity } => {
            let (group, row) =
                (lines.into_iter()).find(|(_, row)| row.index_of(entity).is_some())?;
            let slot = row.index_of(entity)?;
            let moving = entity.clone();
            (group, row, Change::Reorder { moving, slot })
        }
        LayoutHandle::Gap { group, .. } => {
            let (group, row) = (lines.into_iter()).find(|(found, _)| found == group)?;
            // A loose row's gap is a mean, and the drag deals in whole units.
            let start = grid::round(row.gap);
            (group, row, Change::Gap { start, gap: start })
        }
    };
    let parts: Vec<Part> = (row.boxes.iter())
        .map(|(id, _)| Part {
            id: id.clone(),
            starts: live::starts(
                &app.document,
                &scope::operands(&app.document, std::slice::from_ref(id)),
            ),
        })
        .collect();
    let starts: Vec<Start> = parts.iter().flat_map(|part| part.starts.clone()).collect();
    Some(LineDrag {
        followers: live::followers_of(&app.document, &starts),
        group,
        row,
        parts,
        origin: world,
        travel: DVec2::ZERO,
        change,
    })
}

/// The pointer is at the canvas point `world` with `drag` in flight: the
/// line is shown as a release now would leave it.
pub(crate) fn drag(app: &mut App, drag: &mut LineDrag, world: DVec2) {
    drag.travel = world - drag.origin;
    let axis = drag.row.axis;
    match &mut drag.change {
        Change::Reorder { slot, .. } => *slot = drag.row.drop_index(axis.of(world)),
        // The pointer's travel along the axis is the change in the gap.
        Change::Gap { start, gap } => *gap = grid::round(*start + axis.of(drag.travel)).max(0.0),
    }
    for (id, to) in drag.targets() {
        let Some(part) = drag.parts.iter().find(|part| part.id == id) else {
            continue;
        };
        let Some(own) = part.starts.iter().find(|start| start.id == id) else {
            continue;
        };
        let by = to - DVec2::new(own.rect.x, own.rect.y);
        for start in &part.starts {
            let (rect, kind) = start.moved(by);
            live::write(&mut app.document, start, rect, kind);
        }
    }
    live::follow(&mut app.document, &drag.followers);
}

/// The button came up: one undo step, or none when nothing changed.
pub(crate) fn finish(app: &mut App, drag: &LineDrag, effects: &mut Vec<Effect>) {
    let starts = drag.all_starts();
    let Some(group) = &drag.group else {
        live::commit_following(app, &starts, &drag.followers, |_| Vec::new());
        return;
    };
    cancel(app, drag);
    let command = match &drag.change {
        Change::Reorder { moving, slot } => reordered(&app.document, group, moving, *slot),
        // Gaps are whole units, so a change is at least one.
        Change::Gap { start, gap } => ((gap - start).abs() >= 1.0)
            .then(|| act::gap_command(app, group, *gap))
            .flatten(),
    };
    if let Some(command) = command {
        update::document_step(app, command, effects);
    }
}

/// The drag was abandoned: the line goes back as it was.
pub(crate) fn cancel(app: &mut App, drag: &LineDrag) {
    live::restore(&mut app.document, &drag.all_starts());
    live::restore(&mut app.document, &drag.followers);
}
