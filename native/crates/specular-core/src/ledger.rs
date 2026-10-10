//! The resize ledger: one event log for the clocks a page resize crosses.
//!
//! Off unless `SPECULAR_RESIZE_TRACE` names a file when [`init_from_env`]
//! runs. While off, every call here is one atomic load. While on, an entry
//! is a fixed-size value put into a ring allocated up front, so the thread
//! being measured does no I/O and no allocation for it. [`flush`] writes
//! the ring as JSON lines, one entry a line, oldest first.
//!
//! Times are microseconds since [`init_from_env`], from one monotonic clock.
//! This is the one place in this crate that writes a file, and only when
//! asked to by the environment: it lives here so the CEF source, the
//! compositor and both shells can reach it.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Instant;

use crate::{CssSize, PixelSize};

/// The environment variable naming the file the ledger is written to.
pub const TRACE_ENV: &str = "SPECULAR_RESIZE_TRACE";

/// How many entries the ring holds: a run of a few minutes.
const CAPACITY: usize = 400_000;

/// The most pages one frame entry names.
pub const FRAME_PAGES: usize = 4;

/// A pump shorter than this is counted and not logged.
const SHORT_PUMP_US: u64 = 50;

static ENABLED: AtomicBool = AtomicBool::new(false);
static LEDGER: OnceLock<Ledger> = OnceLock::new();
static ASK_SEQ: AtomicU64 = AtomicU64::new(0);
static SHORT_PUMPS: AtomicU32 = AtomicU32::new(0);

/// A page as one frame drew it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageDrawn {
    /// The page host's id.
    pub page: u64,
    /// The CSS size the page is laid out at to fill its rect.
    pub wanted: CssSize,
    /// The CSS size of the frame the compositor holds for it.
    pub drawn: CssSize,
    /// That frame's size in texels.
    pub texels: PixelSize,
}

/// What CEF said about a shared-texture paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintFacts {
    /// The page host's id.
    pub page: u64,
    /// The size of the surface, in texels.
    pub coded: PixelSize,
    /// Whether that is the texel size the page is asked for now.
    pub matches: bool,
    /// `content_rect` as x, y, width, height.
    pub content: [i32; 4],
    /// `source_size`, and whether CEF says it is set.
    pub source: (i32, i32, bool),
    /// `capture_counter`, and whether CEF says it is set.
    pub counter: (u64, bool),
    /// `timestamp`, as CEF gives it.
    pub timestamp: u64,
}

/// One thing that happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Entry {
    /// `AppKit` began a live resize of the window.
    LiveBegin,
    /// `AppKit` ended the live resize.
    LiveEnd,
    /// The canvas view was given this size, in points.
    WinSize(f32, f32),
    /// GPUI laid the canvas slot out at this size, in points.
    Slot(f32, f32),
    /// A page host was asked for a size or a scale.
    Ask {
        /// The page host's id.
        page: u64,
        /// The ask's number, counted up across the process.
        seq: u64,
        /// The CSS viewport asked for.
        css: CssSize,
        /// The texture scale asked for.
        scale: f32,
        /// What asked: `viewport`, `scale` or `shown`.
        why: &'static str,
    },
    /// CEF read the view rect for the first time since an ask.
    ViewRect(u64, CssSize),
    /// CEF painted a shared texture.
    Paint(PaintFacts),
    /// A paint was dropped at the texture cap.
    PaintDropped(u64),
    /// CEF's message loop was pumped.
    Pump {
        /// How long the pump took.
        dur_us: u64,
        /// Pumps too short to log since the last entry.
        short: u32,
    },
    /// The display link fired.
    Tick,
    /// A canvas frame was drawn. The entry's time is when it started.
    Frame {
        /// How long the frame took, up to the present call returning.
        dur_us: u64,
        /// The surface's size in pixels.
        surface: PixelSize,
        /// Whether the surface was configured again for this frame.
        reconfigured: bool,
        /// How long the drawable took to arrive.
        acquire_us: u64,
        /// The pages drawn, the first [`FRAME_PAGES`] of them.
        pages: [PageDrawn; FRAME_PAGES],
        /// How many pages were drawn.
        page_count: u8,
    },
    /// A shared surface was imported: a miss of the import cache.
    Import(u64, u64),
    /// One turn of the main run loop was busy for this long.
    Loop(u64),
    /// A scripted run began the step of this number that is not a wait.
    Step(u32),
}

/// What a frame in flight has noted so far.
#[derive(Debug, Clone, Copy, Default)]
struct Scratch {
    acquire_us: u64,
    pages: [PageDrawn; FRAME_PAGES],
    page_count: u8,
}

struct Ring {
    entries: Vec<(u64, Entry)>,
    /// Where the next entry goes once the ring is full.
    next: usize,
    scratch: Scratch,
}

