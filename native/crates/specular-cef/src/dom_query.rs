//! The questions a page is asked through the devtools protocol, and reading
//! its answers. Pure: the messages are bytes out and bytes in, so all of it
//! is tested with no browser.
//!
//! Each question is one `Runtime.evaluate` of an expression that returns a
//! JSON string, so the page does the DOM work in one round trip and the
//! answer does not depend on devtools object ids. The DOM rules are the
//! Electron preload's (`src/preload/dom-element-utils.ts`): the unique
//! selector is an `nth-of-type` chain up to the nearest unique id, the path
//! is four readable segments, and a region grabs the visible interactive
//! elements it wholly contains, and answers with the box of every visible
//! element it wholly contains, for the outlines drawn while it is dragged.

use glam::Vec2;
use serde_json::{Value, json};
use specular_core::{CssRect, PageColorScheme, PageDevice, PageElement, PixelRect};

/// The most elements a region counts, as Electron's region select does.
const MAX_GRABBED: u32 = 15;

/// The most boxes a region answers with, as Electron's region preview
/// outlines.
const MAX_BOXES: u32 = 200;

/// Helpers both expressions share: the readable path and the selector that
/// finds the element again.
pub(crate) const HELPERS: &str = r#"
const segment = (e) => {
  const tag = e.tagName.toLowerCase();
  const id = e.getAttribute('id');
  if (id) return tag + '#' + id;
  const role = e.getAttribute('role');
  if (role) return tag + '[role="' + role + '"]';
  const classes = [...e.classList].map((n) => n.trim()).filter(Boolean).slice(0, 2);
  return classes.length ? tag + '.' + classes.join('.') : tag;
};
const path = (e, max) => {
  const out = [];
  for (let c = e, d = 0; c && d < max; d += 1) {
    out.unshift(segment(c));
    const root = c.getRootNode();
    if (root instanceof ShadowRoot && root.host instanceof Element) {
      out.unshift('#shadow-root');
      c = root.host;
    } else {
      c = c.parentElement;
    }
  }
  return out.join(' > ');
};
const unique = (e) => {
  const out = [];
  let c = e;
  while (c && c.parentElement) {
    const tag = c.tagName.toLowerCase();
    const id = c.getAttribute('id');
    if (id && c.ownerDocument.querySelectorAll('#' + CSS.escape(id)).length === 1) {
      out.unshift('#' + CSS.escape(id));
      return out.join(' > ');
    }
    const twins = [...c.parentElement.children].filter((s) => s.tagName === c.tagName);
    out.unshift(twins.length > 1 ? tag + ':nth-of-type(' + (twins.indexOf(c) + 1) + ')' : tag);
    c = c.parentElement;
  }
  if (c) out.unshift(c.tagName.toLowerCase());
  return out.join(' > ');
};
"#;

const ELEMENT_AT: &str = r"
let el = document.elementFromPoint(__X__, __Y__);
if (!el) return 'null';
while (el.shadowRoot) {
  const nested = el.shadowRoot.elementFromPoint(__X__, __Y__);
  if (!nested || nested === el) break;
  el = nested;
}
const r = el.getBoundingClientRect();
return JSON.stringify({
  selector: unique(el),
  elementPath: path(el, 4),
  x: Math.round(r.left),
  y: Math.round(r.top),
  width: Math.round(r.width),
  height: Math.round(r.height),
});
";

