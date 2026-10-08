//! What interaction sync (ADR 0030) asks of a page through the devtools
//! protocol, and reading its answers. Pure, like
//! [`dom_query`](crate::dom_query), whose helpers it shares.
//!
//! Three questions. The captured page is asked for the hovers and clicks
//! the user gave it. A peer is asked for the elements a captured bundle
//! could mean, which `specular_core::resolve_locator` then scores. And a
//! peer is sent the input to replay, as `Input.dispatchMouseEvent`, which
//! the page cannot tell from the user's.
//!
//! The DOM rules are the Electron preloads' (`interaction-sync-capture.ts`,
//! `interaction-sync-resolver.ts`, `dom-element-utils.ts`).

use serde::Deserialize;
use serde_json::{Value, json};
use specular_core::{LocatorBundle, LocatorCandidate, PointKind};

use crate::dom_query::{HELPERS, returned};

/// How an element is described, for a bundle and for a candidate alike.
const DESCRIBE: &str = r#"
const compact = (v, max) => {
  if (!v) return null;
  const c = String(v).replace(/\s+/g, ' ').trim();
  if (!c) return null;
  return c.length > max ? c.slice(0, max - 1) + '…' : c;
};
const bestName = (e) => {
  const text = compact(e.innerText, 80) ?? compact(e.getAttribute('aria-label'), 120)
    ?? compact(e.getAttribute('title'), 120) ?? compact(e.value, 120)
    ?? compact(e.getAttribute('placeholder'), 120) ?? compact(e.getAttribute('alt'), 120);
  const tag = e.tagName.toLowerCase();
  return text ? tag + ' "' + text + '"' : tag;
};
const describe = (e) => ({
  id: e.getAttribute('id') || null,
  testId: e.getAttribute('data-testid') || null,
  role: e.getAttribute('role') || null,
  name: bestName(e),
  text: compact(e.textContent, 80),
  tag: e.tagName.toLowerCase(),
  elementPath: path(e, 4),
  fullPath: path(e, 10),
});
"#;

/// The capture. The first poll of a document installs the listeners; every
/// poll then answers with what was captured since the last, at once when
/// there is something and otherwise when something arrives or two seconds
/// pass. Nothing is kept in the devtools session, so a client enabling or
/// disabling a domain on the same channel cannot break it, and a new
/// document is captured from its first poll.
///
/// A hover is reported when the element changes or the pointer moves more
/// than 1.5% of the element, one a frame at most. What was captured more
/// than three seconds before a poll is from before the page was captured,
/// and is dropped.
const CAPTURE: &str = r"
const S = window.__specularSync || (window.__specularSync = (() => {
  const s = { queue: [], wake: null, pending: null, queued: false, sent: false, el: null, ox: 0, oy: 0, bundle: null };
  const clamp = (v) => Math.max(0, Math.min(1, v));
  const target = (ev) => {
    const p = ev.composedPath();
    const first = p.length ? p[0] : ev.target;
    return first instanceof Element ? first : null;
  };
  const offset = (e, x, y) => {
    const r = e.getBoundingClientRect();
    return [r.width > 0 ? clamp((x - r.left) / r.width) : 0.5, r.height > 0 ? clamp((y - r.top) / r.height) : 0.5];
  };
  const push = (kind, bundle) => {
    s.queue.push({ kind, bundle, at: Date.now() });
    if (s.queue.length > 32) s.queue.shift();
    if (s.wake) s.wake();
  };
  const flush = () => {
    s.queued = false;
    const ev = s.pending;
    if (!ev) return;
    const e = target(ev);
    if (!e) { s.el = null; s.bundle = null; return; }
    const [ox, oy] = offset(e, ev.clientX, ev.clientY);
    const changed = e !== s.el;
    if (s.sent && !changed && Math.abs(ox - s.ox) <= 0.015 && Math.abs(oy - s.oy) <= 0.015) return;
    s.bundle = !changed && s.bundle ? { ...s.bundle, offsetX: ox, offsetY: oy } : { ...describe(e), offsetX: ox, offsetY: oy };
    s.sent = true; s.el = e; s.ox = ox; s.oy = oy;
    push('hover', s.bundle);
  };
  addEventListener('mousemove', (ev) => {
    if (!ev.isTrusted) return;
    s.pending = ev;
    if (s.queued) return;
    s.queued = true;
    requestAnimationFrame(flush);
  }, true);
  addEventListener('click', (ev) => {
    if (!ev.isTrusted || ev.button !== 0) return;
    const e = target(ev);
    if (!e) return;
    const [ox, oy] = offset(e, ev.clientX, ev.clientY);
    const bundle = { ...describe(e), offsetX: ox, offsetY: oy };
    push('click', bundle);
    s.pending = null; s.sent = true; s.el = e; s.ox = ox; s.oy = oy; s.bundle = bundle;
  }, true);
  return s;
})());
const drain = () => {
  const fresh = Date.now() - 3000;
  return JSON.stringify(S.queue.splice(0).filter((e) => e.at >= fresh));
};
if (S.queue.length) return drain();
return new Promise((resolve) => {
  const wake = () => { if (S.wake === wake) S.wake = null; clearTimeout(timer); resolve(drain()); };
  const timer = setTimeout(wake, 2000);
  S.wake = wake;
});
";

