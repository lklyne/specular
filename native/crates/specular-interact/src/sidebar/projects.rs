//! The sidebar's projects (ADR 0062): a head for each folder open, its
//! canvases under it, and its details when asked for.

use super::CanvasRow;
use crate::panel::ControlId;
use crate::{
    Action, App, CanvasAction, GitChoice, ProjectAction, ProjectId, RepoAction, SidebarAction,
};

/// One project and its canvases.
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "a plain model a renderer reads: each flag is a fact it draws"
)]
pub struct ProjectSection {
    /// The project.
    pub id: ProjectId,
    /// Its head's name: `sidebar.project.<n>`, by place from 1.
    pub control: ControlId,
    /// What the head says: the folder's name.
    pub label: String,
    /// The folder the person opened.
    pub folder: String,
    /// Whether the folder is a git repo.
    pub repo: bool,
    /// The branch checked out, for a repo.
    pub branch: Option<String>,
    /// Whether the canvas showing is in it.
    pub active: bool,
    /// Whether its canvases are folded away.
    pub folded: bool,
    /// What a click on the head sends: fold or unfold.
    pub toggle: Action,
    /// Whether its details are shown under the head.
    pub details: bool,
    /// What shows or hides the details.
    pub toggle_details: Action,
    /// Its canvases, as the flat list has them.
    pub canvases: Vec<CanvasRow>,
    /// What its add button sends: a new canvas in it.
    pub add_canvas: Action,
    /// Shows its folder in the file manager.
    pub reveal: Action,
    /// Takes it out of the sidebar. `None` for the last project.
    pub close: Option<Action>,
    /// Whether its canvases go into git, for a repo that keeps them in a
    /// folder of their own. `Some(None)` is a repo not decided yet.
    pub git: Option<Option<GitChoice>>,
    /// The sites bound to it.
    pub origins: Vec<OriginRow>,
}

/// A site bound to a project.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginRow {
    /// `scheme://host[:port]`.
    pub origin: String,
    /// Whether comments on it send themselves.
    pub auto_fix: bool,
    /// Turns auto-fix the other way.
    pub toggle_auto_fix: Action,
    /// Drops the binding.
    pub remove: Action,
}

/// A section for each project of `app`, with its rows of `canvases`.
pub(super) fn sections(app: &App, canvases: &[CanvasRow]) -> Vec<ProjectSection> {
    let view = &app.session.sidebar;
    let projects = app.space.projects();
    let active = &app.space.active().project;
    (projects.iter().enumerate())
        .map(|(index, project)| {
            let origins = (app.repos.all().iter())
                .filter(|repo| repo.absolute_path == project.folder)
                .flat_map(|repo| repo.bound_origins.iter())
                .map(|bound| OriginRow {
                    origin: bound.origin.clone(),
                    auto_fix: bound.auto_fix,
                    toggle_auto_fix: Action::Repo(RepoAction::SetAutoFix {
                        origin: bound.origin.clone(),
                        on: !bound.auto_fix,
                    }),
                    remove: Action::Repo(RepoAction::Unlink(bound.origin.clone())),
                })
                .collect();
            ProjectSection {
                id: project.id.clone(),
                control: ControlId::new("sidebar.project").child(index + 1),
                label: project.label.clone(),
                folder: project.folder.clone(),
                repo: project.repo,
                branch: project.branch.clone(),
                active: project.id == *active,
                folded: view.is_project_folded(&project.id),
                toggle: Action::Sidebar(SidebarAction::Project(project.id.clone())),
                details: view.shows_details(&project.id),
                toggle_details: Action::Sidebar(SidebarAction::Details(project.id.clone())),
                canvases: (canvases.iter())
                    .filter(|row| row.project == project.id)
                    .cloned()
                    .collect(),
                add_canvas: Action::Canvas(CanvasAction::New(Some(project.id.clone()))),
                reveal: Action::Project(ProjectAction::Reveal(project.id.clone())),
                close: (projects.len() > 1)
                    .then(|| Action::Project(ProjectAction::Close(project.id.clone()))),
                git: project.keeps_canvases_apart().then_some(project.git),
                origins,
            }
        })
        .collect()
}

/// Every control of a project's head and details, by the name a click on
/// it presses: `sidebar.project.<n>` folds it, and `.add`, `.details`,
/// `.reveal`, `.close`, `.git.commit`, `.git.ignore`,
/// `.origin.<k>.auto` and `.origin.<k>.remove` are beside it.
pub(crate) fn project_controls(section: &ProjectSection) -> Vec<(ControlId, Action)> {
    let head = &section.control;
    let mut out = vec![
        (head.clone(), section.toggle.clone()),
        (head.child("add"), section.add_canvas.clone()),
        (head.child("details"), section.toggle_details.clone()),
        (head.child("reveal"), section.reveal.clone()),
    ];
    if let Some(close) = &section.close {
        out.push((head.child("close"), close.clone()));
    }
    if section.git.is_some() {
        for (word, choice) in [("commit", GitChoice::Commit), ("ignore", GitChoice::Ignore)] {
            let set = Action::Project(ProjectAction::SetGit {
                project: section.id.clone(),
                choice,
            });
            out.push((head.child("git").child(word), set));
        }
    }
    for (index, origin) in section.origins.iter().enumerate() {
        let id = head.child("origin").child(index + 1);
        out.push((id.child("auto"), origin.toggle_auto_fix.clone()));
        out.push((id.child("remove"), origin.remove.clone()));
    }
    out
}