const ELEMENTS_IN_RECT: &str = r"
const region = { left: __X__, top: __Y__, right: __X__ + __W__, bottom: __Y__ + __H__ };
const interactive = (e) => {
  const tag = e.tagName.toLowerCase();
  if (['a', 'button', 'input', 'select', 'textarea', 'summary', 'option', 'label'].includes(tag)) return true;
  const role = e.getAttribute('role');
  if (role && ['button', 'link', 'checkbox', 'textbox', 'menuitem', 'option', 'tab', 'switch'].includes(role)) return true;
  if (e.hasAttribute('onclick') || e.tabIndex >= 0) return true;
  return getComputedStyle(e).cursor === 'pointer';
};
if (!document.body) return JSON.stringify({ count: 0, boxes: [] });
let count = 0;
const boxes = [];
const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
  acceptNode(e) {
    const box = e.getBoundingClientRect();
    if (box.width === 0 && box.height === 0) return NodeFilter.FILTER_SKIP;
    const style = getComputedStyle(e);
    if (style.display === 'none' || style.visibility === 'hidden') return NodeFilter.FILTER_SKIP;
    const inside = box.left >= region.left && box.top >= region.top
      && box.right <= region.right && box.bottom <= region.bottom;
    return inside ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP;
  },
});
while ((count < __MAX__ || boxes.length < __BOXES__) && walker.nextNode()) {
  const e = walker.currentNode;
  if (boxes.length < __BOXES__) {
    const box = e.getBoundingClientRect();
    boxes.push([box.left, box.top, box.width, box.height]);
  }
  if (count < __MAX__ && interactive(e)) count += 1;
}
return JSON.stringify({ count, boxes });
";

/// How far the document can scroll along each axis, as
/// `src/shared/scroll-sync.ts` measures it.
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

/// What the document says of the screen around it: whether its viewport
/// meta asks to cover the screen, and the colour a browser paints round it,
/// its `theme-color` or else its background. Answered once the document is
/// parsed, so a question put while it loads waits for that.
const VIEWPORT_FIT: &str = r#"
if (document.readyState === 'loading') {
  await new Promise((done) => document.addEventListener('DOMContentLoaded', done, { once: true }));
}
const metas = (name) => [...document.querySelectorAll('meta[name="' + name + '"]')];
const fit = metas('viewport').map((meta) => meta.content || '').join(',');
const rgb = (css) => {
  if (!css || !CSS.supports('color', css)) return null;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return null;
  context.fillStyle = css;
  context.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = context.getImageData(0, 0, 1, 1).data;
  if (a === 0) return null;
  // What shows through is the browser's white.
  return [r, g, b].map((c) => Math.round((c * a + 255 * (255 - a)) / 255));
};
const theme = metas('theme-color').find((meta) => !meta.media || matchMedia(meta.media).matches);
const background = (e) => (e ? rgb(getComputedStyle(e).backgroundColor) : null);
const color = (theme && rgb(theme.content))
  || background(document.body) || background(document.documentElement);
return JSON.stringify({ covers: /viewport-fit\s*=\s*cover/i.test(fit), color });
"#;

/// A finite number as a JavaScript literal. Anything else reads as zero, so
/// a bad coordinate cannot break out of the expression.
pub(crate) fn number(value: f32) -> String {
    if value.is_finite() {
        format!("({value})")
    } else {
        "(0)".to_owned()
    }
}

pub(crate) fn expression(body: &str, values: &[(&str, String)]) -> String {
    let mut body = body.to_owned();
    for (name, value) in values {
        body = body.replace(name, value);
    }
    format!("(() => {{{HELPERS}{body}}})()")
}

/// A `Runtime.evaluate` message carrying `expression`, answered by value.
pub(crate) fn evaluate(id: i32, expression: &str) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": { "expression": expression, "returnByValue": true },
    })
    .to_string()
    .into_bytes()
}

/// The message that asks for the element under a viewport point.
pub fn element_at_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let values = [("__X__", number(x)), ("__Y__", number(y))];
    evaluate(id, &expression(ELEMENT_AT, &values))
}

/// The message that asks how many elements a viewport rect grabs, and for
/// the boxes of the visible elements wholly inside it.
pub fn elements_in_rect_message(id: i32, rect: CssRect) -> Vec<u8> {
    let values = [
        ("__MAX__", MAX_GRABBED.to_string()),
        ("__BOXES__", MAX_BOXES.to_string()),
        ("__X__", number(rect.x)),
        ("__Y__", number(rect.y)),
        ("__W__", number(rect.width)),
        ("__H__", number(rect.height)),
    ];
    evaluate(id, &expression(ELEMENTS_IN_RECT, &values))
}