struct Ledger {
    start: Instant,
    path: PathBuf,
    ring: Mutex<Ring>,
}

impl Ledger {
    fn ring(&self) -> MutexGuard<'_, Ring> {
        self.ring
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn put(&self, at_us: u64, entry: Entry) {
        let mut ring = self.ring();
        if ring.entries.len() < CAPACITY {
            ring.entries.push((at_us, entry));
        } else {
            let next = ring.next;
            ring.entries[next] = (at_us, entry);
            ring.next = (next + 1) % CAPACITY;
        }
    }
}

/// Turns the ledger on if [`TRACE_ENV`] names a file. Call once, at start.
pub fn init_from_env() {
    let Some(path) = std::env::var_os(TRACE_ENV).filter(|path| !path.is_empty()) else {
        return;
    };
    let made = LEDGER.set(Ledger {
        start: Instant::now(),
        path: PathBuf::from(path),
        ring: Mutex::new(Ring {
            entries: Vec::with_capacity(CAPACITY),
            next: 0,
            scratch: Scratch::default(),
        }),
    });
    if made.is_ok() {
        ENABLED.store(true, Ordering::Release);
    }
}

/// Whether the ledger is on.
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn ledger() -> Option<&'static Ledger> {
    if enabled() { LEDGER.get() } else { None }
}

/// Microseconds since the ledger started, or 0 while it is off.
pub fn now_us() -> u64 {
    ledger().map_or(0, |ledger| ledger.start.elapsed().as_micros() as u64)
}

/// Logs `entry` as happening now.
#[inline]
pub fn record(entry: Entry) {
    if let Some(ledger) = ledger() {
        ledger.put(ledger.start.elapsed().as_micros() as u64, entry);
    }
}

/// Logs an ask of a page host under the next ask number.
pub fn ask(page: u64, css: CssSize, scale: f32, why: &'static str) {
    if !enabled() {
        return;
    }
    let seq = ASK_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    record(Entry::Ask {
        page,
        seq,
        css,
        scale,
        why,
    });
}

