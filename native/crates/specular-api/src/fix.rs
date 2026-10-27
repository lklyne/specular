//! `POST /annotations/fix`: send one comment, or every open comment on an
//! origin, to the agent on the origin's repo. The answer is Electron's:
//! `{ ok, queued }`, where `queued` counts the comments sent.

use serde_json::json;
use specular_doc::AnnotationId;
use specular_interact::{Action, ApiRun, App, ChatAction, FixTarget, fixable};

use crate::reply::Reply;
use crate::{Request, Response, Step};

pub(crate) fn fix(app: &App, request: &Request) -> Result<Step, Response> {
    let target = match (request.text("annotationId"), request.text("origin")) {
        (Some(id), _) => FixTarget::Comment(AnnotationId::new(id)),
        (None, Some(origin)) => FixTarget::Origin(origin.to_owned()),
        (None, None) => return Err(Response::bad_request("origin or annotationId is required")),
    };
    let queued = fixable(app, &target).len();
    if queued == 0 {
        // A comment that cannot be sent is a failure; an origin with
        // nothing open is not.
        let ok = matches!(target, FixTarget::Origin(_));
        return Ok(Step::Answer(json!({ "ok": ok, "queued": 0 })));
    }
    let run = ApiRun::Act {
        on: None,
        action: Action::Chat(ChatAction::Fix(target)),
    };
    Ok(Step::Run(
        run,
        Reply::Fixed(json!({ "ok": true, "queued": queued })),
    ))
}
