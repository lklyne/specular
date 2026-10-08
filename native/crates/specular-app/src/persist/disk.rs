//! The file calls: stat, read, and a write that never leaves half a file.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use super::file_sync::Stamp;

/// The file's stamp, or `None` when it cannot be read (it may be mid-rename
/// by another writer).
pub(crate) fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = fs::metadata(path).ok()?;
    Some(Stamp {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

/// Writes `text` to `path` by writing a sibling temp file and renaming it
/// over `path`, so a reader or a crash sees the old file or the new one.
pub(crate) fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    let temp = temp_path(path)?;
    let written = File::create(&temp).and_then(|mut file| {
        file.write_all(text.as_bytes())?;
        file.sync_all()
    });
    let renamed = written.and_then(|()| fs::rename(&temp, path));
    if renamed.is_err() {
        // Best effort: the temp file is ours and nothing reads it.
        let _ = fs::remove_file(&temp);
    }
    renamed
}

/// A hidden sibling of `path`, in the same directory so the rename stays on
/// one file system.
fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".{}.tmp", std::process::id()));
    Ok(path.with_file_name(temp_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-app-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_write_replaces_the_file_and_leaves_no_temp_file() {
        let dir = TempDir::new("write");
        let path = dir.0.join("board.canvas");
        fs::write(&path, "old").unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        let names: Vec<_> = fs::read_dir(&dir.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["board.canvas"]);
    }

    #[test]
    fn a_write_that_cannot_land_leaves_the_old_file() {
        let dir = TempDir::new("blocked");
        // A directory where the file should be: the rename fails.
        let path = dir.0.join("board.canvas");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), "old").unwrap();
        assert!(write_atomic(&path, "new").is_err());
        assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "old");
        assert_eq!(
            fs::read_dir(&dir.0).unwrap().count(),
            1,
            "no temp file left"
        );
    }

    #[test]
    fn the_stamp_follows_the_file() {
        let dir = TempDir::new("stamp");
        let path = dir.0.join("board.canvas");
        assert_eq!(stamp(&path), None);
        fs::write(&path, "four").unwrap();
        let first = stamp(&path).unwrap();
        assert_eq!(first.len, 4);
        // The same bytes with another modification time are another stamp.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_ne!(stamp(&path).unwrap(), first);
        fs::write(&path, "longer").unwrap();
        assert_eq!(stamp(&path).unwrap().len, 6);
    }
}
