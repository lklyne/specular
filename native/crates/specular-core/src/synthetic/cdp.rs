//! The devtools protocol a synthetic page speaks: enough of `Runtime`,
//! `Page` and `Input` for what is built on a page's devtools channel to run
//! with no browser. Anything else is answered as a browser answers a method
//! it does not have.

use glam::Vec2;
use serde_json::{Value, json};

use super::{SyntheticPage, SyntheticPageSource, title_of};
use crate::page::PageId;
use crate::source::{PageEvent, PageNav, PageSource, PageSourceError};

/// The frame id every synthetic page gives its one frame.
const FRAME: &str = "synthetic-frame";

/// The JSON-RPC code for a method the agent does not have.
const METHOD_NOT_FOUND: i32 = -32601;
/// The JSON-RPC code for a message that is not one.
const INVALID_REQUEST: i32 = -32600;

/// What a page with the `Page` domain enabled reports when it shows `url`.
pub(super) fn navigated_events(url: &str) -> Vec<String> {
    let frame = json!({
        "id": FRAME,
        "loaderId": "synthetic-loader",
        "url": url,
        "securityOrigin": url,
        "mimeType": "text/html",
    });
    vec![
        json!({ "method": "Page.frameNavigated", "params": { "frame": frame } }).to_string(),
        json!({ "method": "Page.loadEventFired", "params": { "timestamp": 0 } }).to_string(),
    ]
}

/// What a method does to the page beyond answering.
enum Then {
    Nothing,
    Navigate(PageNav),
    ScrollBy(Vec2),
}

/// The value of the few expressions a synthetic document can evaluate.
fn evaluate(page: &SyntheticPage, expression: &str) -> Value {
    let url = &page.history[page.at];
    let value = match expression.trim().trim_end_matches(';') {
        "document.title" => json!(title_of(url)),
        "location.href" | "window.location.href" | "document.URL" | "document.location.href" => {
            json!(url)
        }
        "document.readyState" => json!("complete"),
        "window.scrollY" | "scrollY" => json!(page.scroll.y),
        "window.innerWidth" | "innerWidth" => json!(page.spec.viewport.width),
        "window.innerHeight" | "innerHeight" => json!(page.spec.viewport.height),
        _ => return json!({ "result": { "type": "undefined" } }),
    };
    let kind = if value.is_string() {
        "string"
    } else {
        "number"
    };
    json!({ "result": { "type": kind, "value": value } })
}

fn history(page: &SyntheticPage) -> Value {
    let entries: Vec<Value> = (page.history.iter().enumerate())
        .map(|(index, url)| {
            json!({
                "id": index,
                "url": url,
                "userTypedURL": url,
                "title": title_of(url),
                "transitionType": "typed",
            })
        })
        .collect();
    json!({ "currentIndex": page.at, "entries": entries })
}

fn layout(page: &SyntheticPage) -> Value {
    let (width, height) = (page.spec.viewport.width, page.spec.viewport.height);
    let viewport = json!({
        "pageX": page.scroll.x,
        "pageY": page.scroll.y,
        "clientWidth": width,
        "clientHeight": height,
    });
    // The document is three viewports tall and one wide.
    let content = json!({ "x": 0, "y": 0, "width": width, "height": height * 3 });
    json!({
        "layoutViewport": viewport,
        "cssLayoutViewport": viewport,
        "contentSize": content,
        "cssContentSize": content,
    })
}

/// The answer to `method`, and what the page then does. `Err` is a method
/// the page does not have.
fn answer(page: &mut SyntheticPage, method: &str, params: &Value) -> Result<(Value, Then), ()> {
    let done = |value: Value| Ok((value, Then::Nothing));
    match method {
        "Page.enable" => {
            page.page_events = true;
            done(json!({}))
        }
        "Page.disable" => {
            page.page_events = false;
            done(json!({}))
        }
        "Runtime.enable"
        | "Runtime.runIfWaitingForDebugger"
        | "DOM.enable"
        | "Network.enable"
        | "Log.enable" => done(json!({})),
        "Runtime.evaluate" => done(evaluate(
            page,
            params["expression"].as_str().unwrap_or_default(),
        )),
        "Page.getNavigationHistory" => done(history(page)),
        "Page.getLayoutMetrics" => done(layout(page)),
        "Page.getFrameTree" => done(json!({
            "frameTree": { "frame": { "id": FRAME, "url": page.history[page.at] } }
        })),
        "Page.navigate" => {
            let url = params["url"].as_str().unwrap_or_default().to_owned();
            Ok((
                json!({ "frameId": FRAME }),
                Then::Navigate(PageNav::To(url)),
            ))
        }
        "Page.reload" => Ok((json!({}), Then::Navigate(PageNav::Reload))),
        "Input.dispatchMouseEvent" => {
            let wheel = params["type"].as_str() == Some("mouseWheel");
            let delta = |key: &str| params[key].as_f64().unwrap_or_default() as f32;
            let then = if wheel {
                Then::ScrollBy(Vec2::new(delta("deltaX"), delta("deltaY")))
            } else {
                Then::Nothing
            };
            Ok((json!({}), then))
        }
        _ => Err(()),
    }
}

impl SyntheticPageSource {
    /// Queues the page's answer to `message` for the next pump.
    pub(super) fn answer_devtools(
        &mut self,
        id: PageId,
        message: &str,
    ) -> Result<(), PageSourceError> {
        let page = self.page_mut(id)?;
        let request: Value = serde_json::from_str(message).unwrap_or(Value::Null);
        let Some(method) = request["method"].as_str() else {
            let error = json!({ "code": INVALID_REQUEST, "message": "Message must be an object with a method" });
            (self.devtools_out).push((
                id,
                json!({ "id": request["id"], "error": error }).to_string(),
            ));
            return Ok(());
        };
        let id_of = request["id"].clone();
        let (reply, then) = answer(page, method, &request["params"]).map_or_else(
            |()| {
                let message = format!("'{method}' wasn't found");
                let error = json!({ "code": METHOD_NOT_FOUND, "message": message });
                (json!({ "id": id_of, "error": error }), Then::Nothing)
            },
            |(result, then)| (json!({ "id": id_of, "result": result }), then),
        );
        self.devtools_out.push((id, reply.to_string()));
        match then {
            Then::Nothing => Ok(()),
            Then::Navigate(nav) => self.navigate(id, &nav),
            Then::ScrollBy(delta) => {
                let page = self.page_mut(id)?;
                let next = (page.scroll + delta).clamp(Vec2::ZERO, page.max_scroll());
                if next != page.scroll {
                    page.scroll = next;
                    (self.pending).push(PageEvent::Scrolled {
                        page: id,
                        offset: next,
                    });
                }
                Ok(())
            }
        }
    }

    /// Hands every queued devtools message to the sink. With no sink they
    /// are dropped, as a browser with no client attached sends none.
    pub(super) fn flush_devtools(&mut self) {
        let messages = std::mem::take(&mut self.devtools_out);
        if let Some(sink) = &self.devtools_sink {
            for (page, message) in &messages {
                sink(*page, message);
            }
        }
    }
}
