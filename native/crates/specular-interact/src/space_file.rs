//! Where a file entity's `file` is on disk, and the URL a page loads it
//! from.
//!
//! A `.canvas` names files relative to its space folder, the directory it
//! sits in (`docs/file-formats.md`). Electron's `filePathToSrc` also lets a
//! path be absolute, a `local-file://` URL, or an `http(s)` URL.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// What `resources/starter-space/Welcome.canvas` writes for its space
/// folder.
const SPACE_TOKEN: &str = "__SPECULAR_SPACE__/";

/// What an HTML file whose file cannot be found loads: nothing.
const BLANK: &str = "about:blank";

/// The file to read for `file`, or `None` when there is nothing on disk to
/// read: a web URL, or a relative path with no space folder to start from.
pub fn resolve_file(file: &str, space: Option<&Path>) -> Option<PathBuf> {
    if file.starts_with("http://") || file.starts_with("https://") {
        return None;
    }
    if let Some(rest) = file.strip_prefix("local-file://") {
        // The URL form carries a cache-busting query and is percent-encoded.
        let path = rest.split(['?', '#']).next().unwrap_or(rest);
        return Some(PathBuf::from(percent_decode(path)));
    }
    // The starter space names its own files this way, and Electron writes
    // the folder in when it copies the space. Opened where it lies, the
    // folder is the one the canvas is in.
    let file = file.strip_prefix(SPACE_TOKEN).unwrap_or(file);
    let path = Path::new(file);
    if path.is_absolute() {
        return Some(path.to_owned());
    }
    Some(space?.join(path))
}

/// The URL a page loads the file `file` from: the file on disk as a
/// `file://` URL, so the files it links to are found next to it. A web URL
/// is itself, and a file with no space folder to find it in is blank.
pub(crate) fn file_page_url(file: &str, space: Option<&str>) -> String {
    if file.starts_with("http://") || file.starts_with("https://") {
        return file.to_owned();
    }
    resolve_file(file, space.map(Path::new))
        .map_or_else(|| BLANK.to_owned(), |path| file_url(&path))
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

/// The file a `file://` URL names, or `None` for any other URL.
pub fn file_url_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    Some(PathBuf::from(percent_decode(path)))
}

/// `text` with each `%XX` turned back into its byte. A `%` that is not
/// followed by two hex digits is kept.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = (bytes.get(index + 1..index + 3))
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[index], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                index += 3;
            }
            (byte, _) => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACE: &str = "/Users/me/Specular";

    fn resolved(file: &str) -> Option<String> {
        resolve_file(file, Some(Path::new(SPACE))).map(|path| path.display().to_string())
    }

    #[test]
    fn a_file_names_a_path_in_the_space_or_not_on_disk() {
        for (file, expected) in [
            ("__SPECULAR_SPACE__/Welcome.md", resolved("Welcome.md")),
            (
                "assets/shot.png",
                Some("/Users/me/Specular/assets/shot.png".to_owned()),
            ),
            ("/tmp/photo.jpg", Some("/tmp/photo.jpg".to_owned())),
            (
                "local-file:///tmp/my%20photo%231.png?v=3",
                Some("/tmp/my photo#1.png".to_owned()),
            ),
            ("local-file:///tmp/a.png#top", Some("/tmp/a.png".to_owned())),
            ("https://example.com/a.png", None),
            ("http://example.com/a.png", None),
        ] {
            assert_eq!(resolved(file), expected, "{file}");
        }
    }

    #[test]
    fn a_relative_path_needs_a_space_folder() {
        assert_eq!(resolve_file("assets/shot.png", None), None);
        assert_eq!(resolve_file("__SPECULAR_SPACE__/Welcome.md", None), None);
        assert!(resolve_file("/tmp/photo.jpg", None).is_some());
    }

    #[test]
    fn an_html_file_loads_from_the_space_folder_and_back() {
        let space = Some("/Users/me/My Space");
        for (file, url) in [
            (
                "assets/file_d2.html",
                "file:///Users/me/My%20Space/assets/file_d2.html",
            ),
            ("/tmp/a#1.htm", "file:///tmp/a%231.htm"),
        ] {
            assert_eq!(file_page_url(file, space), url, "{file}");
            assert_eq!(file_url_path(url), resolve_file(file, space.map(Path::new)));
        }
        assert_eq!(file_page_url("assets/a.html", None), BLANK);
        assert_eq!(file_url_path("https://example.com/a.html"), None);
    }

    #[test]
    fn a_stray_percent_is_kept() {
        assert_eq!(percent_decode("100%.png"), "100%.png");
        assert_eq!(percent_decode("a%zzb%4"), "a%zzb%4");
    }
}
