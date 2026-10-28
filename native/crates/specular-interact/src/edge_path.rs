//! Where an edge runs on screen: the curve between its two entities, or
//! the route a flow laid out for it while its ends still meet them, and
//! whether a point is on it.

use glam::{DVec2, Vec2};
use specular_doc::{Edge, EdgeId, EdgeSide, Rect};

use crate::App;
use crate::geometry::ScreenRect;

/// How far outside its entity an edge starts and ends, in logical pixels.
/// An anchor's dot sits here too.
pub(crate) const ANCHOR_OFFSET: f32 = 8.0;
/// The control points sit between these distances from their ends, in canvas
/// units, so a short edge still leaves its entity square-on and a long one
/// does not balloon.
const CONTROL_MIN: f32 = 40.0;
const CONTROL_MAX: f32 = 200.0;
/// Width of the band along an edge where a press selects it, in logical
/// pixels at zoom 1.
const HIT_WIDTH: f32 = 14.0;
/// Straight pieces an edge is cut into for the distance test.
const SEGMENTS: u8 = 32;
/// How far a stored route's end may sit off its entity's side, in canvas
/// units, and still meet it.
const ON_SIDE: f64 = 0.5;

/// The scale of zoom-following hit targets (edge bands, anchor boxes): they
/// shrink with the canvas when zoomed out, but never below this much of
/// their size, and never grow past it when zoomed in.
pub(crate) fn hit_scale(zoom: f32) -> f32 {
    zoom.clamp(0.35, 1.0)
}

/// An edge's cubic bezier in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeCurve {
    /// Where the edge leaves its first entity.
    pub from: Vec2,
    /// The control point for `from`.
    pub from_control: Vec2,
    /// The control point for `to`.
    pub to_control: Vec2,
    /// Where the edge reaches its second entity.
    pub to: Vec2,
}

impl EdgeCurve {
    fn between(from: ScreenRect, to: ScreenRect, edge: &Edge, zoom: f32) -> Self {
        let (from_side, to_side) = match (edge.from_side, edge.to_side) {
            (Some(from_side), Some(to_side)) => (from_side, to_side),
            (None, _) | (_, None) => facing_sides(from, to),
        };
        Self::joining(
            (anchor_point(from, from_side), from_side),
            (anchor_point(to, to_side), to_side),
            zoom,
        )
    }

    /// The curve between two points, each leaving in the direction of the
    /// side it is on.
    pub(crate) fn joining(from: (Vec2, EdgeSide), to: (Vec2, EdgeSide), zoom: f32) -> Self {
        let ((start, from_side), (end, to_side)) = (from, to);
        let reach = (start.distance(end) * 0.4).clamp(CONTROL_MIN * zoom, CONTROL_MAX * zoom);
        Self {
            from: start,
            from_control: start + outward(from_side) * reach,
            to_control: end + outward(to_side) * reach,
            to: end,
        }
    }

    /// The point a fraction `t` of the way along, `0.0` to `1.0`.
    pub fn point(&self, t: f32) -> Vec2 {
        let u = 1.0 - t;
        self.from * (u * u * u)
            + self.from_control * (3.0 * u * u * t)
            + self.to_control * (3.0 * u * t * t)
            + self.to * (t * t * t)
    }

    /// The distance from `point` to the nearest part of the curve.
    fn distance_to(&self, point: Vec2) -> f32 {
        let step = |index: u8| self.point(f32::from(index) / f32::from(SEGMENTS));
        (0..SEGMENTS)
            .map(|index| distance_to_segment(point, step(index), step(index + 1)))
            .fold(f32::INFINITY, f32::min)
    }

    /// Whether a press at `screen` is on the edge.
    pub(crate) fn hit(&self, screen: Vec2, zoom: f32) -> bool {
        self.distance_to(screen) <= HIT_WIDTH * hit_scale(zoom) / 2.0
    }
}

/// A laid-out edge's route in logical screen pixels: right-angle runs from
/// where it leaves its first entity to where it reaches its second.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRoute {
    /// The corners, ends included. There are always at least two.
    pub points: Vec<Vec2>,
}

impl EdgeRoute {
    fn runs(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.points.windows(2).map(|pair| (pair[0], pair[1]))
    }

