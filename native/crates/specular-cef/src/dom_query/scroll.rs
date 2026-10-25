//! Asking a page how far it scrolls and has room to, and scrolling it.

use glam::Vec2;
use serde_json::Value;

use super::{evaluate, expression, number, returned};

/// How far the document can scroll along each axis, as Electron's scroll
/// sync measured it (`git show bc5253fa:src/shared/scroll-sync.ts`).
const SCROLL_EXTENT: &str = r"
const root = document.documentElement, body = document.body;
const maxX = Math.max(0, (root ? root.scrollWidth : 0) - innerWidth, (body ? body.scrollWidth : 0) - innerWidth);
const maxY = Math.max(0, (root ? root.scrollHeight : 0) - innerHeight, (body ? body.scrollHeight : 0) - innerHeight);
";

const SCROLL_PROGRESS: &str = r"
return JSON.stringify({ x: maxX > 0 ? scrollX / maxX : 0, y: maxY > 0 ? scrollY / maxY : 0 });
";

/// Whether a finger at a point, moving by `(__DX__, __DY__)`, has anything
/// to scroll: a scroller above the element there with room left that way,
/// or the document itself. What is inside a frame cannot be seen from
/// here, so a frame is taken to have room.
const SCROLL_ROOM: &str = r"
const vertical = Math.abs(__DY__) >= Math.abs(__DX__);
// A finger that moves down scrolls back towards the start.
const back = (vertical ? __DY__ : __DX__) > 0;
const room = (el) => {
  const at = vertical ? el.scrollTop : el.scrollLeft;
  const max = vertical ? el.scrollHeight - el.clientHeight : el.scrollWidth - el.clientWidth;
  return back ? at > 0.5 : at < max - 0.5;
};
const root = document.scrollingElement || document.documentElement;
let el = document.elementFromPoint(__X__, __Y__);
for (; el; el = el.parentElement || el.getRootNode().host) {
  if (el.tagName === 'IFRAME') return 'true';
  if (el === root || el === document.body || el === document.documentElement) continue;
  const style = getComputedStyle(el);
  const flow = vertical ? style.overflowY : style.overflowX;
  if ((flow === 'auto' || flow === 'scroll') && room(el)) return 'true';
}
return String(room(root));
";

/// `instant` so a page styled with `scroll-behavior: smooth` lands where it
/// was sent and not somewhere on the way.
const SCROLL_TO: &str = r"
scrollTo({ left: maxX * __X__, top: maxY * __Y__, behavior: 'instant' });
return 'ok';
";

/// The message that asks how far through its document a page is scrolled.
pub(crate) fn scroll_progress_message(id: i32) -> Vec<u8> {
    evaluate(
        id,
        &format!("(() => {{{SCROLL_EXTENT}{SCROLL_PROGRESS}}})()"),
    )
}

/// The message that asks whether anything under `at` can scroll with a
/// finger that moves `toward`.
pub(crate) fn scroll_room_message(id: i32, at: Vec2, toward: Vec2) -> Vec<u8> {
    let values = [
        ("__X__", number(at.x)),
        ("__Y__", number(at.y)),
        ("__DX__", number(toward.x)),
        ("__DY__", number(toward.y)),
    ];
    evaluate(id, &expression(SCROLL_ROOM, &values))
}

/// What a [`scroll_room_message`] found. `None` for an answer that cannot
/// be read.
pub(crate) fn parse_scroll_room(result: &[u8]) -> Option<bool> {
    returned(result)?.parse().ok()
}

/// The message that scrolls a page to a fraction of how far it can scroll.
pub(crate) fn scroll_to_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let body = SCROLL_TO
        .replace("__X__", &number(x.clamp(0.0, 1.0)))
        .replace("__Y__", &number(y.clamp(0.0, 1.0)));
    evaluate(id, &format!("(() => {{{SCROLL_EXTENT}{body}}})()"))
}

/// The fractions a [`scroll_progress_message`] found. `None` for an answer
/// that cannot be read.
pub(crate) fn parse_scroll_progress(result: &[u8]) -> Option<(f32, f32)> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let along = |key: &str| Some((found.get(key)?.as_f64()? as f32).clamp(0.0, 1.0));
    Some((along("x")?, along("y")?))
}
