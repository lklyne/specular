//! `GET /state`: the app as named keys of JSON (ADR 0055), every key or
//! the one `?key=` names.

use serde_json::Value;
use specular_interact::{App, STATE_KEYS, THREAD_KEY, state, state_key};

use crate::{Request, Response, Step};

/// `GET /state` and `GET /state?key=panels/toolbar`.
pub(crate) fn read(app: &App, request: &Request) -> Result<Step, Response> {
    let Some(key) = request.query("key") else {
        return Ok(Step::Answer(Value::Object(
            state(app).into_iter().collect(),
        )));
    };
    state_key(app, key).map(Step::Answer).ok_or_else(|| {
        Response::not_found(format!(
            "no state key `{key}`; the keys: {}, {THREAD_KEY}<thread id>",
            STATE_KEYS.join(", ")
        ))
    })
}
