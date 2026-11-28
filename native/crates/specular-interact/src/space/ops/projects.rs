//! Opening, closing and setting up the projects beside the first (ADR
//! 0062).

use super::swap::{activate, new_canvas};
use super::{entry, fresh};
use crate::space::{DEFAULT_CANVAS_NAME, OpenedSpace, ProjectId};
use crate::{App, Effect, ProjectAction};

/// Adds a project read from disk after the others, its canvases in the
/// background. A project already open, or a canvas in no folder, changes
/// nothing. A project with no canvas gets an empty one.
pub(crate) fn add_project(app: &mut App, opened: OpenedSpace, effects: &mut Vec<Effect>) {
    let Some(project) = opened.project else {
        return;
    };
    if app.space.project(&project.id).is_some() {
        return;
    }
    let id = project.id.clone();
    app.space.projects.push(project);
    for canvas in opened.canvases {
        let entry = entry(&app.space.canvases, &id, canvas);
        app.space.canvases.push(entry);
    }
    if app.space.canvases_of(&id).next().is_none() {
        let fresh = fresh();
        let canvas = new_canvas(
            &mut app.session,
            &app.space.canvases,
            &id,
            DEFAULT_CANVAS_NAME.to_owned(),
            fresh,
        );
        effects.push(Effect::WriteCanvas(canvas.id.clone()));
        app.space.canvases.push(canvas);
    }
    effects.push(Effect::SaveSpaceMeta);
    effects.push(Effect::SaveProjects);
    effects.push(Effect::LoadProjectThreads(id));
}

/// Starts taking the project `id` out of the list; nothing on disk
/// changes. The last project stays. When the canvas showing is in it, the
/// first canvas of another project is shown, and the shell writes what is
/// unsaved before [`remove_project`] lets the canvases go.
pub(crate) fn close_project(app: &mut App, id: &ProjectId, effects: &mut Vec<Effect>) -> bool {
    if app.space.projects.len() < 2 || app.space.project(id).is_none() {
        return false;
    }
    if app.space.active().project == *id {
        let Some(next) = app
            .space
            .canvases
            .iter()
            .position(|canvas| canvas.project != *id)
        else {
            return false;
        };
        activate(app, next, effects);
    }
    effects.push(Effect::CloseProject(id.clone()));
    true
}

/// Takes the project `id` and its canvases out of the list, once the shell
/// wrote them. Refused while a canvas of it is the one showing.
pub(crate) fn remove_project(app: &mut App, id: &ProjectId, effects: &mut Vec<Effect>) {
    if app.space.active().project == *id || app.space.projects.len() < 2 {
        return;
    }
    let active = app.space.active().id.clone();
    app.space.canvases.retain(|canvas| canvas.project != *id);
    app.space.projects.retain(|project| project.id != *id);
    app.space.active = app.space.index_of(&active).unwrap_or(0);
    effects.push(Effect::SaveProjects);
}

/// Runs a change to a project. Like a canvas change, refused while a drag
/// is in flight.
pub(crate) fn project_act(app: &mut App, action: ProjectAction, effects: &mut Vec<Effect>) {
    if app.session.gesture.is_some() {
        return;
    }
    match action {
        ProjectAction::Open => effects.push(Effect::ChooseProject),
        ProjectAction::OpenFolder(folder) => effects.push(Effect::OpenProject(folder)),
        ProjectAction::Close(id) => {
            close_project(app, &id, effects);
        }
        ProjectAction::Reveal(id) => {
            if app.space.project(&id).is_some() {
                effects.push(Effect::RevealProject(id));
            }
        }
        ProjectAction::Bind { project, origin } => {
            let Some(folder) = app
                .space
                .project(&project)
                .map(|entry| entry.folder.clone())
            else {
                return;
            };
            let before = app.repos.clone();
            app.repos.bind(&origin, &folder);
            if app.repos != before {
                effects.push(Effect::SaveRepos);
            }
        }
        ProjectAction::SetGit { project, choice } => {
            let Some(entry) = app.space.project_mut(&project) else {
                return;
            };
            if !entry.keeps_canvases_apart() || entry.git == Some(choice) {
                return;
            }
            entry.git = Some(choice);
            effects.push(Effect::WriteGitChoice(project));
            effects.push(Effect::SaveProjects);
        }
    }
}
