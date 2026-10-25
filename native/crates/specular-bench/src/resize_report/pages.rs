//! The page side of a resize ledger: asks, CEF's pickups and paints, and
//! the frames that drew them.

use serde::Serialize;
use specular_core::CssSize;

use super::{Ev, Spread, ms};

/// A gap between asks longer than this starts a new switch. A switch's own
/// asks come within the texture scale's settle time of one another.
const SWITCH_GAP_US: u64 = 400_000;

/// What the pages were asked for and what they painted.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PageSide {
    /// Asks made of page hosts: every `was_resized`.
    pub(crate) asks: usize,
    /// Asks for another texel size than the page's ask before.
    pub(crate) requests: usize,
    /// Shared-texture paints of a page's view.
    pub(crate) paints: usize,
    /// Paints of the texel size asked for at the time.
    pub(crate) paints_matching: usize,
    /// Requests no paint of their size came for before the same size was
    /// requested again.
    pub(crate) unanswered: usize,
    /// Paints dropped at the texture cap.
    pub(crate) dropped: usize,
    /// Paints whose `content_rect` is smaller than the surface.
    pub(crate) letterboxed: usize,
    /// Paints whose `source_size` is set and is not the size of their
    /// `content_rect`: content captured at one size and scaled to another.
    pub(crate) rescaled: usize,
    /// Paints with `has_capture_counter` set.
    pub(crate) with_capture_counter: usize,
    /// Paints with `has_source_size` set.
    pub(crate) with_source_size: usize,
    /// From the first ask CEF had not read to its reading the view rect.
    pub(crate) ask_to_view_rect: Spread,
    /// From that reading to the first paint of the size read.
    pub(crate) view_rect_to_paint: Spread,
    /// From a paint to the end of the first frame started after it that
    /// drew a texture of the paint's size.
    pub(crate) paint_to_frame: Spread,
}

/// The asks one scripted step led to, outside a live resize. In a ledger
/// with no steps, a burst of asks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Switch {
    /// How many asks it made.
    pub(crate) asks: usize,
    /// From the step, or the first ask of a burst, to the end of the first
    /// frame after it that drew the page at the last ask's CSS size, at
    /// any texel size. `None` if none did.
    pub(crate) css_ms: Option<f64>,
    /// The same for a frame that also has the last ask's texel size.
    pub(crate) settle_ms: Option<f64>,
}

fn texels(event: &Ev, scale: f32) -> [u32; 2] {
    let size = CssSize::new(event.w as u32, event.h as u32).to_pixels(scale);
    [size.width, size.height]
}

/// The end of the first frame started at `from` or later, and before
/// `until`, that drew `page` as `fits`.
fn drawn_at(
    frames: &[&Ev],
    page: u64,
    (from, until): (u64, u64),
    fits: impl Fn(&super::PageEv) -> bool,
) -> Option<u64> {
    (frames.iter())
        .filter(|frame| frame.t >= from && frame.t < until)
        .find(|frame| frame.pages.iter().any(|p| p.page == page && fits(p)))
        .map(|frame| frame.end())
}

/// The numbers of the page side of `events`.
pub(crate) fn page_side(events: &[Ev]) -> PageSide {
    let of = |kind: &'static str| events.iter().filter(move |event| event.e == kind);
    let frames: Vec<&Ev> = of("frame").collect();
    let paints: Vec<&Ev> = of("cef.paint").collect();
    let asks: Vec<&Ev> = of("ask").collect();
    let requests = requests(&asks);
    let (to_view_rect, to_paint) =
        view_rect_waits(&of("cef.view_rect").collect::<Vec<_>>(), &asks, &paints);
    let rescaled = |paint: &&&Ev| {
        let [_, _, w, h] = paint.content_rect;
        paint.has_source_size && paint.source_size != [w, h]
    };
    let letterboxed = |paint: &&&Ev| {
        let [_, _, w, h] = paint.content_rect;
        f64::from(w) < paint.w || f64::from(h) < paint.h
    };
    PageSide {
        asks: asks.len(),
        requests: requests.len(),
        paints: paints.len(),
        paints_matching: paints.iter().filter(|paint| paint.matches).count(),
        unanswered: unanswered(&requests, &paints),
        dropped: of("cef.drop").count(),
        letterboxed: paints.iter().filter(letterboxed).count(),
        rescaled: paints.iter().filter(rescaled).count(),
        with_capture_counter: (paints.iter())
            .filter(|paint| paint.has_capture_counter)
            .count(),
        with_source_size: paints.iter().filter(|paint| paint.has_source_size).count(),
        ask_to_view_rect: Spread::of(&to_view_rect),
        view_rect_to_paint: Spread::of(&to_paint),
        paint_to_frame: Spread::of(&paint_to_frame(&paints, &frames)),
    }
}

/// The first paint of `size` for `page` in a span of time.
fn painted(paints: &[&Ev], page: u64, size: [u32; 2], from: u64, until: u64) -> Option<u64> {
    (paints.iter())
        .find(|paint| {
            paint.page == page
                && paint.t > from
                && paint.t < until
                && [paint.w as u32, paint.h as u32] == size
        })
        .map(|paint| paint.t)
}

