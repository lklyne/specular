//! The inspect tool's question: the DOM node under a point, with the facts
//! its outline, popover and chat pill show. Pure like
//! [`dom_query`](crate::dom_query): one `Runtime.evaluate` out, bytes in.
//!
//! The DOM rules are `inspectionPayload` in the Electron preload
//! (`src/preload/dom-element-utils.ts`), minus the overlay skip: a CEF page
//! has no Specular overlay injected into it.

use serde_json::Value;
use specular_core::{InspectedNode, PixelRect};

use crate::dom_query::{evaluate, expression, number, returned};

/// The computed styles reported, in the order `InspectedNode::styles`
/// documents. `background` reads `backgroundColor`.
const STYLES: [(&str, &str); 9] = [
    ("display", "display"),
    ("position", "position"),
    ("font-family", "fontFamily"),
    ("font-size", "fontSize"),
    ("font-weight", "fontWeight"),
    ("color", "color"),
    ("background", "backgroundColor"),
    ("padding", "padding"),
    ("margin", "margin"),
];

const INSPECT_AT: &str = r#"
const compact = (value, max) => {
  if (!value) return undefined;
  const cleaned = value.replace(/\s+/g, ' ').trim();
  if (!cleaned) return undefined;
  return cleaned.length > max ? cleaned.slice(0, max - 1) + '…' : cleaned;
};
let el = null;
for (const candidate of document.elementsFromPoint(__X__, __Y__)) {
  el = candidate;
  break;
}
if (!el) return 'null';
while (el.shadowRoot) {
  const nested = el.shadowRoot.elementFromPoint(__X__, __Y__);
  if (!nested || nested === el) break;
  el = nested;
}
const r = el.getBoundingClientRect();
const tag = el.tagName.toLowerCase();
const text = compact(el.innerText, 80) ?? compact(el.getAttribute('aria-label'), 120)
  ?? compact(el.getAttribute('title'), 120) ?? compact(el.value, 120)
  ?? compact(el.getAttribute('placeholder'), 120) ?? compact(el.getAttribute('alt'), 120);
const style = getComputedStyle(el);
return JSON.stringify({
  nodeId: el.getAttribute('id') || el.getAttribute('data-testid')
    || tag + '@' + Math.round(r.left) + ':' + Math.round(r.top),
  tagName: tag,
  name: text ? tag + ' "' + text + '"' : tag,
  selector: unique(el),
  id: el.getAttribute('id') || null,
  classes: [...el.classList].map((n) => n.trim()).filter(Boolean).slice(0, 6),
  styles: __STYLES__.map(([label, key]) => [label, String(style[key])]),
  x: Math.round(r.left),
  y: Math.round(r.top),
  width: Math.round(r.width),
  height: Math.round(r.height),
});
"#;

/// The message that asks for the node under a viewport point.
pub fn inspect_at_message(id: i32, x: f32, y: f32) -> Vec<u8> {
    let styles = STYLES
        .iter()
        .map(|(label, key)| format!("['{label}','{key}']"))
        .collect::<Vec<_>>()
        .join(",");
    let values = [
        ("__X__", number(x)),
        ("__Y__", number(y)),
        ("__STYLES__", format!("[{styles}]")),
    ];
    evaluate(id, &expression(INSPECT_AT, &values))
}

/// The node an [`inspect_at_message`] found. `None` for a point on no
/// element, a page that threw and an answer that cannot be read.
pub fn parse_inspected(result: &[u8]) -> Option<InspectedNode> {
    let found: Value = serde_json::from_str(&returned(result)?).ok()?;
    let text = |key: &str| Some(found.get(key)?.as_str()?.to_owned());
    let int = |key: &str| found.get(key)?.as_i64();
    let side = |key: &str| Some(u32::try_from(int(key)?.max(0)).unwrap_or(u32::MAX));
    let classes = (found.get("classes")?.as_array()?.iter())
        .filter_map(|class| class.as_str().map(str::to_owned))
        .collect();
    let styles = (found.get("styles")?.as_array()?.iter())
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            Some((
                pair.first()?.as_str()?.to_owned(),
                pair.get(1)?.as_str()?.to_owned(),
            ))
        })
        .collect();
    Some(InspectedNode {
        node_id: text("nodeId")?,
        tag_name: text("tagName")?,
        name: text("name")?,
        selector: text("selector")?,
        id_attribute: text("id").filter(|id| !id.is_empty()),
        classes,
        styles,
        bounding_box: PixelRect::new(
            i32::try_from(int("x")?).ok()?,
            i32::try_from(int("y")?).ok()?,
            side("width")?,
            side("height")?,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn value_result(value: &str) -> Vec<u8> {
        json!({ "result": { "type": "string", "value": value } })
            .to_string()
            .into_bytes()
    }

    #[test]
    fn the_question_evaluates_one_expression_at_the_point() {
        let message: Value = serde_json::from_slice(&inspect_at_message(9, 12.5, 40.0)).unwrap();
        assert_eq!(message["id"], 9);
        assert_eq!(message["method"], "Runtime.evaluate");
        let expression = message["params"]["expression"].as_str().unwrap();
        assert!(expression.contains("document.elementsFromPoint((12.5), (40))"));
        assert!(expression.contains("['background','backgroundColor']"));
        assert!(!expression.contains("__"), "a placeholder was left in");
    }

    #[test]
    fn an_answer_becomes_an_inspected_node() {
        let answer = json!({
            "nodeId": "save", "tagName": "button", "name": "button \"Save\"",
            "selector": "#save", "id": "save", "classes": ["btn", "primary"],
            "styles": [["display", "block"], ["color", "rgb(0, 0, 0)"]],
            "x": 40, "y": 61, "width": 72, "height": 32,
        })
        .to_string();
        let node = parse_inspected(&value_result(&answer)).unwrap();
        assert_eq!(node.node_id, "save");
        assert_eq!(node.name, "button \"Save\"");
        assert_eq!(node.id_attribute.as_deref(), Some("save"));
        assert_eq!(node.classes, ["btn", "primary"]);
        assert_eq!(node.style("color"), Some("rgb(0, 0, 0)"));
        assert_eq!(node.bounding_box, PixelRect::new(40, 61, 72, 32));
    }

    #[test]
    fn no_element_a_thrown_error_and_noise_are_all_no_node() {
        assert_eq!(parse_inspected(&value_result("null")), None);
        let thrown = br#"{"result":{"type":"object"},"exceptionDetails":{"text":"Uncaught"}}"#;
        assert_eq!(parse_inspected(thrown), None);
        assert_eq!(parse_inspected(b"not json"), None);
        assert_eq!(parse_inspected(&value_result(r#"{"nodeId":"a"}"#)), None);
    }
}
