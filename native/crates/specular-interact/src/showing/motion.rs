//! The camera transition between the canvas, Device and Fill.
//!
//! Only `session.camera` is animated. The item's layout rect and its page's
//! host change at once; the camera starts where the item's new layout rect
//! has the screen centre and width the old one had, so the item does not
//! jump, and a spring carries it to the camera the view holds. The item's
//! screen centre moves in a line and the zoom in log space, which keeps the
//! motion centred on the item.
//!
//! A transition runs on three changes: the canvas to an item view, an item
//! view to the canvas, and a lens change. It is off unless the shell turns
//! it on ([`Event::Motion`](crate::Event)), and then no state is made.

use glam::DVec2;
use specular_core::Camera;
use specular_doc::{Entity, Rect};

use super::{App, framed, geometry, presented_rect};

/// The period of the spring's undamped oscillation, in seconds.
const RESPONSE_S: f64 = 0.28;
/// How damped the spring is: 1 settles with no overshoot.
const DAMPING_RATIO: f64 = 0.86;
/// How near the target, and how slow, counts as arrived.
const EPSILON: f64 = 1e-3;
/// The screen distance, in pixels, under which a transition has nowhere to go.
const NEGLIGIBLE_PX: f64 = 0.5;

/// The spring's progress `s` at `elapsed_s` seconds, from 0 at the start to
/// 1 at the target, and whether it has arrived. Arrived is exactly 1.
pub(crate) fn spring(elapsed_s: f64) -> (f64, bool) {
    let t = elapsed_s.max(0.0);
    let omega = std::f64::consts::TAU / RESPONSE_S;
    let decay = DAMPING_RATIO * omega;
    let damped = omega * (1.0 - DAMPING_RATIO * DAMPING_RATIO).sqrt();
    let envelope = (-decay * t).exp();
    let (sin, cos) = (damped * t).sin_cos();
    let value = 1.0 - envelope * (cos + decay / damped * sin);
    let rate = omega * omega / damped * envelope * sin;
    if (1.0 - value).abs() < EPSILON && rate.abs() < EPSILON {
        (1.0, true)
    } else {
        (value, false)
    }
}

/// Where an item is on screen: the centre and width of its layout rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seen {
    centre: DVec2,
    width: f64,
}

/// The rect `entity` is laid out in right now.
pub(super) fn layout_rect(app: &App, entity: &Entity) -> Rect {
    presented_rect(app, entity).unwrap_or_else(|| framed(entity))
}

fn centre_of(rect: Rect) -> DVec2 {
    geometry::origin(rect) + geometry::size(rect) / 2.0
}

/// Where `entity` is on screen under the camera and layout now, when
/// transitions are on.
pub(crate) fn seen(app: &App, entity: &Entity) -> Option<Seen> {
    if !app.session.motion.enabled {
        return None;
    }
    let rect = layout_rect(app, entity);
    let camera = app.session.camera;
    let zoom = f64::from(camera.zoom);
    (rect.width > 0.0).then(|| Seen {
        centre: centre_of(rect) * zoom + camera.pan.as_dvec2(),
        width: rect.width * zoom,
    })
}

/// A transition in flight.
#[derive(Debug, Clone, PartialEq)]
struct Active {
    from: Seen,
    /// The centre of the item's layout rect, in canvas units.
    anchor: DVec2,
    /// The width of the item's layout rect, in canvas units.
    layout_width: f64,
    target: Camera,
    /// The clock at the first tick after it began.
    started_ms: Option<u64>,
    leaving: bool,
    id: u64,
}

impl Active {
    fn start_zoom(&self) -> f64 {
        self.from.width / self.layout_width
    }

    fn is_trivial(&self) -> bool {
        let target = self.target;
        let there = self.anchor * f64::from(target.zoom) + target.pan.as_dvec2();
        let zoom = f64::from(target.zoom);
        (there - self.from.centre).length() < NEGLIGIBLE_PX
            && (self.start_zoom() / zoom - 1.0).abs() < 1e-3
    }

    /// The camera `progress` of the way, with the item's centre and the
    /// log of the zoom both interpolated by it.
    fn camera_at(&self, progress: f64) -> Camera {
        let target = self.target;
        let zoom_to = f64::from(target.zoom);
        let centre_to = self.anchor * zoom_to + target.pan.as_dvec2();
        let centre = self.from.centre.lerp(centre_to, progress);
        let zoom_from = self.start_zoom();
        let zoom = zoom_from * (zoom_to / zoom_from).powf(progress);
        Camera::new((centre - self.anchor * zoom).as_vec2(), zoom as f32)
    }
}

/// The transition state of a session.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Motion {
    enabled: bool,
    active: Option<Active>,
    begun: u64,
}

