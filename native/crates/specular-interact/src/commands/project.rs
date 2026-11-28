//! The sidebar's projects (ADR 0062): opening and closing one, its git
//! choice, and the sites bound to it.

use serde_json::Value;

use super::CommandSpec;
use super::args::{act, choice, idle, switch, text};
use crate::{Action, GitChoice, ProjectAction, ProjectId, RepoAction, SidebarAction};

fn project(args: &Value) -> Result<ProjectId, super::RunError> {
    text(args, "project").map(ProjectId::new)
}

/// The projects.
pub(super) const PROJECT: &[CommandSpec] = &[
    CommandSpec {
        name: "project.open",
        label: "Open project\u{2026}",
        args: "",
        about: "Ask for a folder, a plain folder or a code repo, and add it to the sidebar as a project.",
        enabled: idle,
        build: |_| act(Action::Project(ProjectAction::Open)),
    },
    CommandSpec {
        name: "project.close",
        label: "",
        args: "{project: id}",
        about: "Take a project out of the sidebar. Its folder and canvases stay on disk; the last project stays.",
        enabled: idle,
        build: |args| act(Action::Project(ProjectAction::Close(project(args)?))),
    },
    CommandSpec {
        name: "project.reveal",
        label: "",
        args: "{project: id}",
        about: "Show a project's folder in Finder.",
        enabled: idle,
        build: |args| act(Action::Project(ProjectAction::Reveal(project(args)?))),
    },
    CommandSpec {
        name: "project.git",
        label: "",
        args: "{project: id, canvases: commit|ignore}",
        about: "Say whether a repo project's canvases go into git: commit them with the code, or keep .specular/ out of git.",
        enabled: idle,
        build: |args| {
            let choice = choice(
                args,
                "canvases",
                &[("commit", GitChoice::Commit), ("ignore", GitChoice::Ignore)],
            )?;
            act(Action::Project(ProjectAction::SetGit {
                project: project(args)?,
                choice,
            }))
        },
    },
    CommandSpec {
        name: "project.fold",
        label: "",
        args: "{project: id}",
        about: "Fold a project's canvases away in the sidebar, or show them.",
        enabled: idle,
        build: |args| act(Action::Sidebar(SidebarAction::Project(project(args)?))),
    },
    CommandSpec {
        name: "project.details",
        label: "",
        args: "{project: id}",
        about: "Show a project's folder, git choice and bound sites under its head in the sidebar, or hide them.",
        enabled: idle,
        build: |args| act(Action::Sidebar(SidebarAction::Details(project(args)?))),
    },
    CommandSpec {
        name: "project.bind",
        label: "",
        args: "{project: id, origin: url}",
        about: "Bind a site to a project: comments on its pages run the agent in the project's folder.",
        enabled: idle,
        build: |args| {
            act(Action::Project(ProjectAction::Bind {
                project: project(args)?,
                origin: text(args, "origin")?,
            }))
        },
    },
    CommandSpec {
        name: "project.unbind",
        label: "",
        args: "{origin: url}",
        about: "Drop a site's binding to its project.",
        enabled: idle,
        build: |args| act(Action::Repo(RepoAction::Unlink(text(args, "origin")?))),
    },
    CommandSpec {
        name: "project.auto-fix",
        label: "",
        args: "{origin: url, on: bool}",
        about: "Whether comments on a bound site send themselves to the agent the moment they are placed.",
        enabled: idle,
        build: |args| {
            act(Action::Repo(RepoAction::SetAutoFix {
                origin: text(args, "origin")?,
                on: switch(args, "on")?,
            }))
        },
    },
];
