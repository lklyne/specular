//! The project routes, behind `specular project` (ADR 0062): list the
//! projects open, open a folder as one, close one.

use serde_json::{Value, json};
use specular_interact::{Action, ApiRun, App, ProjectAction};

use crate::reply::Reply;
use crate::{Request, Response, Step};

/// `GET /projects`: every project open and its canvases, as the `projects`
/// key has them.
pub(crate) fn list(app: &App) -> Value {
    specular_interact::state_key(app, "projects").unwrap_or(Value::Null)
}

/// `POST /projects`: open the folder `folder` as a project beside the
/// others. The folder is read after the answer; `GET /projects` shows it.
pub(crate) fn open(request: &Request) -> Result<Step, Response> {
    let folder = request.text("folder").unwrap_or_default().trim().to_owned();
    if !folder.starts_with('/') {
        return Err(Response::bad_request("`folder` must be an absolute path"));
    }
    let run = ApiRun::Act {
        on: None,
        action: Action::Project(ProjectAction::OpenFolder(folder.clone())),
    };
    Ok(Step::Run(run, Reply::Fixed(json!({ "opening": folder }))))
}

/// `POST /projects/close`: take a project out of the sidebar, by id or
/// label. Nothing on disk changes.
pub(crate) fn close(app: &App, request: &Request) -> Result<Step, Response> {
    let wanted = request.text("project").unwrap_or_default();
    let Some(project) = app.space().find_project(wanted) else {
        return Err(Response::bad_request(format!("unknown project '{wanted}'")));
    };
    if app.space().projects().len() < 2 {
        return Err(Response::bad_request("the last project stays open"));
    }
    let run = ApiRun::Act {
        on: None,
        action: Action::Project(ProjectAction::Close(project.id.clone())),
    };
    Ok(Step::Run(
        run,
        Reply::Fixed(json!({ "closed": project.id.as_str() })),
    ))
}