impl Motion {
    pub(crate) fn enable(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.active = None;
        }
    }

    pub(crate) fn cancel(&mut self) {
        self.active = None;
    }

    pub(crate) fn running(&self) -> bool {
        self.active.is_some()
    }

    /// The camera a leave is heading for, while one runs.
    pub(crate) fn leaving_to(&self) -> Option<Camera> {
        (self.active.as_ref().filter(|it| it.leaving)).map(|it| it.target)
    }

    /// The identity of the leave in flight, to tell it from a later one.
    pub(crate) fn leave_id(&self) -> Option<u64> {
        (self.active.as_ref().filter(|it| it.leaving)).map(|it| it.id)
    }

    /// Starts a transition from `from`, or ends any with `None`. Its layout
    /// and target arrive with [`follow`].
    pub(crate) fn begin(&mut self, from: Option<Seen>, leaving: bool) {
        self.begun += 1;
        self.active = from.filter(|_| self.enabled).map(|from| Active {
            from,
            anchor: DVec2::ZERO,
            layout_width: 1.0,
            target: Camera::default(),
            started_ms: None,
            leaving,
            id: self.begun,
        });
    }

    /// Starts the clock of a transition that has not started it.
    pub(crate) fn on_tick(&mut self, now_ms: u64) {
        if let Some(active) = self.active.as_mut().filter(|it| it.started_ms.is_none()) {
            active.started_ms = Some(now_ms);
        }
    }

    /// The camera to show at `now_ms`, with the transition aimed at
    /// `target` for an item laid out in `rect`. Without a transition, or
    /// once it has arrived, that is `target` itself.
    pub(crate) fn follow(&mut self, rect: Rect, target: Camera, now_ms: u64) -> Camera {
        if let Some(active) = self.active.as_mut() {
            active.anchor = centre_of(rect);
            active.layout_width = rect.width.max(f64::EPSILON);
            active.target = target;
        }
        self.sample(now_ms).unwrap_or(target)
    }

    /// The camera of the transition at `now_ms` where it was last aimed,
    /// or `None` once there is none or it has arrived.
    pub(crate) fn sample(&mut self, now_ms: u64) -> Option<Camera> {
        let active = self.active.as_ref()?;
        let elapsed_ms = active.started_ms.map_or(0, |at| now_ms.saturating_sub(at));
        let (progress, arrived) = spring(elapsed_ms as f64 / 1000.0);
        if arrived || (active.started_ms.is_none() && active.is_trivial()) {
            self.active = None;
            return None;
        }
        Some(active.camera_at(progress))
    }
}

/// Where the item that is about to be shown is on the canvas, or `None`
/// when an item view is up: switching from tab to tab is instant.
pub(super) fn seen_on_canvas(app: &App, entity: &Entity) -> Option<Seen> {
    app.shown_item().is_none().then(|| seen(app, entity))?
}

/// Starts the camera of a leave at the item where it was, aimed at
/// `target`, for an item now laid out in `rect`.
pub(super) fn begin_leave(app: &mut App, from: Option<Seen>, rect: Rect, target: Camera) {
    app.session.motion.begin(from, true);
    let now = app.session.now_ms;
    app.session.camera = app.session.motion.follow(rect, target, now);
}

/// Moves a leave along while the canvas is shown. Its target is fixed.
pub(super) fn advance(app: &mut App) {
    let now = app.session.now_ms;
    let motion = &mut app.session.motion;
    let Some(target) = motion.leaving_to() else {
        motion.cancel();
        return;
    };
    app.session.camera = motion.sample(now).unwrap_or(target);
}

/// Ends the leave that was running before an event (`leaving`, its id)
/// when the event was the user's own camera input: it moved the camera,
/// switched canvas or started a gesture. The camera stays where it is.
pub(crate) fn interrupt(app: &mut App, leaving: Option<u64>, before: Camera, switched: bool) {
    let motion = &mut app.session.motion;
    if leaving.is_some()
        && motion.leave_id() == leaving
        && (switched || app.session.camera != before || app.session.gesture.is_some())
    {
        motion.cancel();
    }
}

impl App {
    /// Whether a camera transition is running, so the shell keeps ticking
    /// and drawing until it lands.
    pub fn motion_running(&self) -> bool {
        self.session.motion.running()
    }
}

#[cfg(test)]
mod tests {
    use super::spring;

    #[test]
    fn the_spring_starts_at_zero_overshoots_a_little_and_lands_exactly() {
        assert_eq!(spring(0.0), (0.0, false));
        let peak = (0..2000)
            .map(|step| spring(f64::from(step) / 1000.0).0)
            .fold(0.0_f64, f64::max);
        assert!((1.0..1.02).contains(&peak), "peak {peak}");
        assert_eq!(spring(1.0), (1.0, true));
        assert!(!spring(0.1).1);
    }
}
