//! [`Pending`]: the answer to a write, read off the app once the write has
//! run.

use serde_json::{Value, json};
use specular_doc::AnnotationId;
use specular_interact::{ApiOutcome, App, CanvasId};

use crate::{Response, act, annotations, tabs};

/// What a write answers with once it has run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Reply {
    /// A body known before the write ran.
    Fixed(Value),
    /// The selection, and whether every id asked for named something.
    Selection { ok: bool },
    /// The stack order.
    StackOrder,
    /// What the ungrouped group held, which is now the selection.
    Ungrouped,
    /// A group as it now stands.
    Group(specular_doc::EntityId),
    /// What can be undone and redone now.
    History,
    /// An annotation as it now stands.
    Annotation(AnnotationId),
    /// The canvas just made, found by its name.
    NewTab { name: String },
    /// The canvas just deleted, and what the user is looking at now.
    DeletedTab { deleted: Value, reset: bool },
}

/// The answer to a write that has not run yet. Send the event it came
/// with through `update`, then call [`finish`](Self::finish).
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    ticket: u64,
    reply: Reply,
    /// The background canvas the write ran on, which the answer is read
    /// from.
    canvas: Option<CanvasId>,
}

impl Pending {
    pub(crate) const fn new(ticket: u64, reply: Reply, canvas: Option<CanvasId>) -> Self {
        Self {
            ticket,
            reply,
            canvas,
        }
    }

    /// The ticket of the event this waits on.
    pub const fn ticket(&self) -> u64 {
        self.ticket
    }

    /// The response, given how the event went and the app it left behind.
    pub fn finish(self, app: &App, outcome: &ApiOutcome) -> Response {
        if let ApiOutcome::Refused(reason) = outcome {
            // A tab verb refuses what the caller got wrong: a name that is
            // empty or taken.
            let status = match self.reply {
                Reply::NewTab { .. } => 400,
                _ => 409,
            };
            return Response::error(status, reason.clone());
        }
        let user = app;
        let scoped = self.canvas.as_ref().and_then(|id| app.background(id));
        let app = scoped.as_ref().unwrap_or(app);
        match self.reply {
            Reply::Fixed(body) => Response::ok(body),
            Reply::Selection { ok } => {
                Response::ok(json!({ "ok": ok, "selection": act::selection(app) }))
            }
            Reply::StackOrder => {
                let order: Vec<&str> = (app.document().order().iter())
                    .map(specular_doc::ItemId::as_str)
                    .collect();
                Response::ok(json!({ "ok": true, "entityOrder": order }))
            }
            Reply::Ungrouped => {
                let freed: Vec<&str> = (app.session().selection.entities())
                    .map(specular_doc::EntityId::as_str)
                    .collect();
                Response::ok(json!({ "entityIds": freed }))
            }
            Reply::Group(id) => match crate::placement::arrange::group_json(app, &id) {
                Some(group) => Response::ok(group),
                None => Response::not_found("Nothing to manage"),
            },
            Reply::History => Response::ok(act::history_state(app, true)),
            Reply::Annotation(id) => match app.document().annotation(&id) {
                Some(annotation) => Response::ok(annotations::json(annotation)),
                None => Response::not_found(format!("Annotation not found: {id}")),
            },
            Reply::NewTab { name } => match user.space().resolve(&name) {
                Ok(canvas) => Response::ok(
                    json!({ "id": canvas.id.as_str(), "name": name, "activated": false }),
                ),
                Err(error) => Response::error(500, error.to_string()),
            },
            Reply::DeletedTab { deleted, reset } => Response::ok(json!({
                "deleted": deleted,
                "reset": reset,
                "activeTab": tabs::identity(user)["activeTab"],
            })),
        }
    }
}
