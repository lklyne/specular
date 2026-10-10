//! Reading a resize ledger (`specular_core::ledger`, one JSON object a
//! line) down to the numbers a resize is judged by. Times in the summary
//! are milliseconds.

mod pages;
mod table;

use serde::{Deserialize, Serialize};

pub use self::pages::{PageSide, Switch};
pub use self::table::table;
use crate::stats::percentile;

/// A page as a frame drew it.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PageEv {
    pub(crate) page: u64,
    pub(crate) want: [u32; 2],
    pub(crate) drawn: [u32; 2],
    pub(crate) texels: [u32; 2],
}

/// One line of a ledger. A field an entry does not have is zero.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a ledger line's flags, as the ledger writes them"
)]
pub(crate) struct Ev {
    pub(crate) t: u64,
    pub(crate) e: String,
    pub(crate) w: f64,
    pub(crate) h: f64,
    pub(crate) page: u64,
    pub(crate) scale: f32,
    pub(crate) dur: u64,
    pub(crate) short: u64,
    pub(crate) busy: u64,
    pub(crate) matches: bool,
    pub(crate) sw: u32,
    pub(crate) sh: u32,
    pub(crate) acquire: u64,
    pub(crate) reconfigured: bool,
    pub(crate) pages: Vec<PageEv>,
    pub(crate) content_rect: [i32; 4],
    pub(crate) source_size: [i32; 2],
    pub(crate) has_source_size: bool,
    pub(crate) has_capture_counter: bool,
}

impl Ev {
    /// When the entry's work was over.
    pub(crate) fn end(&self) -> u64 {
        self.t + self.dur
    }
}

/// The middle, the tail and the worst of some times, in milliseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Spread {
    /// How many times there were.
    pub n: usize,
    /// The median.
    pub p50: f64,
    /// The 95th percentile.
    pub p95: f64,
    /// The longest.
    pub max: f64,
}

impl Spread {
    /// The spread of `micros`, which are microseconds.
    pub(crate) fn of(micros: &[u64]) -> Self {
        let mut ms: Vec<f64> = micros.iter().map(|&us| us as f64 / 1_000.0).collect();
        ms.sort_by(f64::total_cmp);
        let Some(&max) = ms.last() else {
            return Self::default();
        };
        Self {
            n: ms.len(),
            p50: percentile(&ms, 0.5),
            p95: percentile(&ms, 0.95),
            max,
        }
    }
}

/// What one run's ledger comes to.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Summary {
    /// The ledger's file name without its extension.
    pub run: String,
    /// Time inside `AppKit` live resizes.
    pub live_ms: f64,
    /// `win.size` steps inside live resizes.
    pub win_steps: usize,
    /// Canvas frames drawn inside live resizes.
    pub live_frames: usize,
    /// Those frames a second of live resize.
    pub live_fps: f64,
    /// Display link ticks a second of live resize.
    pub live_tick_hz: f64,
    /// From a `win.size` step to the end of the first frame whose surface
    /// has that size, for steps that got one before the next step.
    pub win_to_frame: Spread,
    /// Steps that got no frame of their size before the next step.
    pub win_unmatched: usize,
    /// Frames that configured the surface again.
    pub reconfigures: usize,
    /// What the pages were asked for and what they painted.
    pub page: PageSide,
    /// Canvas frames that drew a page.
    pub page_frames: usize,
    /// The share of those that drew a page at another size than wanted.
    pub stale_share: f64,
    /// The widest wanted-minus-drawn gap of a page, in CSS pixels.
    pub widest_gap_css: u32,
    /// Pumps of CEF's message loop, the short ones included.
    pub pumps: u64,
    /// Time in the pumps long enough to be logged.
    pub pump_total_ms: f64,
    /// The longest pump.
    pub pump_max_ms: f64,
    /// The wait for a drawable.
    pub acquire: Spread,
    /// Shared surfaces imported: misses of the import cache.
    pub import_misses: usize,
    /// Time importing them.
    pub import_total_ms: f64,
    /// The share of the run the main run loop was busy.
    pub busy_share: f64,
    /// The same inside live resizes.
    pub live_busy_share: f64,
    /// The longest busy turn of the main run loop.
    pub busy_max_ms: f64,
    /// Each burst of asks outside a live resize: a tab or lens switch.
    pub switches: Vec<Switch>,
}

fn ms(micros: u64) -> f64 {
    micros as f64 / 1_000.0
}

fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// The live-resize spans of a ledger, as start and end times.
fn live_spans(events: &[Ev]) -> Vec<(u64, u64)> {
    let mut spans = Vec::new();
    let mut begun = None;
    for event in events {
        match event.e.as_str() {
            "live.begin" => begun = Some(event.t),
            "live.end" => spans.extend(begun.take().map(|from| (from, event.t))),
            _ => {}
        }
    }
    spans
}

