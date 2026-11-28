//! The devtools messages that carry the page bridge (ADR 0055), and reading
//! a page's calls on it. Pure, like [`dom_query`](crate::dom_query).
//!
//! The bridge is a `Runtime.addBinding` named
//! [`BRIDGE_BINDING`](specular_core::BRIDGE_BINDING): a function on the
//! page's `window` whose every call comes back as a `Runtime.bindingCalled`
//! event. The app's install script, added for new documents and evaluated
//! in the one shown, wraps it as `window.specular`; answers go back as
//! scripts evaluated in the page.

use serde::Deserialize;
use serde_json::{Value, json};
use specular_core::BRIDGE_BINDING;

/// The methods that put the bridge in: the binding, then `script` for every
/// later document when `for_new_documents`, then `script` in the document
/// shown now.
pub(crate) fn attach_methods(
    script: &str,
    bind: bool,
    for_new_documents: bool,
) -> Vec<(&'static str, Value)> {
    let mut methods = Vec::new();
    if bind {
        methods.push(("Runtime.addBinding", json!({ "name": BRIDGE_BINDING })));
    }
    if for_new_documents {
        let source = json!({ "source": script });
        methods.push(("Page.addScriptToEvaluateOnNewDocument", source));
    }
    methods.push(("Runtime.evaluate", evaluate_params(script)));
    methods
}

/// The method that takes the bridge out. The install script stays added
/// for new documents, and finds no binding there to wrap.
pub(crate) fn detach_method() -> (&'static str, Value) {
    ("Runtime.removeBinding", json!({ "name": BRIDGE_BINDING }))
}

/// `Runtime.evaluate`'s params for `script`, whose result is not read.
pub(crate) fn evaluate_params(script: &str) -> Value {
    json!({ "expression": script, "returnByValue": false, "silent": true })
}

#[derive(Deserialize)]
struct Event<'a> {
    #[serde(borrow)]
    method: Option<&'a str>,
    params: Option<Called>,
}

#[derive(Deserialize)]
struct Called {
    name: String,
    payload: String,
}

/// The payload of a call on the bridge binding, when `message` is one.
pub(crate) fn bridge_call(message: &[u8]) -> Option<String> {
    let event: Event<'_> = serde_json::from_slice(message).ok()?;
    if event.method != Some("Runtime.bindingCalled") {
        return None;
    }
    let called = event.params?;
    (called.name == BRIDGE_BINDING).then_some(called.payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_call_on_the_bridge_binding_is_read_as_one() {
        let called = |name: &str| {
            json!({
                "method": "Runtime.bindingCalled",
                "params": { "name": name, "payload": "{\"id\":1}", "executionContextId": 3 },
            })
            .to_string()
        };
        let rows = [
            (called(BRIDGE_BINDING), Some("{\"id\":1}".to_owned())),
            (called("somebodyElse"), None),
            (
                json!({ "method": "Page.loadEventFired", "params": {} }).to_string(),
                None,
            ),
            (json!({ "id": 4, "result": {} }).to_string(), None),
        ];
        for (message, payload) in rows {
            assert_eq!(bridge_call(message.as_bytes()), payload, "{message}");
        }
    }
}
