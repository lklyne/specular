//! Telling a plain folder from a repo (ADR 0062), and writing a repo's git
//! choice for its canvases.

use std::io;
use std::path::Path;

use specular_interact::{GitChoice, Project};

use super::listing;
use crate::persist::write_atomic;

/// Where a repo keeps its canvases, inside it.
const CANVAS_FOLDER: &str = ".specular/canvases";

/// The project at `folder`. A git repo keeps its canvases in
/// `.specular/canvases/`, unless it already has canvases or a canvas
/// index at its top, as a space folder that is also a repo does: those
/// stay where they are.
pub(crate) fn read(folder: &Path, git: Option<GitChoice>) -> Project {
    let path = folder.to_string_lossy().into_owned();
    let mut project = Project::folder(path.clone());
    let dot_git = folder.join(".git");
    if !dot_git.exists() {
        return project;
    }
    project.repo = true;
    project.branch = branch(&dot_git);
    let has_canvases = !listing::canvas_files(folder).is_empty()
        || folder
            .join(".specular")
            .join("workspace-meta.json")
            .exists();
    if !has_canvases {
        project.canvases = format!("{}/{CANVAS_FOLDER}", path.trim_end_matches('/'));
        project.git = git;
    }
    project
}

/// The branch `HEAD` names, read from the file, so no `git` runs. A
/// worktree's `.git` is a file naming the real folder.
fn branch(dot_git: &Path) -> Option<String> {
    let head = if dot_git.is_file() {
        let pointer = std::fs::read_to_string(dot_git).ok()?;
        let target = pointer.trim().strip_prefix("gitdir:")?.trim().to_owned();
        let target = dot_git.parent()?.join(target);
        std::fs::read_to_string(target.join("HEAD")).ok()?
    } else {
        std::fs::read_to_string(dot_git.join("HEAD")).ok()?
    };
    let head = head.trim();
    match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => Some(branch.to_owned()),
        None => head.get(..7).map(str::to_owned),
    }
}

/// Writes `.specular/.gitignore` in the repo `project` for its git choice:
/// commit keeps only the agent transcripts out, ignore keeps the whole
/// `.specular/` folder out. Nothing is written before the person chooses.
pub(crate) fn write_git_choice(project: &Project) -> io::Result<()> {
    let Some(choice) = project.git else {
        return Ok(());
    };
    let text = match choice {
        GitChoice::Commit => {
            "# Specular: canvases are committed; agent transcripts stay local.\ncanvases/.specular/threads/\n"
        }
        GitChoice::Ignore => "# Specular: canvases stay out of git.\n*\n",
    };
    let folder = Path::new(&project.folder).join(".specular");
    std::fs::create_dir_all(&folder)?;
    write_atomic(&folder.join(".gitignore"), text)
}
