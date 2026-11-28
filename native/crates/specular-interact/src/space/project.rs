//! [`Project`]: one folder the sidebar lists, and the canvases it holds
//! (ADR 0062).
//!
//! A project is a folder the person opened: a plain folder of canvases or
//! a code repo. Its `folder` is what they picked and where the agent works.
//! Its `canvases` folder is where the `.canvas` files, their assets and the
//! `.specular/` index live: the folder itself for a plain folder, and
//! `.specular/canvases/` inside a repo, so a repo's tree stays its own.

/// Names one project: the id [`folder_id`](specular_agent::folder_id) gives
/// its folder, which is also the folder's id as a repo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProjectId(String);

impl ProjectId {
    /// The id `text` spells.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The id of the project at `folder`.
    pub fn of_folder(folder: &str) -> Self {
        Self(specular_agent::folder_id(folder))
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a repo's canvases go into its git history. The person chooses;
/// until they do, nothing is written for git.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitChoice {
    /// Commit the canvases with the code. Agent transcripts stay out.
    Commit,
    /// Keep the whole `.specular/` folder out of git.
    Ignore,
}

impl GitChoice {
    /// The word the preferences file keeps.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Ignore => "ignore",
        }
    }

    /// The choice `text` names.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "commit" => Some(Self::Commit),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

/// One folder the sidebar lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// Its id.
    pub id: ProjectId,
    /// What the sidebar calls it: the folder's name.
    pub label: String,
    /// The folder the person opened, and where the agent works.
    pub folder: String,
    /// Where its canvases, their assets and the `.specular/` index are.
    pub canvases: String,
    /// Whether the folder is a git repo.
    pub repo: bool,
    /// The branch checked out, for a repo.
    pub branch: Option<String>,
    /// Whether its canvases go into git, once the person has said.
    pub git: Option<GitChoice>,
}

impl Project {
    /// A plain folder of canvases at `folder`, which keeps them itself.
    pub fn folder(folder: impl Into<String>) -> Self {
        let folder = folder.into();
        Self {
            id: ProjectId::of_folder(&folder),
            label: folder_label(&folder),
            canvases: folder.clone(),
            folder,
            repo: false,
            branch: None,
            git: None,
        }
    }

    /// Whether its canvases are in a folder of their own inside it, which
    /// is when the git choice means anything.
    pub fn keeps_canvases_apart(&self) -> bool {
        self.canvases != self.folder
    }
}

/// The last part of `folder`.
pub fn folder_label(folder: &str) -> String {
    let trimmed = folder.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or(trimmed).to_owned()
}
