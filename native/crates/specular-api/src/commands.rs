//! The command registry over HTTP: `GET /commands` lists it and
//! `POST /run {name, args}` runs one by name, through `update` like every
//! other write.

use serde_json::{Value, json};
use specular_interact::{ApiRun, App, Run, RunError, commands, run_named};

use crate::reply::Reply;
use crate::{Request, Response, Step};

/// `GET /commands`: every command, and whether it can run now.
pub(crate) fn list(app: &App) -> Value {
    let listed: Vec<Value> = (commands().iter())
        .map(|spec| {
            json!({
                "name": spec.name,
                "args": spec.args,
                "about": spec.about,
                "label": spec.label,
                "enabled": (spec.enabled)(app),
            })
        })
        .collect();
    json!({ "commands": listed })
}

/// `POST /run`: the command `name` with `args`, as one write.
pub(crate) fn run(app: &App, request: &Request) -> Result<Step, Response> {
    let name = (request.text("name"))
        .ok_or_else(|| Response::bad_request("name is required"))?
        .to_owned();
    let args = request.body.get("args").cloned().unwrap_or(Value::Null);
    let run = run_named(app, &name, &args).map_err(|error| {
        let status = match &error {
            RunError::Unknown { .. } | RunError::Control(_) => 404,
            RunError::Args(_) => 400,
            RunError::Disabled(_) => 409,
        };
        Response::error(status, error.to_string())
    })?;
    let run = match run {
        Run::Action(action) => ApiRun::Act { on: None, action },
        Run::Api(run) => run,
        Run::Shell(_) => {
            return Err(Response::error(
                409,
                format!("`{name}` is the window's own command and is not run over HTTP"),
            ));
        }
    };
    Ok(Step::Run(
        run,
        Reply::Fixed(json!({ "ok": true, "name": name })),
    ))
}
