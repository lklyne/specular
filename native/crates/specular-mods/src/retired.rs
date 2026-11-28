//! Shipped mods a release no longer ships. Each shipped mod's folder gets a
//! `.shipped` marker that records what the app wrote there, so a later
//! start can tell its own untouched folder from a person's: a folder that
//! is no longer shipped and still holds exactly what the app wrote is
//! removed, and anything else is left alone. Folders written before the
//! marker existed are recognised by the known contents of the mods retired
//! since ([`RETIRED`]).

use std::fmt::Write as _;
use std::io;
use std::path::Path;

use crate::shipped::{File, LIBRARY_FOLDER, MODS, is_clone};

/// The marker in a shipped mod's folder: one `<file>\t<hash>` line for each
/// file the app wrote.
pub(crate) const MARKER: &str = ".shipped";

/// Files a mod's runtime writes into its folder on load, which are not the
/// person's work.
const WRITTEN_ON_LOAD: [&str; 2] = ["jsconfig.json", "gpui-kit.d.ts"];

/// Mods shipped before the marker, by name, with the hash of every version
/// of each file they shipped with.
const RETIRED: &[(&str, &[Wrote])] = &[(
    "ink",
    &[
        ("main.js", &[0x6ab9_4b5d_260f_c7fd]),
        ("mod.json", &[0x99da_08e0_33db_c05d, 0xb32d_4bd6_5e59_8d4a]),
    ],
)];

/// A file the app wrote, with the hash of every version of it.
type Wrote = (&'static str, &'static [u64]);

/// FNV-1a: stable across builds, which is all a marker needs.
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Records in `dir` what the app just wrote there.
pub(crate) fn mark(dir: &Path, files: &[File]) -> io::Result<()> {
    let mut lines = String::new();
    for file in files {
        let _ = writeln!(lines, "{}\t{:016x}", file.name, hash(file.text.as_bytes()));
    }
    let path = dir.join(MARKER);
    if std::fs::read_to_string(&path).is_ok_and(|old| old == lines) {
        return Ok(());
    }
    std::fs::write(path, lines)
}

/// Removes from `mods` the folders of mods no longer shipped that hold just
/// what the app wrote, and every error file whose mod folder is gone.
/// Returns the names of the folders removed. One that cannot be removed is
/// logged and left.
pub(crate) fn sweep(mods: &Path) -> io::Result<Vec<String>> {
    let mut removed = Vec::new();
    for entry in std::fs::read_dir(mods)?.flatten() {
        let dir = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let ours = dir.is_dir()
            && name != LIBRARY_FOLDER
            && !MODS.iter().any(|shipped| shipped.name == name)
            && !is_clone(&dir)
            && untouched(&dir, &name);
        if !ours {
            continue;
        }
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => removed.push(name),
            Err(error) => tracing::warn!("mod `{name}` is no longer shipped: {error}"),
        }
    }
    for entry in std::fs::read_dir(mods)?.flatten() {
        let path = entry.path();
        let stray =
            path.extension().is_some_and(|ext| ext == "error") && !path.with_extension("").is_dir();
        if stray {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(removed)
}

/// Whether the shipped mod's folder `dir` was edited in place: a file the
/// marker says the app wrote holds neither what it wrote then nor what is
/// in `files`, what it is about to write. Other files in the folder, such
/// as one an older release wrote, are not edits. A folder from before the
/// marker is not edited.
pub(crate) fn edited(dir: &Path, files: &[File]) -> bool {
    let Some(last) = marked(dir) else {
        return false;
    };
    last.iter().any(|(name, hashes)| {
        let Ok(bytes) = std::fs::read(dir.join(name)) else {
            return false;
        };
        let now = hash(&bytes);
        let next = (files.iter())
            .find(|file| file.name == name)
            .map(|file| hash(file.text.as_bytes()));
        !hashes.contains(&now) && next != Some(now)
    })
}

/// What the marker in `dir` says the app wrote there.
fn marked(dir: &Path) -> Option<Vec<(String, Vec<u64>)>> {
    let marker = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    Some(
        (marker.lines())
            .filter_map(|line| {
                let (file, hex) = line.split_once('\t')?;
                Some((file.to_owned(), vec![u64::from_str_radix(hex, 16).ok()?]))
            })
            .collect(),
    )
}

/// Whether `dir` holds exactly the files the app wrote, unchanged.
fn untouched(dir: &Path, name: &str) -> bool {
    let wrote: Vec<(String, Vec<u64>)> = match marked(dir) {
        Some(wrote) => wrote,
        None => match RETIRED.iter().find(|(retired, _)| *retired == name) {
            Some((_, files)) => (files.iter())
                .map(|(file, hashes)| ((*file).to_owned(), hashes.to_vec()))
                .collect(),
            None => return false,
        },
    };
    holds(dir, &wrote)
}

/// Whether `dir` holds exactly the files `wrote` names, each with one of
/// its hashes, besides the marker and what a load writes.
fn holds(dir: &Path, wrote: &[(String, Vec<u64>)]) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut seen = 0;
    for entry in entries.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        if file == MARKER || WRITTEN_ON_LOAD.contains(&file.as_str()) {
            continue;
        }
        let Some((_, hashes)) = wrote.iter().find(|(name, _)| *name == file) else {
            return false;
        };
        let same = std::fs::read(entry.path()).is_ok_and(|bytes| hashes.contains(&hash(&bytes)));
        if !same {
            return false;
        }
        seen += 1;
    }
    seen == wrote.len()
}
