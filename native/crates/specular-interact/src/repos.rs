//! Repo bindings: which local folder a site's source lives in.
//!
//! `specular-agent` holds the model ([`Repos`](specular_agent::Repos)) in the
//! shape of the Electron app's `repos.json`. This module owns it on the
//! [`App`], changes it through [`RepoAction`], and reads it as the settings
//! dialog's Repos pane.

use crate::{Action, App, Effect};

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

/// The settings dialog's Repos pane (`ReposPane.tsx`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReposPane {
    /// Asks for a folder to connect.
    pub connect: Action,
    /// The connected repos, in the file's order.
    pub repos: Vec<RepoRow>,
}

/// One connected repo.
#[derive(Debug, Clone, PartialEq)]
pub struct RepoRow {
    /// The repo's id.
    pub id: String,
    /// Its label.
    pub label: String,
    /// Its folder.
    pub path: String,
    /// Disconnects it.
    pub disconnect: Action,
    /// The origins bound to it.
    pub origins: Vec<BoundOriginRow>,
}

/// One origin bound to a repo.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundOriginRow {
    /// The origin.
    pub origin: String,
    /// Whether auto-fix is on for it.
    pub auto_fix: bool,
    /// Drops the binding.
    pub remove: Action,
}

impl App {
    /// The connected repos and the origins bound to them.
    pub fn repos(&self) -> &specular_agent::Repos {
        &self.repos
    }
}

/// The Repos pane for `app` as it is now.
pub fn repos_pane(app: &App) -> ReposPane {
    let repos = (app.repos.all().iter())
        .map(|repo| RepoRow {
            id: repo.id.clone(),
            label: repo.label.clone(),
            path: repo.absolute_path.clone(),
            disconnect: Action::Repo(RepoAction::Disconnect(repo.id.clone())),
            origins: (repo.bound_origins.iter())
                .map(|bound| BoundOriginRow {
                    origin: bound.origin.clone(),
                    auto_fix: bound.auto_fix,
                    remove: Action::Repo(RepoAction::Unlink(bound.origin.clone())),
                })
                .collect(),
        })
        .collect();
    ReposPane {
        connect: Action::Repo(RepoAction::Pick(None)),
        repos,
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
        RepoAction::Connect(path) => {
            app.repos.connect(&path);
        }
        RepoAction::Bind { origin, path } => app.repos.bind(&origin, &path),
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
