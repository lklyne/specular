//! The share routes, behind `specular share`: connect a canvas to the live
//! room of a share link (ADR 0050), stop, and say whether it is shared.

use serde_json::{Value, json};
use specular_interact::{ApiRun, App};

use crate::reply::Reply;
use crate::{Request, Response, Step};

/// `GET /share`: whether the canvas the user is on is shared.
pub(crate) fn status(app: &App) -> Value {
    json!({ "shared": app.is_shared(), "tab": app.space().active().id.as_str() })
}

/// `POST /share`: connect the canvas to the room `link` opens, or with
/// `join`, a new canvas made for it so the one the user is on stays.
pub(crate) fn start(request: &Request) -> Result<Step, Response> {
    let link = request.text("link").unwrap_or_default().trim().to_owned();
    if !link.contains("/c/") || !link.contains("#t=") {
        return Err(Response::bad_request(
            "link must be a share link: https://<server>/c/<doc>#t=<token>",
        ));
    }
    let fresh = request
        .body
        .get("join")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reply = Reply::Fixed(json!({ "connecting": true, "join": fresh }));
    Ok(Step::Run(ApiRun::Share { link, fresh }, reply))
}

/// `POST /share/stop`: stop sharing the canvas. Its document stays.
pub(crate) fn stop() -> Step {
    Step::Run(ApiRun::Unshare, Reply::Fixed(json!({ "shared": false })))
}
