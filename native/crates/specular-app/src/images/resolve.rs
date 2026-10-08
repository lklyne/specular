//! Where a file entity's `file` is on disk.
//!
//! A `.canvas` names files relative to its space folder, the directory it
//! sits in (`docs/file-formats.md`). Electron's `filePathToSrc` also lets a
//! path be absolute, a `local-file://` URL, or an `http(s)` URL.

use std::path::{Path, PathBuf};

/// What `resources/starter-space/Welcome.canvas` writes for its space
/// folder.
const SPACE_TOKEN: &str = "__SPECULAR_SPACE__/";

/// The file to read for `file`, or `None` when there is nothing on disk to
/// read: a web URL, or a relative path with no space folder to start from.
pub(crate) fn resolve(file: &str, space: Option<&Path>) -> Option<PathBuf> {
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
        resolve(file, Some(Path::new(SPACE))).map(|path| path.display().to_string())
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
        assert_eq!(resolve("assets/shot.png", None), None);
        assert_eq!(resolve("__SPECULAR_SPACE__/Welcome.md", None), None);
        assert!(resolve("/tmp/photo.jpg", None).is_some());
    }

    #[test]
    fn a_stray_percent_is_kept() {
        assert_eq!(percent_decode("100%.png"), "100%.png");
        assert_eq!(percent_decode("a%zzb%4"), "a%zzb%4");
    }
}
