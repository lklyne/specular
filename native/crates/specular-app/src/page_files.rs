//! The files hosted pages load from disk, watched so that a page loads
//! again when what it shows changes.
//!
//! A page at a `file://` URL (an HTML file node, or a link to a file) is
//! watched with the stylesheets, scripts and data beside it, since editing
//! one of those is editing the page. Other pages and images beside it are
//! left out: they change nothing the page has loaded.

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
const SIBLINGS: [&str; 4] = ["css", "js", "mjs", "json"];

struct Watched {
    /// The file the page loads.
    path: PathBuf,
    /// What it and the files beside it looked like when last looked at.
    stamp: u64,
}

/// The files of the hosted pages, and how they looked when last seen.
#[derive(Default)]
pub(crate) struct PageFiles {
    watched: HashMap<EntityId, Watched>,
    checked_at: Option<Instant>,
}

impl PageFiles {
    /// Watches the file the page `page` loads from `url`, if it is one, in
    /// place of any it loaded before.
    pub(crate) fn hosted(&mut self, page: &EntityId, url: &str) {
        self.watched.remove(page);
        if let Some(path) = file_url_path(url) {
            let stamp = stamp(&path, &mut Listings::default());
            self.watched.insert(page.clone(), Watched { path, stamp });
        }
    }

    /// Stops watching the file of the page `page`.
    pub(crate) fn closed(&mut self, page: &EntityId) {
        self.watched.remove(page);
    }

    /// Whether `url` is the file watched for the page `page`.
    pub(crate) fn watches(&self, page: &EntityId, url: &str) -> bool {
        (self.watched.get(page))
            .is_some_and(|watched| file_url_path(url).is_some_and(|path| path == watched.path))
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
        let mut listings = Listings::default();
        let mut changed = Vec::new();
        for (page, watched) in &mut self.watched {
            let now = stamp(&watched.path, &mut listings);
            if now != watched.stamp {
                watched.stamp = now;
                changed.push(page.clone());
            }
        }
        changed
    }
}

/// The hashed sibling files of each folder looked at in one check, so pages
/// that share a folder list it once.
#[derive(Default)]
struct Listings(HashMap<PathBuf, u64>);

impl Listings {
    /// One number for the sibling files in `folder`.
    fn of(&mut self, folder: &Path) -> u64 {
        *(self.0.entry(folder.to_owned())).or_insert_with(|| {
            let entries = fs::read_dir(folder).into_iter().flatten().flatten();
            (entries.map(|entry| entry.path()))
                .filter(|entry| is_sibling(entry))
                // A sum, so the order the folder lists its files in does not
                // matter.
                .fold(0_u64, |stamp, entry| stamp.wrapping_add(file_hash(&entry)))
        })
    }
}

/// One number for the name, size and modification time of `path` and of
/// each sibling file beside it. Any of them changing, appearing or going
/// changes it.
fn stamp(path: &Path, listings: &mut Listings) -> u64 {
    let own = if is_sibling(path) { 0 } else { file_hash(path) };
    let siblings = path.parent().map_or(0, |folder| listings.of(folder));
    own.wrapping_add(siblings)
}

fn is_sibling(path: &Path) -> bool {
    (path.extension().and_then(|it| it.to_str()))
        .is_some_and(|it| SIBLINGS.iter().any(|known| it.eq_ignore_ascii_case(known)))
}

/// The name, size and modification time of `path` as one number, or 0 for
/// a file that is not there.
fn file_hash(path: &Path) -> u64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0;
    };
    let mut hasher = DefaultHasher::new();
    (path, metadata.len(), metadata.modified().ok()).hash(&mut hasher);
    hasher.finish()
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
        // A picture or another page beside it is nothing the page loaded.
        fs::write(folder.join("shot.png"), [0_u8; 8]).unwrap();
        fs::write(folder.join("b.html"), "<p>other</p>").unwrap();
        assert_eq!(files.changed(at + CHECK_INTERVAL), []);
        files.closed(&page);
        fs::remove_dir_all(&folder).unwrap();
    }
}
