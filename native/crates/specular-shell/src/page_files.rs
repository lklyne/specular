//! The files hosted pages load from disk, watched so that a page loads
//! again when what it shows changes.
//!
//! A page at a `file://` URL (an HTML file node, or a link to a file) is
//! watched with the stylesheets, scripts and data beside it, since editing
//! one of those is editing the page. Other pages and images beside it are
//! left out: they change nothing the page has loaded.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use specular_doc::EntityId;
use specular_interact::file_url_path;

use crate::watch::{self, Polled};

/// The files beside a page that it may have loaded.
const SIBLINGS: [&str; 4] = ["css", "js", "mjs", "json"];

/// The files of the hosted pages, and how they looked when last seen.
#[derive(Default)]
pub(crate) struct PageFiles {
    /// The file each page loads, and it with the siblings beside it.
    loads: HashMap<EntityId, (PathBuf, Vec<PathBuf>)>,
    polled: Polled<EntityId>,
    checked_at: Option<Instant>,
}

impl PageFiles {
    /// Watches the file the page `page` loads from `url`, if it is one, in
    /// place of any it loaded before.
    pub(crate) fn hosted(&mut self, page: &EntityId, url: &str) {
        self.closed(page);
        if let Some(path) = file_url_path(url) {
            let paths = watched_paths(&path);
            self.polled.watch(page.clone(), paths.clone());
            self.loads.insert(page.clone(), (path, paths));
        }
    }

    /// Stops watching the file of the page `page`.
    pub(crate) fn closed(&mut self, page: &EntityId) {
        self.loads.remove(page);
        self.polled.forget(page);
    }

    /// Whether `url` is the file watched for the page `page`.
    pub(crate) fn watches(&self, page: &EntityId, url: &str) -> bool {
        (self.loads.get(page))
            .is_some_and(|(watched, _)| file_url_path(url).is_some_and(|path| path == *watched))
    }

    /// The pages whose files changed since they were last looked at: one
    /// was edited, or a sibling came or went. Looks at most every
    /// [`watch::INTERVAL`].
    pub(crate) fn changed(&mut self, now: Instant) -> Vec<EntityId> {
        if self.polled.is_empty()
            || (self.checked_at).is_some_and(|at| now.duration_since(at) < watch::INTERVAL)
        {
            return Vec::new();
        }
        self.checked_at = Some(now);
        let mut changed = self.polled.changed();
        for (page, (path, paths)) in &mut self.loads {
            let now = watched_paths(path);
            if now != *paths {
                self.polled.watch(page.clone(), now.clone());
                *paths = now;
                if !changed.contains(page) {
                    changed.push(page.clone());
                }
            }
        }
        changed
    }
}

/// The page's own file and the siblings beside it, in a stable order.
fn watched_paths(path: &Path) -> Vec<PathBuf> {
    let mut paths = vec![path.to_owned()];
    if let Some(folder) = path.parent() {
        let entries = fs::read_dir(folder).into_iter().flatten().flatten();
        paths.extend((entries.map(|entry| entry.path())).filter(|entry| {
            entry != path
                && (entry.extension().and_then(|it| it.to_str()))
                    .is_some_and(|it| SIBLINGS.iter().any(|known| it.eq_ignore_ascii_case(known)))
        }));
        paths[1..].sort();
    }
    paths
}
