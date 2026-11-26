//! The mods the app ships, and the guide and declarations beside them.
//!
//! They are compiled in and written into the app's mods folder on every
//! start, replacing what was there, so a release that changes one reaches
//! everyone. A folder whose `mod.json` says `cloned_from` is a person's own
//! copy and is never touched.

use std::io;
use std::path::{Path, PathBuf};

/// One file of a shipped mod.
#[derive(Debug, Clone, Copy)]
pub struct File {
    /// Its name inside the mod's folder.
    pub name: &'static str,
    /// What it holds.
    pub text: &'static str,
}

/// A mod the app ships.
#[derive(Debug, Clone, Copy)]
pub struct Shipped {
    /// Its folder's name, which is its id.
    pub name: &'static str,
    /// Its files.
    pub files: &'static [File],
}

/// A shipped mod of `mod.json` and `main.js`, and any other files named,
/// compiled in from `mods/<name>/`.
macro_rules! shipped {
    ($name:literal $(, $file:literal)*) => {
        Shipped {
            name: $name,
            files: &[
                File {
                    name: "mod.json",
                    text: include_str!(concat!("../mods/", $name, "/mod.json")),
                },
                File {
                    name: "main.js",
                    text: include_str!(concat!("../mods/", $name, "/main.js")),
                },
                $(File {
                    name: $file,
                    text: include_str!(concat!("../mods/", $name, "/", $file)),
                },)*
            ],
        }
    };
}

/// Every mod the app ships.
pub const MODS: &[Shipped] = &[
    shipped!("toolbar"),
    shipped!("tabs"),
    shipped!("menu"),
    shipped!("share"),
    shipped!("toast"),
    shipped!("welcome"),
];

/// The `specular` module's declarations, `mods/specular.d.ts`.
pub const DECLARATIONS: &str = include_str!("../specular.d.ts");

/// The guide, `mods/README.md`.
pub const GUIDE: &str = include_str!("../mods/README.md");

/// The shipped mod `name`.
pub fn named(name: &str) -> Option<&'static Shipped> {
    MODS.iter().find(|shipped| shipped.name == name)
}

/// Whether the folder `dir` holds a person's own copy: its `mod.json` says
/// what it was cloned from.
pub fn is_clone(dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join(crate::MANIFEST_FILE)) else {
        return false;
    };
    serde_json::from_str::<serde_json::Value>(&text).is_ok_and(|manifest| {
        manifest
            .get("cloned_from")
            .is_some_and(|from| !from.is_null())
    })
}

/// Writes `text` to `path` whole: a watcher never reads half of it.
fn write(path: &Path, text: &str) -> io::Result<()> {
    if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    let staged = path.with_extension("writing");
    std::fs::write(&staged, text)?;
    std::fs::rename(&staged, path)
}

/// Writes the shipped mods, the guide and the declarations into `mods`,
/// creating it. A file already holding what would be written is left
/// alone, so a mod's watcher is not woken for nothing. Returns the names
/// of the mods left alone as clones.
pub fn install(mods: &Path) -> io::Result<Vec<&'static str>> {
    std::fs::create_dir_all(mods)?;
    write(&mods.join("README.md"), GUIDE)?;
    write(&mods.join("specular.d.ts"), DECLARATIONS)?;
    let mut kept = Vec::new();
    for shipped in MODS {
        let dir = mods.join(shipped.name);
        if is_clone(&dir) {
            kept.push(shipped.name);
            continue;
        }
        std::fs::create_dir_all(&dir)?;
        for file in shipped.files {
            write(&dir.join(file.name), file.text)?;
        }
    }
    Ok(kept)
}

/// Where the reason the mod in `dir` did not load is written: beside its
/// folder, `mods/<name>.error`.
pub fn error_file(dir: &Path) -> PathBuf {
    let name = dir.file_name().map_or_else(
        || "mod".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    dir.with_file_name(format!("{name}.error"))
}

/// Writes why the mod in `dir` did not load.
pub fn write_error(dir: &Path, message: &str) -> io::Result<()> {
    std::fs::write(error_file(dir), format!("{message}\n"))
}

/// Takes away a reason written earlier, after a good load.
pub fn clear_error(dir: &Path) -> io::Result<()> {
    match std::fs::remove_file(error_file(dir)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        Ok(()) | Err(_) => Ok(()),
    }
}
