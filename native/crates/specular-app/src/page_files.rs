//! The files hosted pages load from disk, watched so that a page loads
//! again when what it shows changes.
//!
//! A page at a `file://` URL (an HTML file node, or a link to a file) is
//! watched with the stylesheets, scripts and other pages beside it, since
//! editing one of those is editing the page. Images are left out: a picture
//! dropped into the same folder changes nothing the page has loaded.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use specular_doc::EntityId;
use specular_interact::file_url_path;

/// How often the watched folders are looked at, as the note thread does.
const CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// The files beside a page that it may have loaded.
const SIBLINGS: [&str; 6] = ["html", "htm", "css", "js", "mjs", "json"];

struct Watched {
    /// The file the page loads.
    path: PathBuf,
    /// What its folder's watched files looked like when last looked at.
    stamp: u64,
}

/// The files of the hosted pages, and how they looked when last seen.
#[derive(Default)]
pub(crate) struct PageFiles {
    watched: HashMap<EntityId, Watched>,
    checked_at: Option<Instant>,
}

impl PageFiles {
    /// Watches the file the page `page` loads from `url`, if it is one.
    pub(crate) fn hosted(&mut self, page: &EntityId, url: &str) {
        self.watched.remove(page);
        if let Some(path) = file_url_path(url) {
            let stamp = folder_stamp(&path);
            self.watched.insert(page.clone(), Watched { path, stamp });
        }
    }

    /// Stops watching the file of the page `page`.
    pub(crate) fn closed(&mut self, page: &EntityId) {
        self.watched.remove(page);
    }

    /// The pages whose files changed since they were last looked at. Looks
    /// at most every [`CHECK_INTERVAL`].
    pub(crate) fn changed(&mut self, now: Instant) -> Vec<EntityId> {
        if self.watched.is_empty()
            || (self.checked_at).is_some_and(|at| now.duration_since(at) < CHECK_INTERVAL)
        {
            return Vec::new();
        }
        self.checked_at = Some(now);
        let mut changed = Vec::new();
        for (page, watched) in &mut self.watched {
            let now = folder_stamp(&watched.path);
            if now != watched.stamp {
                watched.stamp = now;
                changed.push(page.clone());
            }
        }
        changed
    }
}

/// One number for the name, size and modification time of `path` and of
/// each watched file beside it. Any of them changing, appearing or going
/// changes it.
fn folder_stamp(path: &Path) -> u64 {
    let watched = |entry: &Path| {
        entry == path
            || (entry.extension().and_then(|it| it.to_str()))
                .is_some_and(|it| SIBLINGS.iter().any(|known| it.eq_ignore_ascii_case(known)))
    };
    let entries = (path.parent())
        .and_then(|folder| fs::read_dir(folder).ok())
        .into_iter()
        .flatten()
        .flatten();
    let mut stamp = 0_u64;
    for entry in entries {
        let entry = entry.path();
        if !watched(&entry) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&entry) else {
            continue;
        };
        let mut hasher = DefaultHasher::new();
        (&entry, metadata.len(), metadata.modified().ok()).hash(&mut hasher);
        // A sum, so the order the folder lists its files in does not matter.
        stamp = stamp.wrapping_add(hasher.finish());
    }
    stamp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_told_once_that_its_file_or_a_script_beside_it_changed() {
        let folder =
            std::env::temp_dir().join(format!("specular-page-files-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("a.html"), "<p>one</p>").unwrap();
        let page = EntityId::from("p1");
        let mut files = PageFiles::default();
        files.hosted(
            &page,
            &format!("file://{}", folder.join("a.html").display()),
        );
        let mut at = Instant::now();
        assert_eq!(files.changed(at), []);
        for (file, text) in [
            ("a.html", "<p>two, longer</p>"),
            ("a.js", "document.title = 'x'"),
        ] {
            fs::write(folder.join(file), text).unwrap();
            at += CHECK_INTERVAL;
            assert_eq!(files.changed(at), std::slice::from_ref(&page), "{file}");
            at += CHECK_INTERVAL;
            assert_eq!(files.changed(at), [], "{file} is said once");
        }
        // A picture beside it is nothing the page loaded.
        fs::write(folder.join("shot.png"), [0_u8; 8]).unwrap();
        assert_eq!(files.changed(at + CHECK_INTERVAL), []);
        files.closed(&page);
        fs::remove_dir_all(&folder).unwrap();
    }
}
