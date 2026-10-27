//! The ledger as JSON lines.

use std::fmt::Write as _;

use super::{Entry, FRAME_PAGES, PageDrawn, Ring};

/// The pages a frame drew, as a JSON list's items.
fn frame_pages(out: &mut String, pages: &[PageDrawn], page_count: u8) {
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
            frame_pages(out, &pages, page_count);
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
pub(super) fn text(ring: &Ring) -> String {
    let mut out = String::with_capacity(ring.entries.len() * 96);
    let (newer, older) = ring.entries.split_at(ring.next);
    for (at_us, entry) in older.iter().chain(newer) {
        line(&mut out, *at_us, entry);
    }
    out
}