    /// The middle of the longest run that goes the way the route leaves,
    /// which is where its label sits: across the flow from the screens, on
    /// a run with room for it.
    fn middle(&self) -> Vec2 {
        let first = self.points[1] - self.points[0];
        let along = |(a, b): &(Vec2, Vec2)| {
            let run = *b - *a;
            (run.x == 0.0) == (first.x == 0.0) || run.length_squared() == 0.0
        };
        let longest = (self.runs())
            .filter(along)
            .max_by(|a, b| a.0.distance(a.1).total_cmp(&b.0.distance(b.1)));
        longest.map_or(self.points[0], |(a, b)| (a + b) / 2.0)
    }
}

/// Where an edge runs on screen.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgePath {
    /// A curve between the sides of its two entities.
    Curve(EdgeCurve),
    /// The route a flow laid out for it.
    Route(EdgeRoute),
}

impl EdgePath {
    /// Where the edge leaves its first entity.
    pub fn from(&self) -> Vec2 {
        match self {
            Self::Curve(curve) => curve.from,
            Self::Route(route) => route.points[0],
        }
    }

    /// Where the edge reaches its second entity.
    pub fn to(&self) -> Vec2 {
        match self {
            Self::Curve(curve) => curve.to,
            Self::Route(route) => route.points[route.points.len() - 1],
        }
    }

    /// Where its label sits.
    pub fn middle(&self) -> Vec2 {
        match self {
            Self::Curve(curve) => curve.point(0.5),
            Self::Route(route) => route.middle(),
        }
    }

    /// The points that bound what is drawn.
    pub fn hull(&self) -> Vec<Vec2> {
        match self {
            Self::Curve(curve) => vec![curve.from, curve.from_control, curve.to_control, curve.to],
            Self::Route(route) => route.points.clone(),
        }
    }

    /// The point just before `to`, which the edge arrives from.
    pub fn before_to(&self) -> Vec2 {
        match self {
            Self::Curve(curve) => curve.to_control,
            Self::Route(route) => route.points[route.points.len() - 2],
        }
    }

    /// The point just after `from`, which the edge leaves towards.
    pub fn after_from(&self) -> Vec2 {
        match self {
            Self::Curve(curve) => curve.from_control,
            Self::Route(route) => route.points[1],
        }
    }

    /// Whether a press at `screen` is on the edge.
    pub(crate) fn hit(&self, screen: Vec2, zoom: f32) -> bool {
        match self {
            Self::Curve(curve) => curve.hit(screen, zoom),
            Self::Route(route) => {
                let distance = (route.runs())
                    .map(|(a, b)| distance_to_segment(screen, a, b))
                    .fold(f32::INFINITY, f32::min);
                distance <= HIT_WIDTH * hit_scale(zoom) / 2.0
            }
        }
    }
}

impl App {
    /// Where the edge `id` runs on screen, between its entities where they
    /// are seen: its stored route while both its ends still meet them,
    /// else a curve. `None` when the edge or either entity it names is
    /// missing, or has scrolled out of its page.
    pub fn edge_path(&self, id: &EdgeId) -> Option<EdgePath> {
        let edge = self.document.edge(id)?;
        let camera = &self.session.camera;
        let shown = |entity| crate::shown_rect(self, self.document.entity(entity)?);
        let (from, to) = (shown(&edge.from)?, shown(&edge.to)?);
        if let Some(route) = meeting_route(edge, from, to) {
            let mut points: Vec<Vec2> = (route.iter())
                .map(|point| camera.world_to_screen(point.as_vec2()))
                .collect();
            pull_in(&mut points);
            return Some(EdgePath::Route(EdgeRoute { points }));
        }
        let on_screen = |rect| crate::geometry::on_screen(camera, rect);
        Some(EdgePath::Curve(EdgeCurve::between(
            on_screen(from),
            on_screen(to),
            edge,
            camera.zoom,
        )))
    }
}