/// A peer's candidates: the one visible element with the bundle's `id` or
/// `data-testid` when there is exactly one, and otherwise every visible
/// element twelve levels deep, open shadow roots included.
const CANDIDATES: &str = r"
const visible = (e) => {
  const r = e.getBoundingClientRect();
  if (r.width === 0 && r.height === 0) return false;
  const s = getComputedStyle(e);
  return s.display !== 'none' && s.visibility !== 'hidden';
};
const interactive = (e) => {
  const tag = e.tagName.toLowerCase();
  if (['a', 'button', 'input', 'select', 'textarea', 'summary', 'option', 'label'].includes(tag)) return true;
  const role = e.getAttribute('role');
  if (role && ['button', 'link', 'checkbox', 'textbox', 'menuitem', 'option', 'tab', 'switch'].includes(role)) return true;
  if (e.hasAttribute('onclick') || e.tabIndex >= 0) return true;
  return getComputedStyle(e).cursor === 'pointer';
};
const candidate = (e) => {
  const r = e.getBoundingClientRect();
  return { ...describe(e), interactive: interactive(e), rect: { x: r.left, y: r.top, width: r.width, height: r.height } };
};
const depth = (e) => {
  let d = 0, c = e;
  while (c && c !== document.body) { d += 1; c = c.parentElement; }
  return c === document.body ? d : Infinity;
};
const only = (attribute, value) => {
  let one = null;
  for (const e of document.querySelectorAll('[' + attribute + '=' + JSON.stringify(value) + ']')) {
    if (!visible(e) || depth(e) > 12) continue;
    if (one) return null;
    one = e;
  }
  return one ? [candidate(one)] : null;
};
const id = __ID__, testId = __TEST_ID__;
const quick = id ? only('id', id) : testId ? only('data-testid', testId) : null;
if (quick) return JSON.stringify(quick);
const out = [];
const walk = (e, d) => {
  if (d > 12 || !visible(e)) return;
  out.push(candidate(e));
  for (const c of e.children) walk(c, d + 1);
  if (e.shadowRoot) for (const c of e.shadowRoot.children) walk(c, d + 1);
};
if (document.body) walk(document.body, 0);
return JSON.stringify(out);
";

fn evaluate(id: i32, body: &str, await_promise: bool) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": {
            "expression": format!("(() => {{{HELPERS}{DESCRIBE}{body}}})()"),
            "returnByValue": true,
            "awaitPromise": await_promise,
        },
    })
    .to_string()
    .into_bytes()
}

/// The message that asks the captured page what it was pointed at. The
/// answer waits for something to tell, up to two seconds.
pub fn capture_message(id: i32) -> Vec<u8> {
    evaluate(id, CAPTURE, true)
}

/// A string as a JavaScript literal, or `null`.
fn literal(value: Option<&str>) -> String {
    (value.filter(|value| !value.is_empty()))
        .and_then(|value| serde_json::to_string(value).ok())
        .unwrap_or_else(|| "null".to_owned())
}

/// The message that asks a page for the elements `bundle` could mean.
pub fn candidates_message(id: i32, bundle: &LocatorBundle) -> Vec<u8> {
    let body = CANDIDATES
        .replace("__ID__", &literal(bundle.id.as_deref()))
        .replace("__TEST_ID__", &literal(bundle.test_id.as_deref()));
    evaluate(id, &body, false)
}

