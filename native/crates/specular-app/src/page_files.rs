//! HTML files hosted as pages: the URL each loads from, and which changed
//! on disk since it loaded.
//!
//! A file entity names its file relative to the space folder, as an image's
//! does, and the page loads it as a `file://` URL so that the stylesheets,
//! scripts and images it links to are found next to it. Only the HTML file
//! itself is watched: a change to one of those is seen on the next reload.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use specular_doc::EntityId;
use specular_interact::PageAddress;

use crate::images::resolve::resolve;
use crate::persist::{Stamp, stamp};

/// What a page whose file cannot be found loads: nothing.
const BLANK: &str = "about:blank";

/// How often the watched files are looked at, as the note thread does.
const CHECK_INTERVAL: Duration = Duration::from_millis(500);

/// The URL a host loads for `address`, with relative files in `space`.
pub(crate) fn page_url(address: &PageAddress, space: Option<&Path>) -> String {
    match address {
        PageAddress::Url(url) => url.clone(),
        PageAddress::File(file) => {
            if let Some(path) = resolve(file, space) {
                return file_url(&path);
            }
            tracing::warn!(file, "html file is not on disk; nothing to show");
            BLANK.to_owned()
        }
    }
}

/// `path` as a `file://` URL, with every byte a URL path cannot hold as is
/// percent-encoded.
fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for &byte in path.to_string_lossy().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~!$&'()*+,;=:@".contains(&byte) {
            url.push(byte as char);
        } else {
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}

struct Watched {
    path: PathBuf,
    stamp: Option<Stamp>,
}

/// The HTML files of the hosted pages, and their stamps when they loaded.
#[derive(Default)]
pub(crate) struct PageFiles {
    watched: HashMap<EntityId, Watched>,
    checked_at: Option<Instant>,
}

impl PageFiles {
    /// Watches the file the page `page` loads, if it loads one from disk.
    pub(crate) fn hosted(&mut self, page: &EntityId, address: &PageAddress, space: Option<&Path>) {
        self.watched.remove(page);
        if let PageAddress::File(file) = address
            && let Some(path) = resolve(file, space)
        {
            let stamp = stamp(&path);
            self.watched.insert(page.clone(), Watched { path, stamp });
        }
    }

    /// Stops watching the file of the page `page`.
    pub(crate) fn closed(&mut self, page: &EntityId) {
        self.watched.remove(page);
    }

    /// The pages whose file changed since it was last looked at. Looks at
    /// most every [`CHECK_INTERVAL`].
    pub(crate) fn changed(&mut self, now: Instant) -> Vec<EntityId> {
        if self.watched.is_empty()
            || (self.checked_at).is_some_and(|at| now.duration_since(at) < CHECK_INTERVAL)
        {
            return Vec::new();
        }
        self.checked_at = Some(now);
        let mut changed = Vec::new();
        for (page, watched) in &mut self.watched {
            let now = stamp(&watched.path);
            if now != watched.stamp {
                watched.stamp = now;
                changed.push(page.clone());
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_html_file_loads_from_the_space_folder() {
        let space = Some(Path::new("/Users/me/My Space"));
        for (address, expected) in [
            (
                PageAddress::File("assets/file_d2.html".to_owned()),
                "file:///Users/me/My%20Space/assets/file_d2.html",
            ),
            (
                PageAddress::File("/tmp/a#1.htm".to_owned()),
                "file:///tmp/a%231.htm",
            ),
            (
                PageAddress::Url("https://example.com/a b".to_owned()),
                "https://example.com/a b",
            ),
        ] {
            assert_eq!(page_url(&address, space), expected, "{address:?}");
        }
        let relative = PageAddress::File("assets/a.html".to_owned());
        assert_eq!(page_url(&relative, None), BLANK);
    }

    #[test]
    fn a_page_is_told_once_that_its_file_changed() {
        let folder =
            std::env::temp_dir().join(format!("specular-page-files-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.html"), "<p>one</p>").unwrap();
        let page = EntityId::from("p1");
        let mut files = PageFiles::default();
        let address = PageAddress::File("a.html".to_owned());
        files.hosted(&page, &address, Some(&folder));
        let start = Instant::now();
        assert_eq!(files.changed(start), []);
        std::fs::write(folder.join("a.html"), "<p>two, longer</p>").unwrap();
        let later = start + CHECK_INTERVAL;
        assert_eq!(files.changed(later), std::slice::from_ref(&page));
        assert_eq!(files.changed(later + CHECK_INTERVAL), []);
        files.closed(&page);
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
