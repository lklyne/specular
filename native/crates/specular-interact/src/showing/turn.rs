//! A page turning on to its side, or back up.
//!
//! Nothing stored is animated. The page's rect, its orientation and its
//! host change at once, and what is drawn turns: the page's shell starts a
//! quarter turn back from where it now lies, which is where it was, and the
//! spring of the camera transitions carries it round. The frame inside
//! stays upright. A turn runs only when the shell turns transitions on
//! ([`Event::Motion`](crate::Event)).

use std::f64::consts::FRAC_PI_2;

use glam::DVec2;
use specular_doc::{Entity, EntityId, Rect};

use super::motion::{self, spring};
use super::{App, Lens, framed, presented_rect};
use crate::app::page_of;
use crate::property::is_landscape;

/// One page on its way round.
#[derive(Debug, Clone, PartialEq)]
struct Turn {
    page: EntityId,
    /// The orientation it is turning to.
    landscape: bool,
    /// The angle it started at, in radians, clockwise.
    from: f64,
    /// Its shell before the turn, in canvas units.
    was: Rect,
    /// The clock at the first tick after it began.
    started_ms: Option<u64>,
}

impl Turn {
    /// How far along it is at `now_ms`, or `None` once it has arrived.
    fn progress(&self, now_ms: u64) -> Option<f64> {
        let elapsed_ms = self.started_ms.map_or(0, |at| now_ms.saturating_sub(at));
        let (progress, arrived) = spring(elapsed_ms as f64 / 1000.0);
        (!arrived).then_some(progress)
    }
}

/// The turns in flight.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Turns(Vec<Turn>);

/// Where a turning page is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Turning {
    /// The angle its shell is drawn at, in radians, clockwise on screen. It
    /// ends at 0.
    pub angle: f32,
    /// How far along the turn is: 0 at its start and 1 at its end, and a
    /// little past 1 on the way.
    pub progress: f32,
    /// The point the shell turns about: its centre, in canvas units.
    pub pivot: DVec2,
    /// Its shell before the turn, in canvas units.
    pub was: Rect,
}

/// A page as it was before a change that may have turned it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Before {
    page: EntityId,
    landscape: bool,
    shell: Rect,
    /// The angle a turn in flight had reached, 0 with none.
    reached: f64,
    seen: Option<motion::Seen>,
}

/// The pages of the selection as they are, to hand to [`begin`] after a
/// change. Empty when transitions are off.
pub(crate) fn before(app: &App) -> Vec<Before> {
    if !app.session.motion.enabled() {
        return Vec::new();
    }
    (app.session.selection.entities())
        .filter_map(|id| app.document.entity(id))
        // A page that fills the view has no shape of its own to turn.
        .filter(|entity| presented_rect(app, entity).is_none())
        .filter_map(|entity| {
            Some(Before {
                page: entity.id.clone(),
                landscape: is_landscape(page_of(entity)?),
                shell: framed(entity),
                reached: (app.turning(&entity.id)).map_or(0.0, |it| f64::from(it.angle)),
                seen: motion::seen(app, entity),
            })
        })
        .collect()
}

/// Starts the turn of every page of `before` whose orientation changed. A
/// page still turning carries on from the angle it had reached. The camera
/// of a page shown alone goes with it, at the scale the page had.
pub(crate) fn begin(app: &mut App, before: Vec<Before>) {
    for was in before {
        let Some(entity) = app.document.entity(&was.page) else {
            continue;
        };
        let landscape = page_of(entity).is_some_and(is_landscape);
        if landscape == was.landscape {
            continue;
        }
        let shell = framed(entity);
        let shown = app.shown_item() == Some(&was.page) && app.lens() == Some(Lens::Device);
        // Lying down is a quarter turn clockwise, so the shell that now
        // lies down stood a quarter turn back.
        let from = was.reached + if landscape { -FRAC_PI_2 } else { FRAC_PI_2 };
        let turns = &mut app.session.turns.0;
        turns.retain(|turn| turn.page != was.page);
        turns.push(Turn {
            page: was.page,
            landscape,
            from,
            was: was.shell,
            started_ms: None,
        });
        if let Some(seen) = was.seen.filter(|_| shown) {
            // The shell is as wide now as it was tall, so the camera starts
            // where that width is the height that was seen.
            let seen = seen.as_wide_as(shell.width / was.shell.width.max(f64::EPSILON));
            app.session.motion.begin(Some(seen), false);
        }
    }
}

/// Starts the clock of each turn that has not started it, and drops the
/// ones that have arrived or whose page is gone or no longer lies the way
/// it was turning to.
pub(crate) fn on_tick(app: &mut App) {
    let now = app.session.now_ms;
    let document = &app.document;
    app.session.turns.0.retain_mut(|turn| {
        turn.started_ms.get_or_insert(now);
        let page = document.entity(&turn.page).and_then(page_of);
        turn.progress(now).is_some() && page.is_some_and(|it| is_landscape(it) == turn.landscape)
    });
}

impl App {
    /// Where `id` is in its turn, or `None` for an item that is not
    /// turning. A page whose orientation is no longer the one it was
    /// turning to (an undo) is not turning.
    pub fn turning(&self, id: &EntityId) -> Option<Turning> {
        let turn = self.session.turns.0.iter().find(|turn| turn.page == *id)?;
        let entity = self.document.entity(id)?;
        let progress = turn.progress(self.session.now_ms)?;
        let shell = framed(entity);
        (is_landscape(page_of(entity)?) == turn.landscape).then(|| Turning {
            angle: (turn.from * (1.0 - progress)) as f32,
            progress: progress as f32,
            pivot: DVec2::new(shell.x + shell.width / 2.0, shell.y + shell.height / 2.0),
            was: turn.was,
        })
    }

    /// Whether a page is turning, so the shell keeps ticking and drawing
    /// until it lands.
    pub fn turn_running(&self) -> bool {
        !self.session.turns.0.is_empty()
    }
}

/// The shell of `entity` as drawn: where it was at the start of a turn,
/// where it is at the end, and between the two on the way. The chrome that
/// stays upright around a page follows it.
pub fn turning_rect(app: &App, entity: &Entity) -> Option<Rect> {
    let turning = app.turning(&entity.id)?;
    let (to, t) = (framed(entity), f64::from(turning.progress));
    let lerp = |from: f64, to: f64| from + (to - from) * t;
    Some(Rect::new(
        lerp(turning.was.x, to.x),
        lerp(turning.was.y, to.y),
        lerp(turning.was.width, to.width),
        lerp(turning.was.height, to.height),
    ))
}
