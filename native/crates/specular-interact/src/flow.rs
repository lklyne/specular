//! Laying out entities as a flow: each in a rank by the edges between
//! them, so the arrows run one way, and each edge routed at right angles
//! between them. An agent screenshots a set of screens, links them by what
//! leads where, and the flow lays them out as a state diagram. One flow is
//! one undo step.
//!
//! The layout is [`layered`]. A route is stored on its edge
//! ([`Edge::route`]) and drawn while its ends still sit on their entities;
//! once either moves, the edge is drawn as a curve again until the flow is
//! laid out once more.

mod layered;

use glam::DVec2;
use specular_doc::{Command, Document, Edge, EdgeSide, EntityId, Rect};

use crate::{App, place_command, verbs};

use self::layered::{Link, Node, Spacing};

/// The room a flow leaves, in canvas units: between screens of a rank, the
/// least between ranks, between the ends on one side, between turns, and
/// kept clear round a rank and a label.
const SPACING: Spacing = Spacing {
    node: 80.0,
    rank: 160.0,
    port: 32.0,
    track: 20.0,
    margin: 32.0,
};

/// A character of an edge label, across and along, as a share of the
/// label's size. Rough: the label only needs room, not a fit.
const LABEL_ADVANCE: f64 = 0.6;
const LABEL_LINE: f64 = 1.4;

/// Which way a flow runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlowDirection {
    /// Left to right: ranks are columns. Screens are taller than wide, so
    /// this is the default.
    #[default]
    Across,
    /// Top to bottom: ranks are rows.
    Down,
}

impl FlowDirection {
    /// `LR` or `TB`, as Mermaid names them.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "LR" => Some(Self::Across),
            "TB" | "TD" => Some(Self::Down),
            _ => None,
        }
    }

    /// A canvas point or size in the layout's axes (across, along), and
    /// back: the same swap both ways.
    const fn turn(self, (a, b): (f64, f64)) -> (f64, f64) {
        match self {
            Self::Across => (b, a),
            Self::Down => (a, b),
        }
    }
}

/// What a flow does: where each entity's top-left goes, and the edges
/// with their new routes. Edges between an entity of the flow and one
/// outside it lose their route, which the move would leave behind.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowPlan {
    /// Each entity and its new top-left.
    pub targets: Vec<(EntityId, DVec2)>,
    /// The edges whose route changes, as they become.
    pub edges: Vec<Edge>,
}