/// The message that asks how far through its document a page is scrolled.
pub fn scroll_progress_message(id: i32) -> Vec<u8> {
    evaluate(
        id,
        &format!("(() => {{{SCROLL_EXTENT}{SCROLL_PROGRESS}}})()"),
    )
}

/// The message that asks whether anything under `at` can scroll with a
/// finger that moves `toward`.
pub fn scroll_room_message(id: i32, at: Vec2, toward: Vec2) -> Vec<u8> {
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
pub fn parse_scroll_room(result: &[u8]) -> Option<bool> {
    returned(result)?.parse().ok()
}

/// The message that scrolls a page to a fraction of how far it can scroll.
pub fn scroll_to_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let body = SCROLL_TO
        .replace("__X__", &number(x.clamp(0.0, 1.0)))
        .replace("__Y__", &number(y.clamp(0.0, 1.0)));
    evaluate(id, &format!("(() => {{{SCROLL_EXTENT}{body}}})()"))
}

/// The fractions a [`scroll_progress_message`] found. `None` for an answer
/// that cannot be read.
pub fn parse_scroll_progress(result: &[u8]) -> Option<(f32, f32)> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let along = |key: &str| Some((found.get(key)?.as_f64()? as f32).clamp(0.0, 1.0));
    Some((along("x")?, along("y")?))
}

/// The message that asks a document how it fits its screen. The answer
/// waits for the document to be parsed.
pub fn viewport_fit_message(id: i32) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Runtime.evaluate",
        "params": {
            "expression": format!("(async () => {{{VIEWPORT_FIT}}})()"),
            "returnByValue": true,
            "awaitPromise": true,
        },
    })
    .to_string()
    .into_bytes()
}

/// Whether the document covers its screen and the colour round it, as a
/// [`viewport_fit_message`] found them. `None` for an answer that cannot be
/// read, as that of a document that went away while it was asked.
pub fn parse_viewport_fit(result: &[u8]) -> Option<(bool, Option<[u8; 3]>)> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let covers = found.get("covers")?.as_bool()?;
    let color = (found.get("color").cloned())
        .and_then(|color| serde_json::from_value::<[u8; 3]>(color).ok());
    Some((covers, color))
}

/// The message that makes a page report `scheme` as its
/// `prefers-color-scheme`: `applyPageColorScheme`'s
/// `Emulation.setEmulatedMedia`, over the page's devtools channel.
pub fn color_scheme_message(id: i32, scheme: PageColorScheme) -> Vec<u8> {
    let value = match scheme {
        PageColorScheme::Light => "light",
        PageColorScheme::Dark => "dark",
    };
    json!({
        "id": id,
        "method": "Emulation.setEmulatedMedia",
        "params": { "features": [{ "name": "prefers-color-scheme", "value": value }] },
    })
    .to_string()
    .into_bytes()
}