/// The surface a view of `w` by `h` points has at a display scale of one
/// or two.
fn surface_of(step: &Ev, frame: &Ev) -> bool {
    [1.0, 2.0].iter().any(|scale| {
        let pixels = |points: f64| ((points * scale).round() as u32).max(1);
        pixels(step.w) == frame.sw && pixels(step.h) == frame.sh
    })
}

/// Reads a ledger's text. A line that is not an entry is an error.
pub fn parse(text: &str) -> Result<Vec<Event>, serde_json::Error> {
    (text.lines().filter(|line| !line.trim().is_empty()))
        .map(|line| serde_json::from_str::<Ev>(line).map(Event))
        .collect()
}

/// One entry of a ledger.
#[derive(Debug, Clone)]
pub struct Event(Ev);

/// The numbers of the run whose ledger is `events`.
pub fn summarize(run: &str, events: &[Event]) -> Summary {
    let events: Vec<Ev> = events.iter().map(|event| event.0.clone()).collect();
    let of = |kind: &'static str| events.iter().filter(move |event| event.e == kind);
    let spans = live_spans(&events);
    let live = |t: u64| spans.iter().any(|&(from, to)| from <= t && t <= to);
    let live_us: u64 = spans.iter().map(|&(from, to)| to - from).sum();
    let per_live_second = |count: usize| {
        if live_us == 0 {
            0.0
        } else {
            count as f64 / (live_us as f64 / 1e6)
        }
    };

    let frames: Vec<&Ev> = of("frame").collect();
    let steps: Vec<&Ev> = of("win.size").filter(|step| live(step.t)).collect();
    let mut win_lags = Vec::new();
    for (at, step) in steps.iter().enumerate() {
        let until = steps.get(at + 1).map_or(u64::MAX, |next| next.t);
        let matched = (frames.iter())
            .find(|frame| frame.t >= step.t && frame.t < until && surface_of(step, frame));
        win_lags.extend(matched.map(|frame| frame.end() - step.t));
    }
    let live_frames = frames.iter().filter(|frame| live(frame.t)).count();

    let page_frames: Vec<&Ev> = (frames.iter().copied())
        .filter(|frame| !frame.pages.is_empty())
        .collect();
    let stale = |frame: &&&Ev| frame.pages.iter().any(|page| page.want != page.drawn);
    let widest_gap_css = (page_frames.iter())
        .flat_map(|frame| frame.pages.iter())
        .map(|page| {
            let gap = |axis: usize| page.want[axis].saturating_sub(page.drawn[axis]);
            gap(0).max(gap(1))
        })
        .max()
        .unwrap_or(0);

    let (first, last) = (
        events.first().map_or(0, |event| event.t),
        events.last().map_or(0, |event| event.t),
    );
    let busy: u64 = of("loop").map(|turn| turn.busy).sum();
    let live_busy: u64 = of("loop")
        .filter(|turn| live(turn.t))
        .map(|turn| turn.busy)
        .sum();
    let acquires: Vec<u64> = frames.iter().map(|frame| frame.acquire).collect();
    Summary {
        run: run.to_owned(),
        live_ms: ms(live_us),
        win_steps: steps.len(),
        live_frames,
        live_fps: per_live_second(live_frames),
        live_tick_hz: per_live_second(of("tick").filter(|tick| live(tick.t)).count()),
        win_unmatched: steps.len() - win_lags.len(),
        win_to_frame: Spread::of(&win_lags),
        reconfigures: frames.iter().filter(|frame| frame.reconfigured).count(),
        page: pages::page_side(&events),
        page_frames: page_frames.len(),
        stale_share: share(
            page_frames.iter().filter(stale).count() as u64,
            page_frames.len() as u64,
        ),
        widest_gap_css,
        pumps: of("pump").map(|pump| 1 + pump.short).sum(),
        pump_total_ms: ms(of("pump").map(|pump| pump.dur).sum()),
        pump_max_ms: ms(of("pump").map(|pump| pump.dur).max().unwrap_or(0)),
        acquire: Spread::of(&acquires),
        import_misses: of("import").count(),
        import_total_ms: ms(of("import").map(|import| import.dur).sum()),
        busy_share: share(busy, last - first),
        live_busy_share: share(live_busy, live_us),
        busy_max_ms: ms(of("loop").map(|turn| turn.busy).max().unwrap_or(0)),
        switches: pages::switches(&events, &spans),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(ledger: &str) -> Summary {
        summarize("run", &parse(ledger).unwrap())
    }

    #[test]
    fn a_drag_s_steps_are_matched_to_frames_of_their_size_and_its_asks_to_paints() {
        // Two steps of a live resize on a display of scale 2. The first gets
        // no frame of its size before the second; the second gets one 6 ms
        // later. The page is asked for two sizes: CEF picks the first up
        // after 1 ms, paints it 9 ms after that, a frame draws it 2 ms after
        // the paint, and the second size is never painted.
        let ledger = r#"
{"t":1000,"e":"live.begin"}
{"t":2000,"e":"win.size","w":800,"h":600}
{"t":2100,"e":"ask","page":1,"seq":1,"w":800,"h":500,"scale":2,"why":"viewport"}
{"t":3000,"e":"tick"}
{"t":3000,"e":"frame","dur":1000,"sw":1640,"sh":1200,"reconfigured":false,"acquire":300,"page_count":1,"pages":[{"page":1,"want":[800,500],"drawn":[820,500],"texels":[1640,1000]}]}
{"t":3100,"e":"cef.view_rect","page":1,"w":800,"h":500}
{"t":4000,"e":"win.size","w":790,"h":600}
{"t":4100,"e":"ask","page":1,"seq":2,"w":790,"h":500,"scale":2,"why":"viewport"}
{"t":5000,"e":"pump","dur":2000,"short":3}
{"t":9000,"e":"frame","dur":1000,"sw":1580,"sh":1200,"reconfigured":true,"acquire":500,"page_count":1,"pages":[{"page":1,"want":[790,500],"drawn":[820,500],"texels":[1640,1000]}]}
{"t":12100,"e":"cef.paint","page":1,"w":1600,"h":1000,"matches":false,"content_rect":[0,0,1600,1000],"source_size":[0,0],"has_source_size":false,"capture_counter":7,"has_capture_counter":true,"timestamp":0}
{"t":12200,"e":"import","page":1,"dur":400}
{"t":13000,"e":"frame","dur":1100,"sw":1580,"sh":1200,"reconfigured":false,"acquire":100,"page_count":1,"pages":[{"page":1,"want":[790,500],"drawn":[800,500],"texels":[1600,1000]}]}
{"t":15000,"e":"loop","busy":7000}
{"t":21000,"e":"live.end"}
"#;
        let got = summary(ledger);
        assert_eq!((got.live_ms, got.win_steps, got.live_frames), (20.0, 2, 3));
        assert_eq!(got.live_fps, 150.0);
        assert_eq!(got.live_tick_hz, 50.0);
        assert_eq!((got.win_to_frame.n, got.win_unmatched), (1, 1));
        assert_eq!(got.win_to_frame.max, 6.0);
        assert_eq!(got.reconfigures, 1);
        let page = &got.page;
        assert_eq!((page.asks, page.requests, page.paints), (2, 2, 1));
        assert_eq!((page.paints_matching, page.unanswered), (0, 1));
        assert_eq!(page.ask_to_view_rect.max, 1.0);
        assert_eq!(page.view_rect_to_paint.max, 9.0);
        assert_eq!(page.paint_to_frame.max, 2.0);
        assert_eq!((page.letterboxed, page.with_capture_counter), (0, 1));
        assert_eq!(page.rescaled, 0);
        assert_eq!((got.page_frames, got.stale_share), (3, 1.0));
        assert_eq!(got.widest_gap_css, 0);
        assert_eq!((got.pumps, got.pump_max_ms), (4, 2.0));
        assert_eq!((got.import_misses, got.import_total_ms), (1, 0.4));
        assert_eq!((got.live_busy_share, got.busy_max_ms), (0.35, 7.0));
        assert_eq!(got.acquire.max, 0.5);
        assert_eq!(got.switches, []);
    }

    #[test]
    fn a_tab_switch_is_a_burst_of_asks_timed_to_the_first_frame_at_the_last_one_s_size() {
        // The viewport first, the scale 120 ms later, and a frame at both
        // 40 ms after that. A second switch, a second later, never lands.
        let ledger = r#"
{"t":1000,"e":"ask","page":1,"seq":1,"w":1400,"h":800,"scale":1,"why":"viewport"}
{"t":30000,"e":"frame","dur":1000,"page_count":1,"pages":[{"page":1,"want":[1400,800],"drawn":[1400,800],"texels":[1400,800]}]}
{"t":121000,"e":"ask","page":1,"seq":2,"w":1400,"h":800,"scale":2,"why":"scale"}
{"t":160000,"e":"frame","dur":1000,"page_count":1,"pages":[{"page":1,"want":[1400,800],"drawn":[1400,800],"texels":[2800,1600]}]}
{"t":1200000,"e":"ask","page":1,"seq":3,"w":800,"h":500,"scale":1,"why":"viewport"}
"#;
        let got = summary(ledger);
        assert_eq!(
            got.switches,
            [
                Switch {
                    asks: 2,
                    css_ms: Some(30.0),
                    settle_ms: Some(160.0)
                },
                Switch {
                    asks: 1,
                    css_ms: None,
                    settle_ms: None
                }
            ]
        );
    }
}
