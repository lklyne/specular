//! The questions a page is asked through the devtools protocol, and reading
//! its answers. Pure: the messages are bytes out and bytes in, so all of it
//! is tested with no browser.
//!
//! Each question is one `Runtime.evaluate` of an expression that returns a
//! JSON string, so the page does the DOM work in one round trip and the
//! answer does not depend on devtools object ids. The DOM rules are the
//! Electron preload's (`git show bc5253fa:src/preload/dom-element-utils.ts`): the unique
//! selector is an `nth-of-type` chain up to the nearest unique id, the path
//! is four readable segments, and a region grabs the visible interactive
//! elements it wholly contains, and answers with the box of every visible
//! element it wholly contains, for the outlines drawn while it is dragged.

mod device;
mod scroll;

#[cfg(feature = "cef")]
pub(crate) use self::device::{
    color_scheme_message, device_methods, method_message, parse_viewport_fit, target_info_message,
    viewport_fit_message,
};
#[cfg(feature = "cef")]
pub(crate) use self::scroll::{
    parse_scroll_progress, parse_scroll_room, scroll_progress_message, scroll_room_message,
    scroll_to_message,
};

use serde_json::{Value, json};
use specular_core::{CssRect, PageElement, PixelRect};

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
pub(crate) fn element_at_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let values = [("__X__", number(x)), ("__Y__", number(y))];
    evaluate(id, &expression(ELEMENT_AT, &values))
}

/// The message that asks how many elements a viewport rect grabs, and for
/// the boxes of the visible elements wholly inside it.
pub(crate) fn elements_in_rect_message(id: i32, rect: CssRect) -> Vec<u8> {
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
pub(crate) fn parse_element(result: &[u8]) -> Option<PageElement> {
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
pub(crate) fn parse_elements_in_rect(result: &[u8]) -> (usize, Vec<CssRect>) {
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
pub(crate) fn parse_target_id(result: &[u8]) -> Option<String> {
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