/// Lays out `entities` of `document` as a flow running `direction`, along
/// the edges between them, its top-left at `origin` or, without one, where
/// the entities' bounds start. `None` when fewer than two of them are in
/// the document.
pub fn flow_plan(
    document: &Document,
    entities: &[EntityId],
    direction: FlowDirection,
    origin: Option<DVec2>,
) -> Option<FlowPlan> {
    let mut ids: Vec<&EntityId> = Vec::new();
    for id in entities {
        if document.entity(id).is_some() && !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() < 2 {
        return None;
    }
    let rects: Vec<Rect> = (ids.iter())
        .filter_map(|id| document.entity(id).map(|entity| entity.rect))
        .collect();
    let index = |id: &EntityId| ids.iter().position(|known| *known == id);
    let inside: Vec<&Edge> = (document.edges())
        .filter(|edge| index(&edge.from).is_some() && index(&edge.to).is_some())
        .collect();

    let nodes: Vec<Node> = (rects.iter())
        .map(|rect| {
            let (w, h) = direction.turn((rect.width, rect.height));
            Node { w, h }
        })
        .collect();
    let links: Vec<Link> = (inside.iter())
        .filter_map(|edge| {
            Some(Link {
                tail: index(&edge.from)?,
                head: index(&edge.to)?,
                label: label_room(edge, direction),
            })
        })
        .collect();
    let laid = layered::layout(&nodes, &links, SPACING);

    let start = origin.unwrap_or_else(|| {
        let least = |side: fn(&Rect) -> f64| rects.iter().map(side).fold(f64::INFINITY, f64::min);
        DVec2::new(least(|rect| rect.x), least(|rect| rect.y))
    });
    let place = |point: (f64, f64)| {
        let (x, y) = direction.turn(point);
        start + DVec2::new(x, y)
    };
    let placed: Vec<Rect> = (rects.iter().zip(&laid.centres))
        .map(|(rect, &centre)| {
            let middle = place(centre);
            Rect::new(
                (middle.x - rect.width / 2.0).round(),
                (middle.y - rect.height / 2.0).round(),
                rect.width,
                rect.height,
            )
        })
        .collect();
    let targets = (ids.iter().zip(&placed))
        .map(|(id, rect)| ((*id).clone(), DVec2::new(rect.x, rect.y)))
        .collect();

    let mut edges = Vec::new();
    for (edge, route) in inside.iter().zip(&laid.routes) {
        let (Some(from), Some(to)) = (index(&edge.from), index(&edge.to)) else {
            continue;
        };
        let routed = route.as_ref().and_then(|route| {
            let mut points: Vec<DVec2> = route.iter().map(|&point| place(point)).collect();
            let from_side = settle(placed[from], points.first_mut()?);
            let to_side = settle(placed[to], points.last_mut()?);
            Some((points, from_side, to_side))
        });
        let mut next = (*edge).clone();
        match routed {
            Some((points, from_side, to_side)) => {
                next.route = Some(points.iter().map(|point| [point.x, point.y]).collect());
                next.from_side = Some(from_side);
                next.to_side = Some(to_side);
            }
            None => next.route = None,
        }
        if next != **edge {
            edges.push(next);
        }
    }
    // Edges out of the flow keep their curve: a route left behind would
    // no longer meet its entity.
    for edge in document.edges() {
        let ends = (index(&edge.from).is_some(), index(&edge.to).is_some());
        if ends.0 != ends.1 && edge.route.is_some() {
            edges.push(Edge {
                route: None,
                ..edge.clone()
            });
        }
    }
    Some(FlowPlan { targets, edges })
}

/// The room `edge`'s label needs along a flow running `direction`.
fn label_room(edge: &Edge, direction: FlowDirection) -> f64 {
    let size = f64::from(crate::EDGE_LABEL_SIZE);
    match edge.label.as_deref() {
        Some(text) if !text.is_empty() => match direction {
            FlowDirection::Across => text.chars().count() as f64 * size * LABEL_ADVANCE,
            FlowDirection::Down => size * LABEL_LINE,
        },
        Some(_) | None => 0.0,
    }
}

/// The side of `rect` that `point` is on, with `point` put exactly on it:
/// the layout works to a fraction of a unit and the entity is placed on a
/// whole one.
fn settle(rect: Rect, point: &mut DVec2) -> EdgeSide {
    let distances = [
        (EdgeSide::Top, (point.y - rect.y).abs()),
        (EdgeSide::Bottom, (point.y - (rect.y + rect.height)).abs()),
        (EdgeSide::Left, (point.x - rect.x).abs()),
        (EdgeSide::Right, (point.x - (rect.x + rect.width)).abs()),
    ];
    let (side, _) = (distances.into_iter())
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((EdgeSide::Bottom, 0.0));
    match side {
        EdgeSide::Top => point.y = rect.y,
        EdgeSide::Bottom => point.y = rect.y + rect.height,
        EdgeSide::Left => point.x = rect.x,
        EdgeSide::Right => point.x = rect.x + rect.width,
    }
    side
}

/// The commands that carry out `plan` on `document`: each entity moved
/// with what is inside its group, then the edges replaced.
pub fn plan_commands(document: &Document, plan: &FlowPlan) -> Vec<Command> {
    let mut commands: Vec<Command> = (plan.targets.iter())
        .flat_map(|(id, to)| {
            let Some(entity) = document.entity(id) else {
                return Vec::new();
            };
            let by = *to - DVec2::new(entity.rect.x, entity.rect.y);
            if by == DVec2::ZERO {
                return Vec::new();
            }
            verbs::move_commands(document, std::slice::from_ref(id), by)
        })
        .collect();
    commands.extend(edge_commands(plan));
    commands
}

fn edge_commands(plan: &FlowPlan) -> impl Iterator<Item = Command> + '_ {
    (plan.edges.iter()).map(|edge| Command::ReplaceEdge(Box::new(edge.clone())))
}

/// The one step that lays out `entities` as a flow where they stand, as
/// the arrange verb does: moved as a drag would move them, then their
/// edges routed. `None` when nothing would change.
pub fn flow_command(app: &App, entities: &[EntityId], direction: FlowDirection) -> Option<Command> {
    let plan = flow_plan(&app.document, entities, direction, None)?;
    let mut commands: Vec<Command> = place_command(app, &plan.targets).into_iter().collect();
    commands.extend(edge_commands(&plan));
    match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Batch(commands)),
    }
}
