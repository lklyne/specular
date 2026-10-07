//! Where a file entity's `file` is on disk.
//!
//! A `.canvas` names files relative to its space folder, the directory it
//! sits in (`docs/file-formats.md`). Electron's `filePathToSrc` also lets a
//! path be absolute, a `local-file://` URL, or an `http(s)` URL.

use std::path::{Path, PathBuf};

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
    fn a_relative_path_is_inside_the_space_folder() {
        assert_eq!(
            resolved("assets/shot.png").as_deref(),
            Some("/Users/me/Specular/assets/shot.png")
        );
    }

    #[test]
    fn an_absolute_path_is_itself() {
        assert_eq!(
            resolved("/tmp/photo.jpg").as_deref(),
            Some("/tmp/photo.jpg")
        );
    }

    #[test]
    fn a_local_file_url_is_its_decoded_path_without_the_query() {
        assert_eq!(
            resolved("local-file:///tmp/my%20photo%231.png?v=3").as_deref(),
            Some("/tmp/my photo#1.png")
        );
    }

    #[test]
    fn a_web_url_is_not_on_disk() {
        assert_eq!(resolved("https://example.com/a.png"), None);
        assert_eq!(resolved("http://example.com/a.png"), None);
    }

    #[test]
    fn a_relative_path_needs_a_space_folder() {
        assert_eq!(resolve("assets/shot.png", None), None);
        assert!(resolve("/tmp/photo.jpg", None).is_some());
    }

    #[test]
    fn a_stray_percent_is_kept() {
        assert_eq!(percent_decode("100%.png"), "100%.png");
        assert_eq!(percent_decode("a%zzb%4"), "a%zzb%4");
    }
}
