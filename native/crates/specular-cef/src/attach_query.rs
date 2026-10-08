//! What element attachment (ADR 0032) asks of a page through the devtools
//! protocol, and reading its answers. Pure, like
//! [`dom_query`](crate::dom_query), whose helpers it shares.
//!
//! Two questions. A page is asked which element an item placed over a
//! document point should follow. And a page is asked where the elements
//! items already follow sit now, which it answers when one has moved.
//!
//! The DOM rules are the Electron preloads' (`element-attachment-capture.ts`
//! and the attachment half of `annotation-bbox-tracker.ts`).

use glam::Vec2;
use serde::Deserialize;
use serde_json::json;
use specular_core::{CapturedElement, ElementPlace};

use crate::dom_query::{HELPERS, returned};

/// Whether an element sits in a fixed or sticky container, so that it moves
/// through the document as the page scrolls.
const POSITIONED: &str = r"
const positioned = (e) => {
  for (let c = e; c && c !== document.body; c = c.parentElement) {
    const p = getComputedStyle(c).position;
    if (p === 'fixed' || p === 'sticky') return true;
  }
  return false;
};
";

/// The capture rule: the element under the point, walked up past bare
/// wrappers to the nearest one with an id, a role or a class that is neither
/// a sliver nor a shell twice the viewport; with none, the such element
/// nearest across at that height; with none, the body, which follows the top
/// of the document.
const CAPTURE: &str = r"
if (!document.body) return 'null';
const meaningful = (e) => {
  const named = e.getAttribute('id') || e.getAttribute('role')
    || [...e.classList].some((n) => n.trim());
  if (!named) return false;
  const r = e.getBoundingClientRect();
  if (r.width < 8 || r.height < 8) return false;
  return !(r.width > innerWidth * 2 && r.height > innerHeight * 2);
};
const walkUp = (start) => {
  for (let c = start; c && c !== document.body; c = c.parentElement) if (meaningful(c)) return c;
  return null;
};
const nearest = (x, y) => {
  let best = null, least = Infinity;
  for (const e of document.body.querySelectorAll('*')) {
    if (!meaningful(e)) continue;
    const r = e.getBoundingClientRect();
    const top = r.top + scrollY;
    if (y < top || y > top + r.height) continue;
    const away = Math.abs(r.left + scrollX + r.width / 2 - x);
    if (away < least) { least = away; best = e; }
  }
  return best;
};
const X = __X__, Y = __Y__;
let hit = document.elementFromPoint(X - scrollX, Y - scrollY);
while (hit && hit.shadowRoot) {
  const nested = hit.shadowRoot.elementFromPoint(X - scrollX, Y - scrollY);
  if (!nested || nested === hit) break;
  hit = nested;
}
const target = (hit && walkUp(hit)) || nearest(X, Y) || document.body;
const r = target.getBoundingClientRect();
return JSON.stringify({
  selector: unique(target),
  docX: r.left + scrollX,
  docY: r.top + scrollY,
  viewportPositioned: positioned(target),
});
";

/// The tracker. The first poll of a document installs the listeners; every
/// poll then answers with the selectors whose element has moved, appeared or
/// gone since the last answer, at once when there are some and otherwise
/// when a reflow brings some or two seconds pass. Nothing is kept in the
/// devtools session, and a new document reports everything from its first
/// poll.
///
/// A reflow is a resize, a load, or a change under the body, which is
/// looked at 150 ms after the first of a burst. A scroll counts only while
/// a tracked element is fixed or sticky: the others keep their place in the
/// document however the page scrolls.
const TRACK: &str = r"
const T = window.__specularTrack || (window.__specularTrack = (() => {
  const t = { key: '', last: new Map(), wake: null, timer: 0, queued: false, fixed: false, dirty: true };
  const mark = () => { t.dirty = true; if (t.wake) t.wake(); };
  addEventListener('resize', mark);
  addEventListener('load', mark);
  addEventListener('scroll', () => {
    if (!t.fixed || t.queued) return;
    t.queued = true;
    requestAnimationFrame(() => { t.queued = false; mark(); });
  }, true);
  const observe = () => {
    new MutationObserver(() => {
      if (t.timer) return;
      t.timer = setTimeout(() => { t.timer = 0; mark(); }, 150);
    }).observe(document.body, { childList: true, subtree: true, attributes: true });
  };
  if (document.body) observe();
  else addEventListener('DOMContentLoaded', () => { if (document.body) observe(); mark(); });
  return t;
})());
const selectors = __SELECTORS__;
const key = JSON.stringify(selectors);
if (T.key !== key) { T.key = key; T.last = new Map(); T.dirty = true; }
const resolve = (selector) => {
  try {
    const e = document.querySelector(selector);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) return null;
    return { selector, docX: Math.round(r.left + scrollX), docY: Math.round(r.top + scrollY), viewportPositioned: positioned(e) };
  } catch { return null; }
};
const changed = () => {
  T.dirty = false;
  const next = new Map(), out = [];
  let fixed = false;
  for (const selector of selectors) {
    const at = resolve(selector);
    if (!at) {
      if (T.last.has(selector)) out.push({ selector });
      continue;
    }
    fixed = fixed || at.viewportPositioned;
    const stamp = at.docX + ':' + at.docY + ':' + at.viewportPositioned;
    next.set(selector, stamp);
    if (T.last.get(selector) !== stamp) out.push(at);
  }
  T.last = next;
  T.fixed = fixed;
  return out;
};
if (T.dirty) {
  const out = changed();
  if (out.length) return JSON.stringify(out);
}
return new Promise((done) => {
  const finish = (out) => { if (T.wake === wake) T.wake = null; clearTimeout(timer); done(JSON.stringify(out)); };
  const wake = () => { const out = changed(); if (out.length) finish(out); };
  const timer = setTimeout(() => finish([]), 2000);
  T.wake = wake;
});
";

