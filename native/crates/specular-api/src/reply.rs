//! [`Pending`]: the answer to a write, read off the app once the write has
//! run.

use serde_json::{Value, json};
use specular_doc::AnnotationId;
use specular_interact::{ApiOutcome, App};

use crate::{Response, act, annotations};

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
    /// What can be undone and redone now.
    History,
    /// An annotation as it now stands.
    Annotation(AnnotationId),
}

/// The answer to a write that has not run yet. Send the event it came
/// with through `update`, then call [`finish`](Self::finish).
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    ticket: u64,
    reply: Reply,
}

impl Pending {
    pub(crate) const fn new(ticket: u64, reply: Reply) -> Self {
        Self { ticket, reply }
    }

    /// The ticket of the event this waits on.
    pub const fn ticket(&self) -> u64 {
        self.ticket
    }

    /// The response, given how the event went and the app it left behind.
    pub fn finish(self, app: &App, outcome: &ApiOutcome) -> Response {
        if let ApiOutcome::Refused(reason) = outcome {
            return Response::error(409, reason.clone());
        }
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
            Reply::History => Response::ok(act::history_state(app, true)),
            Reply::Annotation(id) => match app.document().annotation(&id) {
                Some(annotation) => Response::ok(annotations::json(annotation)),
                None => Response::not_found(format!("Annotation not found: {id}")),
            },
        }
    }
}