/// The devtools methods that make a page tell its document it is on
/// `device`, each with its params: the ones Chrome's device mode sends.
/// Every one is sent whatever the device, so a page that was a phone stops
/// being one.
pub fn device_methods(device: &PageDevice) -> [(&'static str, Value); 4] {
    let user_agent = device.user_agent.map_or_else(
        // An empty user agent is the browser's own.
        || json!({ "userAgent": "" }),
        |agent| {
            json!({
                "userAgent": agent.user_agent,
                "platform": agent.navigator_platform,
                "userAgentMetadata": {
                    "platform": agent.hint_platform,
                    "platformVersion": "",
                    "architecture": "",
                    "model": agent.model,
                    "mobile": true,
                },
            })
        },
    );
    // No width or height: the page keeps the viewport it is laid out at.
    // Chromium rasters at the screen's scale whatever ratio is reported.
    let metrics = json!({
        "width": 0,
        "height": 0,
        "deviceScaleFactor": device.pixel_ratio.unwrap_or(0.0),
        "mobile": device.mobile,
    });
    let touch = json!({ "enabled": device.touch, "maxTouchPoints": 5 });
    // The insets `env(safe-area-inset-*)` resolve to. The `Max` ones are
    // what `env(safe-area-max-inset-*)` do, the same on a screen whose safe
    // area does not move.
    let area = device.safe_area;
    let safe_area = json!({ "insets": {
        "top": area.top, "topMax": area.top,
        "right": area.right, "rightMax": area.right,
        "bottom": area.bottom, "bottomMax": area.bottom,
        "left": area.left, "leftMax": area.left,
    }});
    [
        ("Emulation.setUserAgentOverride", user_agent),
        ("Emulation.setDeviceMetricsOverride", metrics),
        ("Emulation.setTouchEmulationEnabled", touch),
        ("Emulation.setSafeAreaInsetsOverride", safe_area),
    ]
}

/// The message that calls the devtools `method` with `params`.
pub fn method_message(id: i32, method: &str, params: &Value) -> Vec<u8> {
    json!({ "id": id, "method": method, "params": params })
        .to_string()
        .into_bytes()
}

/// The message that asks a page for its own devtools target.
pub fn target_info_message(id: i32) -> Vec<u8> {
    json!({ "id": id, "method": "Target.getTargetInfo" })
        .to_string()
        .into_bytes()
}

/// The string an expression returned, from a `Runtime.evaluate` result.
/// `None` when the page threw or returned something else.
pub(crate) fn returned(result: &[u8]) -> Option<String> {
    let result: Value = serde_json::from_slice(result).ok()?;
    if result.get("exceptionDetails").is_some() {
        return None;
    }
    Some(result.get("result")?.get("value")?.as_str()?.to_owned())
}

/// The element an [`element_at_message`] found. `None` for a point on no
/// element and for an answer that cannot be read.
pub fn parse_element(result: &[u8]) -> Option<PageElement> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let int = |key: &str| found.get(key)?.as_i64();
    let side = |key: &str| Some(u32::try_from(int(key)?.max(0)).unwrap_or(u32::MAX));
    Some(PageElement {
        selector: found.get("selector")?.as_str()?.to_owned(),
        element_path: (found.get("elementPath").and_then(Value::as_str))
            .filter(|path| !path.is_empty())
            .map(str::to_owned),
        bounding_box: PixelRect::new(
            i32::try_from(int("x")?).ok()?,
            i32::try_from(int("y")?).ok()?,
            side("width")?,
            side("height")?,
        ),
    })
}

/// The count and the boxes an [`elements_in_rect_message`] found; none for
/// an answer that cannot be read.
pub fn parse_elements_in_rect(result: &[u8]) -> (usize, Vec<CssRect>) {
    let found: Option<Value> = returned(result).and_then(|text| serde_json::from_str(&text).ok());
    let Some(found) = found else {
        return (0, Vec::new());
    };
    let count = found.get("count").and_then(Value::as_u64).unwrap_or(0);
    let boxes: Vec<[f32; 4]> = (found.get("boxes").cloned())
        .and_then(|boxes| serde_json::from_value(boxes).ok())
        .unwrap_or_default();
    let boxes = (boxes.into_iter())
        .map(|[x, y, width, height]| CssRect::new(x, y, width, height))
        .collect();
    (usize::try_from(count).unwrap_or(usize::MAX), boxes)
}

/// The target id in a `Target.getTargetInfo` result.
pub fn parse_target_id(result: &[u8]) -> Option<String> {
    let result: Value = serde_json::from_slice(result).ok()?;
    let id = result.get("targetInfo")?.get("targetId")?.as_str()?;
    (!id.is_empty()).then(|| id.to_owned())
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
    fn an_element_answer_becomes_a_page_element() {
        let answer = r##"{"selector":"#top","elementPath":"body > section#one > h1#top","x":40,"y":61,"width":720,"height":64}"##;
        assert_eq!(
            parse_element(&value_result(answer)),
            Some(PageElement {
                selector: "#top".to_owned(),
                element_path: Some("body > section#one > h1#top".to_owned()),
                bounding_box: PixelRect::new(40, 61, 720, 64),
            })
        );
    }
}
