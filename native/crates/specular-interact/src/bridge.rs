//! Which pages get the page bridge (ADR 0055, "Trust"): a page whose
//! document is a file inside the open space folder, and no other. A page
//! from the web, a file elsewhere on disk or a `.canvas` someone else wrote
//! that points outside its folder never does.

use std::path::{Component, Path, PathBuf};

use crate::space_file::{file_url_path, resolve_file};

/// Whether the page showing `page_url` may have the bridge, with the space
/// folder at `space_root`. True only for a `file://` URL (or a
/// `local-file://` one, as a `.canvas` may name a file) whose path, with
/// `.` and `..` taken out as written, lies inside the folder. Nothing on
/// disk is read: a link inside the folder is the folder owner's own.
#[must_use]
pub fn bridge_allowed(page_url: &str, space_root: &Path) -> bool {
    let path = if page_url.starts_with("file://") {
        file_url_path(page_url)
    } else if page_url.starts_with("local-file://") {
        resolve_file(page_url, None)
    } else {
        None
    };
    let (Some(path), Some(root)) = (path.as_deref().and_then(lexical), lexical(space_root)) else {
        return false;
    };
    path != root && path.starts_with(&root)
}

/// `path` with `.` dropped and each `..` taking off the part before it,
/// as written. `None` for a path that is not absolute.
fn lexical(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => out.push(part),
            Component::CurDir => {}
            // Above the root is the root: popping it leaves it.
            Component::ParentDir => {
                out.pop();
            }
        }
    }
    Some(out)
}
