//! Repo bindings: which local folder a site's source lives in.
//!
//! `specular-agent` holds the model ([`Repos`](specular_agent::Repos)) in the
//! shape of the Electron app's `repos.json`. This module owns it on the
//! [`App`] and changes it through [`RepoAction`]. The sidebar shows each
//! repo's bindings on its project (ADR 0062).

use crate::{App, Effect};

/// A change to the connected repos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoAction {
    /// Ask the user for a folder to bind `origin` to, or with no origin, a
    /// folder to connect.
    Pick(Option<String>),
    /// Connect the folder at this path with no origin bound.
    Connect(String),
    /// Bind an origin to the folder at `path`, connecting it first.
    Bind {
        /// The origin, or any URL of it.
        origin: String,
        /// The repo's folder.
        path: String,
    },
    /// Bind an origin, or any URL of it, to a connected repo by its id.
    BindTo {
        /// The repo.
        repo: String,
        /// The origin, or any URL of it.
        origin: String,
    },
    /// Drop the binding of an origin.
    Unlink(String),
    /// Turn auto-fix on or off for a bound origin.
    SetAutoFix {
        /// The origin.
        origin: String,
        /// Whether its comments send themselves.
        on: bool,
    },
    /// Disconnect a repo by its id, with its bindings.
    Disconnect(String),
}

impl App {
    /// The connected repos and the origins bound to them.
    pub fn repos(&self) -> &specular_agent::Repos {
        &self.repos
    }
}

/// Runs `action`. Bindings are app settings, not part of the document, so
/// nothing here is undoable; the file is saved only when something changed.
pub(crate) fn run(app: &mut App, action: RepoAction, effects: &mut Vec<Effect>) {
    let before = app.repos.clone();
    match action {
        RepoAction::Pick(origin) => {
            effects.push(Effect::PickRepoFolder { origin });
            return;
        }
        // A folder connected for a site's source is a project too (ADR
        // 0062), so the sidebar lists it and its canvases.
        RepoAction::Connect(path) => {
            app.repos.connect(&path);
            effects.push(Effect::OpenProject(path));
        }
        RepoAction::Bind { origin, path } => {
            app.repos.bind(&origin, &path);
            effects.push(Effect::OpenProject(path));
        }
        RepoAction::BindTo { repo, origin } => {
            app.repos.bind_to(&repo, &origin);
        }
        RepoAction::Unlink(origin) => {
            app.repos.unbind(&origin);
        }
        RepoAction::SetAutoFix { origin, on } => {
            app.repos.set_auto_fix(&origin, on);
        }
        RepoAction::Disconnect(repo) => {
            app.repos.disconnect(&repo);
        }
    }
    if app.repos != before {
        effects.push(Effect::SaveRepos);
    }
}