fn evaluate(id: i32, body: &str, await_promise: bool) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": {
            "expression": format!("(() => {{{HELPERS}{POSITIONED}{body}}})()"),
            "returnByValue": true,
            "awaitPromise": await_promise,
        },
    })
    .to_string()
    .into_bytes()
}

/// A finite number as a JavaScript literal; anything else reads as zero.
fn number(value: f32) -> String {
    if value.is_finite() {
        format!("({value})")
    } else {
        "(0)".to_owned()
    }
}

/// The message that asks which element an item centred on the document
/// point (`x`, `y`) should follow.
pub fn capture_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let body = CAPTURE
        .replace("__X__", &number(x))
        .replace("__Y__", &number(y));
    evaluate(id, &body, false)
}

/// The message that asks where the elements `selectors` name have moved to.
/// The answer waits for one to move, up to two seconds.
pub fn track_message(id: i32, selectors: &[String]) -> Vec<u8> {
    let list = serde_json::to_string(selectors).unwrap_or_else(|_| "[]".to_owned());
    evaluate(id, &TRACK.replace("__SELECTORS__", &list), true)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Found {
    selector: String,
    doc_x: Option<f32>,
    doc_y: Option<f32>,
    #[serde(default)]
    viewport_positioned: bool,
}

impl Found {
    fn place(&self) -> Option<ElementPlace> {
        Some(ElementPlace {
            doc: Vec2::new(self.doc_x?, self.doc_y?),
            viewport_positioned: self.viewport_positioned,
        })
    }
}

/// The element a [`capture_message`] found. `None` for a document with no
/// body and for an answer that cannot be read.
pub fn parse_capture(result: &[u8]) -> Option<CapturedElement> {
    let found: Found = serde_json::from_str(&returned(result)?).ok()?;
    Some(CapturedElement {
        place: found.place()?,
        selector: found.selector,
    })
}

/// The places a [`track_message`] found changed: each selector with where
/// its element is now, or `None` when it finds nothing. Empty for an answer
/// that cannot be read.
pub fn parse_places(result: &[u8]) -> Vec<(String, Option<ElementPlace>)> {
    let found: Vec<Found> = returned(result)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    (found.into_iter())
        .map(|found| {
            let place = found.place();
            (found.selector, place)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn value_result(value: &str) -> Vec<u8> {
        json!({ "result": { "type": "string", "value": value } })
            .to_string()
            .into_bytes()
    }

    fn expression(message: &[u8]) -> String {
        let sent: Value = serde_json::from_slice(message).unwrap();
        assert_eq!(sent["method"], "Runtime.evaluate");
        sent["params"]["expression"].as_str().unwrap().to_owned()
    }

    #[test]
    fn a_capture_is_asked_at_a_document_point_and_read_with_its_rail() {
        let ask = expression(&capture_message(3, 120.5, 900.0));
        assert!(ask.contains("const X = (120.5), Y = (900);"));
        assert!(ask.contains("document.elementFromPoint(X - scrollX, Y - scrollY)"));
        assert!(!ask.contains("__"), "a placeholder was left in");

        let answer = r##"{"selector":"#hero","docX":40.5,"docY":612,"viewportPositioned":true}"##;
        assert_eq!(
            parse_capture(&value_result(answer)),
            Some(CapturedElement {
                selector: "#hero".to_owned(),
                place: ElementPlace {
                    doc: Vec2::new(40.5, 612.0),
                    viewport_positioned: true,
                },
            })
        );
        assert_eq!(parse_capture(&value_result("null")), None);
        assert_eq!(parse_capture(&value_result(r##"{"selector":"#a"}"##)), None);
    }

    #[test]
    fn tracking_names_its_selectors_as_data_and_reads_moved_and_gone() {
        let selectors = ["#hero".to_owned(), "a[title=\"x`${1}\"]".to_owned()];
        let message = track_message(4, &selectors);
        let sent: Value = serde_json::from_slice(&message).unwrap();
        assert_eq!(sent["params"]["awaitPromise"], true);
        let ask = expression(&message);
        assert!(ask.contains(r##"const selectors = ["#hero","a[title=\"x`${1}\"]"];"##));
        assert!(!ask.contains("__SELECTORS__"));

        let answer = r##"[{"selector":"#hero","docX":0,"docY":140,"viewportPositioned":false},{"selector":"#gone"}]"##;
        assert_eq!(
            parse_places(&value_result(answer)),
            [
                (
                    "#hero".to_owned(),
                    Some(ElementPlace {
                        doc: Vec2::new(0.0, 140.0),
                        viewport_positioned: false,
                    })
                ),
                ("#gone".to_owned(), None),
            ]
        );
        assert_eq!(parse_places(&value_result("nope")), []);
    }
}