/// `edge`'s stored route in canvas units, when it has one whose ends sit on
/// the sides it names of `from` and `to`.
fn meeting_route(edge: &Edge, from: Rect, to: Rect) -> Option<Vec<DVec2>> {
    let route = edge.route.as_ref().filter(|route| route.len() >= 2)?;
    let points: Vec<DVec2> = route.iter().map(|&[x, y]| DVec2::new(x, y)).collect();
    let meets = |rect: Rect, side: Option<EdgeSide>, point: DVec2| {
        let (min, max) = (
            DVec2::new(rect.x, rect.y),
            DVec2::new(rect.x + rect.width, rect.y + rect.height),
        );
        let within = point.cmpge(min - ON_SIDE).all() && point.cmple(max + ON_SIDE).all();
        let on = match side {
            Some(EdgeSide::Top) => (point.y - min.y).abs() <= ON_SIDE,
            Some(EdgeSide::Bottom) => (point.y - max.y).abs() <= ON_SIDE,
            Some(EdgeSide::Left) => (point.x - min.x).abs() <= ON_SIDE,
            Some(EdgeSide::Right) => (point.x - max.x).abs() <= ON_SIDE,
            None => false,
        };
        within && on
    };
    let (first, last) = (points[0], points[points.len() - 1]);
    (meets(from, edge.from_side, first) && meets(to, edge.to_side, last)).then_some(points)
}

/// Starts and ends a route [`ANCHOR_OFFSET`] off its entities, as a curve
/// does, where its end runs are long enough to give that up.
fn pull_in(points: &mut [Vec2]) {
    let count = points.len();
    for (end, next) in [(0, 1), (count - 1, count - 2)] {
        let run = points[next] - points[end];
        if run.length() > 2.0 * ANCHOR_OFFSET {
            points[end] += run.normalize() * ANCHOR_OFFSET;
        }
    }
}

impl App {
    /// `entity` on screen where it is seen, which for one that follows its
    /// page is not where it is stored. `None` when it is hidden.
    pub(crate) fn shown_on_screen(&self, entity: &specular_doc::Entity) -> Option<ScreenRect> {
        let rect = crate::shown_rect(self, entity)?;
        Some(crate::geometry::on_screen(&self.session.camera, rect))
    }
}

/// The middle of `side` of `rect`.
pub(crate) fn side_point(rect: ScreenRect, side: EdgeSide) -> Vec2 {
    let (centre, max) = (rect.centre(), rect.max());
    match side {
        EdgeSide::Top => Vec2::new(centre.x, rect.min().y),
        EdgeSide::Bottom => Vec2::new(centre.x, max.y),
        EdgeSide::Left => Vec2::new(rect.min().x, centre.y),
        EdgeSide::Right => Vec2::new(max.x, centre.y),
    }
}

/// Where an edge meets `side` of `rect`, and where that side's anchor dot
/// is: [`ANCHOR_OFFSET`] outside the middle of the side.
pub(crate) fn anchor_point(rect: ScreenRect, side: EdgeSide) -> Vec2 {
    side_point(rect, side) + outward(side) * ANCHOR_OFFSET
}

/// The unit vector pointing away from an entity through `side`.
pub(crate) const fn outward(side: EdgeSide) -> Vec2 {
    match side {
        EdgeSide::Top => Vec2::NEG_Y,
        EdgeSide::Bottom => Vec2::Y,
        EdgeSide::Left => Vec2::NEG_X,
        EdgeSide::Right => Vec2::X,
    }
}

/// The sides two entities face each other with, for an edge that names none.
pub(crate) fn facing_sides(from: ScreenRect, to: ScreenRect) -> (EdgeSide, EdgeSide) {
    let delta = Vec2::from(to.centre()) - Vec2::from(from.centre());
    if delta.x.abs() > delta.y.abs() {
        if delta.x > 0.0 {
            (EdgeSide::Right, EdgeSide::Left)
        } else {
            (EdgeSide::Left, EdgeSide::Right)
        }
    } else if delta.y > 0.0 {
        (EdgeSide::Bottom, EdgeSide::Top)
    } else {
        (EdgeSide::Top, EdgeSide::Bottom)
    }
}

/// The distance from `point` to the nearest part of the segment `a` to `b`.
pub(crate) fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    if length_squared == 0.0 {
        return point.distance(a);
    }
    let t = ((point - a).dot(along) / length_squared).clamp(0.0, 1.0);
    point.distance(a + along * t)
}