/// The requests among `asks`: an ask for another texel size than the
/// page's last, as `(page, time, size)`.
fn requests(asks: &[&Ev]) -> Vec<(u64, u64, [u32; 2])> {
    let mut requests: Vec<(u64, u64, [u32; 2])> = Vec::new();
    for ask in asks {
        let size = texels(ask, ask.scale);
        let last = requests.iter().rev().find(|(page, ..)| *page == ask.page);
        if last.is_none_or(|(_, _, held)| *held != size) {
            requests.push((ask.page, ask.t, size));
        }
    }
    requests
}

/// How many requests no paint of their size answered before the same size
/// was asked again.
fn unanswered(requests: &[(u64, u64, [u32; 2])], paints: &[&Ev]) -> usize {
    let again = |at: usize, page: u64, size: [u32; 2]| {
        (requests[at + 1..].iter())
            .find(|(other, _, held)| *other == page && *held == size)
            .map_or(u64::MAX, |(_, t, _)| *t)
    };
    (requests.iter().enumerate())
        .filter(|&(at, &(page, t, size))| {
            painted(paints, page, size, t, again(at, page, size)).is_none()
        })
        .count()
}

/// For each time CEF read a page's view rect: how long after the ask it
/// answers, and how long before a paint of that size followed.
fn view_rect_waits(reads: &[&Ev], asks: &[&Ev], paints: &[&Ev]) -> (Vec<u64>, Vec<u64>) {
    let mut to_view_rect = Vec::new();
    let mut to_paint = Vec::new();
    for read in reads {
        // The ask it answers is the first one since CEF's read before.
        let before = (reads.iter())
            .filter(|other| other.page == read.page && other.t < read.t)
            .map(|other| other.t)
            .max()
            .unwrap_or(0);
        let mine = |ask: &&&Ev| ask.page == read.page && ask.t <= read.t;
        let first = asks.iter().filter(mine).find(|ask| ask.t >= before);
        to_view_rect.extend(first.map(|ask| read.t - ask.t));
        let scale = (asks.iter().rfind(mine)).map_or(1.0, |ask| ask.scale);
        let size = texels(read, scale);
        let next_read = (reads.iter())
            .find(|other| other.page == read.page && other.t > read.t)
            .map_or(u64::MAX, |other| other.t);
        to_paint.extend(painted(paints, read.page, size, read.t, next_read).map(|t| t - read.t));
    }
    (to_view_rect, to_paint)
}

/// How long each paint waited to be drawn. A paint is there to be drawn
/// until one of another size replaces it.
fn paint_to_frame(paints: &[&Ev], frames: &[&Ev]) -> Vec<u64> {
    (paints.iter().enumerate())
        .filter_map(|(at, paint)| {
            let size = [paint.w as u32, paint.h as u32];
            let replaced = (paints[at + 1..].iter())
                .find(|next| next.page == paint.page && [next.w as u32, next.h as u32] != size)
                .map_or(u64::MAX, |next| next.t);
            drawn_at(frames, paint.page, (paint.t, replaced), |p| {
                p.texels == size
            })
            .map(|end| end - paint.t)
        })
        .collect()
}

/// The asks of each scripted step outside the live resizes `spans`, with
/// the step's time. With no steps in the ledger, each burst of asks with
/// the time of its first.
fn groups<'a>(events: &'a [Ev], spans: &[(u64, u64)]) -> Vec<(u64, Vec<&'a Ev>)> {
    let live = |t: u64| spans.iter().any(|&(from, to)| from <= t && t <= to);
    let asks = (events.iter()).filter(|event| event.e == "ask" && !live(event.t));
    let steps: Vec<u64> = (events.iter())
        .filter(|event| event.e == "step")
        .map(|event| event.t)
        .collect();
    let mut groups: Vec<(u64, Vec<&Ev>)> = steps.iter().map(|&t| (t, Vec::new())).collect();
    for ask in asks {
        if steps.is_empty() {
            match groups.last_mut() {
                Some((_, burst))
                    if burst
                        .last()
                        .is_some_and(|last| ask.t - last.t <= SWITCH_GAP_US) =>
                {
                    burst.push(ask);
                }
                _ => groups.push((ask.t, vec![ask])),
            }
        } else if let Some(at) = steps.iter().rposition(|&step| step <= ask.t) {
            groups[at].1.push(ask);
        }
    }
    groups
}

/// What each scripted step, or burst of asks, outside the live resizes
/// `spans` asked of a page and how long the page took to be drawn so.
pub(crate) fn switches(events: &[Ev], spans: &[(u64, u64)]) -> Vec<Switch> {
    let frames: Vec<&Ev> = events.iter().filter(|event| event.e == "frame").collect();
    (groups(events, spans).iter())
        .filter_map(|(from, asks)| {
            let last = asks.last()?;
            let css = [last.w as u32, last.h as u32];
            let size = texels(last, last.scale);
            let drawn = |fits: &dyn Fn(&super::PageEv) -> bool| {
                drawn_at(&frames, last.page, (*from, u64::MAX), fits).map(|end| ms(end - from))
            };
            Some(Switch {
                asks: asks.len(),
                css_ms: drawn(&|p| p.drawn == css),
                settle_ms: drawn(&|p| p.drawn == css && p.texels == size),
            })
        })
        .collect()
}