/// Logs a pump that started at `started_us`, unless it was a short one.
pub fn pump(started_us: u64) {
    let Some(ledger) = ledger() else {
        return;
    };
    let now = ledger.start.elapsed().as_micros() as u64;
    let dur_us = now.saturating_sub(started_us);
    if dur_us < SHORT_PUMP_US {
        SHORT_PUMPS.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let short = SHORT_PUMPS.swap(0, Ordering::Relaxed);
    ledger.put(started_us, Entry::Pump { dur_us, short });
}

/// Notes how long the frame in flight waited for its drawable.
pub fn note_acquire(acquire_us: u64) {
    if let Some(ledger) = ledger() {
        ledger.ring().scratch.acquire_us = acquire_us;
    }
}

/// Notes a page the frame in flight draws.
pub fn note_page(page: PageDrawn) {
    let Some(ledger) = ledger() else {
        return;
    };
    let mut ring = ledger.ring();
    let at = usize::from(ring.scratch.page_count);
    if let Some(slot) = ring.scratch.pages.get_mut(at) {
        *slot = page;
    }
    ring.scratch.page_count = ring.scratch.page_count.saturating_add(1);
}

/// Forgets what was noted for a frame that was not drawn.
pub fn frame_abandoned() {
    if let Some(ledger) = ledger() {
        ledger.ring().scratch = Scratch::default();
    }
}

/// Logs the frame that started at `started_us` and has just been drawn,
/// with what was noted for it.
pub fn frame(started_us: u64, surface: PixelSize, reconfigured: bool) {
    let Some(ledger) = ledger() else {
        return;
    };
    let now = ledger.start.elapsed().as_micros() as u64;
    let scratch = std::mem::take(&mut ledger.ring().scratch);
    ledger.put(
        started_us,
        Entry::Frame {
            dur_us: now.saturating_sub(started_us),
            surface,
            reconfigured,
            acquire_us: scratch.acquire_us,
            pages: scratch.pages,
            page_count: scratch.page_count,
        },
    );
}

fn line(out: &mut String, at_us: u64, entry: &Entry) {
    let _ = write!(out, "{{\"t\":{at_us},");
    let _ = match *entry {
        Entry::LiveBegin => write!(out, "\"e\":\"live.begin\""),
        Entry::LiveEnd => write!(out, "\"e\":\"live.end\""),
        Entry::WinSize(w, h) => write!(out, "\"e\":\"win.size\",\"w\":{w},\"h\":{h}"),
        Entry::Slot(w, h) => write!(out, "\"e\":\"slot\",\"w\":{w},\"h\":{h}"),
        Entry::Ask {
            page,
            seq,
            css,
            scale,
            why,
        } => write!(
            out,
            "\"e\":\"ask\",\"page\":{page},\"seq\":{seq},\"w\":{},\"h\":{},\"scale\":{scale},\"why\":\"{why}\"",
            css.width, css.height
        ),
        Entry::ViewRect(page, css) => write!(
            out,
            "\"e\":\"cef.view_rect\",\"page\":{page},\"w\":{},\"h\":{}",
            css.width, css.height
        ),
        Entry::Paint(paint) => write!(
            out,
            "\"e\":\"cef.paint\",\"page\":{},\"w\":{},\"h\":{},\"matches\":{},\
             \"content_rect\":{:?},\"source_size\":[{},{}],\"has_source_size\":{},\
             \"capture_counter\":{},\"has_capture_counter\":{},\"timestamp\":{}",
            paint.page,
            paint.coded.width,
            paint.coded.height,
            paint.matches,
            paint.content,
            paint.source.0,
            paint.source.1,
            paint.source.2,
            paint.counter.0,
            paint.counter.1,
            paint.timestamp
        ),
        Entry::PaintDropped(page) => write!(out, "\"e\":\"cef.drop\",\"page\":{page}"),
        Entry::Pump { dur_us, short } => {
            write!(out, "\"e\":\"pump\",\"dur\":{dur_us},\"short\":{short}")
        }
        Entry::Tick => write!(out, "\"e\":\"tick\""),
        Entry::Frame {
            dur_us,
            surface,
            reconfigured,
            acquire_us,
            pages,
            page_count,
        } => {
            let _ = write!(
                out,
                "\"e\":\"frame\",\"dur\":{dur_us},\"sw\":{},\"sh\":{},\"reconfigured\":{reconfigured},\
                 \"acquire\":{acquire_us},\"page_count\":{page_count},\"pages\":[",
                surface.width, surface.height
            );
            let shown = usize::from(page_count).min(FRAME_PAGES);
            for (at, page) in pages.iter().take(shown).enumerate() {
                let _ = write!(
                    out,
                    "{}{{\"page\":{},\"want\":[{},{}],\"drawn\":[{},{}],\"texels\":[{},{}]}}",
                    if at == 0 { "" } else { "," },
                    page.page,
                    page.wanted.width,
                    page.wanted.height,
                    page.drawn.width,
                    page.drawn.height,
                    page.texels.width,
                    page.texels.height
                );
            }
            write!(out, "]")
        }
        Entry::Import(page, dur_us) => {
            write!(out, "\"e\":\"import\",\"page\":{page},\"dur\":{dur_us}")
        }
        Entry::Loop(busy_us) => write!(out, "\"e\":\"loop\",\"busy\":{busy_us}"),
        Entry::Step(n) => write!(out, "\"e\":\"step\",\"n\":{n}"),
    };
    out.push_str("}\n");
}

/// The ledger so far as JSON lines, oldest entry first.
fn text(ring: &Ring) -> String {
    let mut out = String::with_capacity(ring.entries.len() * 96);
    let (newer, older) = ring.entries.split_at(ring.next);
    for (at_us, entry) in older.iter().chain(newer) {
        line(&mut out, *at_us, entry);
    }
    out
}

/// Writes the ledger to its file. Does nothing while the ledger is off, and
/// may be called again: each call writes everything held.
pub fn flush() {
    let Some(ledger) = ledger() else {
        return;
    };
    let text = text(&ledger.ring());
    if let Err(error) = std::fs::write(&ledger.path, text) {
        eprintln!(
            "specular: writing the resize ledger to {}: {error}",
            ledger.path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_ring_keeps_the_newest_entries_and_writes_them_oldest_first() {
        let mut ring = Ring {
            entries: vec![
                (30, Entry::Tick),
                (10, Entry::LiveBegin),
                (20, Entry::WinSize(800.0, 600.5)),
            ],
            next: 1,
            scratch: Scratch::default(),
        };
        ring.entries[0] = (
            30,
            Entry::Frame {
                dur_us: 900,
                surface: PixelSize::new(1600, 1200),
                reconfigured: true,
                acquire_us: 40,
                pages: [PageDrawn {
                    page: 1,
                    wanted: CssSize::new(800, 600),
                    drawn: CssSize::new(780, 600),
                    texels: PixelSize::new(1560, 1200),
                }; FRAME_PAGES],
                page_count: 1,
            },
        );
        let text = text(&ring);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines,
            [
                r#"{"t":10,"e":"live.begin"}"#,
                r#"{"t":20,"e":"win.size","w":800,"h":600.5}"#,
                r#"{"t":30,"e":"frame","dur":900,"sw":1600,"sh":1200,"reconfigured":true,"acquire":40,"page_count":1,"pages":[{"page":1,"want":[800,600],"drawn":[780,600],"texels":[1560,1200]}]}"#,
            ]
        );
    }
}
