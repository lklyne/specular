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
//! elements it wholly contains.

use serde_json::{Value, json};
use specular_core::{CssRect, PageColorScheme, PageElement, PixelRect};

/// The most elements a region counts, as Electron's region select does.
const MAX_GRABBED: u32 = 15;

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
if (!document.body) return '0';
const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_ELEMENT, {
  acceptNode(e) {
    const box = e.getBoundingClientRect();
    if (box.width === 0 && box.height === 0) return NodeFilter.FILTER_SKIP;
    const style = getComputedStyle(e);
    if (style.display === 'none' || style.visibility === 'hidden') return NodeFilter.FILTER_SKIP;
    const inside = box.left >= region.left && box.top >= region.top
      && box.right <= region.right && box.bottom <= region.bottom;
    return inside && interactive(e) ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_SKIP;
  },
});
let count = 0;
while (count < __MAX__ && walker.nextNode()) count += 1;
return String(count);
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

/// `instant` so a page styled with `scroll-behavior: smooth` lands where it
/// was sent and not somewhere on the way.
const SCROLL_TO: &str = r"
scrollTo({ left: maxX * __X__, top: maxY * __Y__, behavior: 'instant' });
return 'ok';
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
pub fn element_at_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let values = [("__X__", number(x)), ("__Y__", number(y))];
    evaluate(id, &expression(ELEMENT_AT, &values))
}

/// The message that asks how many elements a viewport rect grabs.
pub fn elements_in_rect_message(id: i32, rect: CssRect) -> Vec<u8> {
    let values = [
        ("__MAX__", MAX_GRABBED.to_string()),
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

/// The count an [`elements_in_rect_message`] found; zero for an answer that
/// cannot be read.
pub fn parse_count(result: &[u8]) -> usize {
    returned(result)
        .and_then(|count| count.parse().ok())
        .unwrap_or(0)
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

    fn sent(message: &[u8]) -> Value {
        serde_json::from_slice(message).unwrap()
    }

    #[test]
    fn an_element_question_evaluates_one_expression_at_the_point() {
        let message = sent(&element_at_message(7, 12.5, 40.0));
        assert_eq!(message["id"], 7);
        assert_eq!(message["method"], "Runtime.evaluate");
        assert_eq!(message["params"]["returnByValue"], true);
        let expression = message["params"]["expression"].as_str().unwrap();
        assert!(expression.contains("document.elementFromPoint((12.5), (40))"));
        assert!(!expression.contains("__"), "a placeholder was left in");
    }

    #[test]
    fn a_region_question_carries_the_rect_and_the_cap() {
        let rect = CssRect::new(10.0, 20.0, 300.0, 150.5);
        let message = sent(&elements_in_rect_message(3, rect));
        let expression = message["params"]["expression"].as_str().unwrap();
        assert!(
            expression
                .contains("{ left: (10), top: (20), right: (10) + (300), bottom: (20) + (150.5) }")
        );
        assert!(expression.contains("while (count < 15 &&"));
        assert!(expression.contains("NodeFilter.SHOW_ELEMENT"));
        assert!(!expression.contains("__"), "a placeholder was left in");
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

    #[test]
    fn no_element_a_thrown_error_and_noise_are_all_no_element() {
        assert_eq!(parse_element(&value_result("null")), None);
        let thrown = br#"{"result":{"type":"object"},"exceptionDetails":{"text":"Uncaught"}}"#;
        assert_eq!(parse_element(thrown), None);
        assert_eq!(parse_element(b"not json"), None);
        assert_eq!(parse_element(&value_result(r#"{"selector":"a"}"#)), None);
    }

    #[test]
    fn scrolling_is_asked_and_answered_as_a_fraction_of_the_scrollable_extent() {
        let ask = sent(&scroll_progress_message(4));
        let ask = ask["params"]["expression"].as_str().unwrap();
        assert!(ask.contains("scrollY / maxY"));
        let answer = value_result(r#"{"x":0,"y":0.25}"#);
        assert_eq!(parse_scroll_progress(&answer), Some((0.0, 0.25)));
        assert_eq!(parse_scroll_progress(&value_result("nope")), None);

        let go = sent(&scroll_to_message(5, 0.0, 1.5));
        let go = go["params"]["expression"].as_str().unwrap();
        assert!(go.contains("left: maxX * (0), top: maxY * (1)"));
        assert!(!go.contains("__"), "a placeholder was left in");
    }

    #[test]
    fn a_target_id_is_read_from_target_info() {
        let result = br#"{"targetInfo":{"targetId":"D84418FF","type":"page","url":"https://a/"}}"#;
        assert_eq!(parse_target_id(result), Some("D84418FF".to_owned()));
        assert_eq!(parse_target_id(br#"{"targetInfo":{"targetId":""}}"#), None);
        assert_eq!(parse_target_id(b"{}"), None);
    }
}