/// The messages that replay a hover or a click at a viewport point: one
/// move, or a press and a release of the primary button.
pub fn replay_messages(kind: PointKind, x: f32, y: f32) -> Vec<Value> {
    let mouse = |event: &str, button: &str, clicks: u8| {
        json!({
            "method": "Input.dispatchMouseEvent",
            "params": { "type": event, "x": x, "y": y, "button": button, "clickCount": clicks },
        })
    };
    match kind {
        PointKind::Hover => vec![mouse("mouseMoved", "none", 0)],
        PointKind::Click => vec![
            mouse("mousePressed", "left", 1),
            mouse("mouseReleased", "left", 1),
        ],
    }
}

#[derive(Deserialize)]
struct Captured {
    kind: String,
    bundle: LocatorBundle,
}

/// What a [`capture_message`] answered, oldest first. Empty for an answer
/// that cannot be read.
pub fn parse_captured(result: &[u8]) -> Vec<(PointKind, LocatorBundle)> {
    let captured: Vec<Captured> = returned(result)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    (captured.into_iter())
        .filter_map(|captured| {
            let kind = match captured.kind.as_str() {
                "hover" => PointKind::Hover,
                "click" => PointKind::Click,
                _ => return None,
            };
            Some((kind, captured.bundle))
        })
        .collect()
}

/// The elements a [`candidates_message`] found. Empty for an answer that
/// cannot be read, which then matches nothing.
pub fn parse_candidates(result: &[u8]) -> Vec<LocatorCandidate> {
    returned(result)
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_result(value: &str) -> Vec<u8> {
        json!({ "result": { "type": "string", "value": value } })
            .to_string()
            .into_bytes()
    }

    #[test]
    fn a_captured_click_is_read_and_an_unknown_kind_is_dropped() {
        let answer = r#"[
            {"kind":"click","at":1,"bundle":{"id":"go","tag":"button","elementPath":"main > button#go","fullPath":"body > main > button#go","offsetX":0.25,"offsetY":0.5}},
            {"kind":"drag","at":2,"bundle":{"tag":"div"}}
        ]"#;
        let captured = parse_captured(&value_result(answer));
        assert_eq!(captured.len(), 1);
        let (kind, bundle) = &captured[0];
        assert_eq!(*kind, PointKind::Click);
        assert_eq!(bundle.id.as_deref(), Some("go"));
        assert!((bundle.offset_x - 0.25).abs() < f64::EPSILON);
        assert_eq!(parse_captured(b"not json").len(), 0);
    }

    #[test]
    fn a_candidates_question_quotes_the_identity_keys_and_its_answer_is_read() {
        let bundle = LocatorBundle {
            id: Some("a\"]); alert(1); (\"".to_owned()),
            ..LocatorBundle::default()
        };
        let message: Value = serde_json::from_slice(&candidates_message(9, &bundle)).unwrap();
        let expression = message["params"]["expression"].as_str().unwrap();
        assert!(expression.contains(r#"const id = "a\"]); alert(1); (\"", testId = null;"#));
        assert!(!expression.contains("__"), "a placeholder was left in");

        let answer = r#"[{"id":"go","testId":null,"role":null,"name":"button \"Go\"","text":"Go","tag":"button","elementPath":"button#go","fullPath":"body > button#go","interactive":true,"rect":{"x":10,"y":20,"width":40,"height":20}}]"#;
        let candidates = parse_candidates(&value_result(answer));
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].interactive);
        assert!((candidates[0].rect.width - 40.0).abs() < f64::EPSILON);
        assert_eq!(parse_candidates(b"not json").len(), 0);
    }

    #[test]
    fn a_click_is_replayed_as_a_press_and_a_release_and_a_hover_as_a_move() {
        let message = |event: &str, button: &str, clicks: u8| {
            json!({
                "method": "Input.dispatchMouseEvent",
                "params": { "type": event, "x": 30.0, "y": 40.0, "button": button, "clickCount": clicks },
            })
        };
        assert_eq!(
            replay_messages(PointKind::Hover, 30.0, 40.0),
            [message("mouseMoved", "none", 0)]
        );
        assert_eq!(
            replay_messages(PointKind::Click, 30.0, 40.0),
            [
                message("mousePressed", "left", 1),
                message("mouseReleased", "left", 1)
            ]
        );
    }
}
