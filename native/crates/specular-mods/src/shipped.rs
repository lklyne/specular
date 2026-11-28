//! The mods the app ships, and the guide and declarations beside them.
//!
//! They are compiled in and written into the app's mods folder on every
//! start, replacing what was there, so a release that changes one reaches
//! everyone. A folder whose `mod.json` says `cloned_from` is a person's own
//! copy and is never touched. A mod a release stops shipping is removed when
//! its folder still holds just what the app wrote (`retired`).

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
    shipped!("palette"),
    shipped!("confirm"),
    shipped!("prompt"),
    shipped!("project"),
    shipped!("sidebar"),
    shipped!("notes"),
    shipped!("pages"),
    shipped!("dock"),
    shipped!("chat"),
    shipped!("status"),
    shipped!("feed"),
    shipped!("watercolor"),
    shipped!("star"),
    shipped!("thread", "item.html"),
];

/// The folder beside the mods that holds what any mod's web page can load,
/// `mods/lib/`: a page in a mod's folder names it as `../lib/<file>`. It
/// has no `mod.json`, so it is no mod.
pub const LIBRARY_FOLDER: &str = "lib";

/// The shared files of [`LIBRARY_FOLDER`].
pub const LIBRARY: &[File] = &[
    File {
        name: "chat.js",
        text: include_str!("../mods/lib/chat.js"),
    },
    File {
        name: "chat.css",
        text: include_str!("../mods/lib/chat.css"),
    },
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

/// What a start did to the mods folder besides writing the shipped mods.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Installed {
    /// The shipped mods left alone because their folder is a clone.
    pub kept: Vec<&'static str>,
    /// The shipped mods whose folder was edited in place, each with the
    /// folder name its edited copy was moved to.
    pub set_aside: Vec<(&'static str, String)>,
}

/// Writes the shipped mods, the shared library, the guide and the
/// declarations into `mods`, creating it. A file already holding what
/// would be written is left alone, so a mod's watcher is not woken for
/// nothing. A shipped mod's folder someone edited is moved to
/// `<name>-edited` first, as their clone. Removes the folders of mods no
/// longer shipped that the person never changed.
pub fn install(mods: &Path) -> io::Result<Installed> {
    std::fs::create_dir_all(mods)?;
    write(&mods.join("README.md"), GUIDE)?;
    write(&mods.join("specular.d.ts"), DECLARATIONS)?;
    let library = mods.join(LIBRARY_FOLDER);
    std::fs::create_dir_all(&library)?;
    for file in LIBRARY {
        write(&library.join(file.name), file.text)?;
    }
    let mut installed = Installed::default();
    for shipped in MODS {
        let dir = mods.join(shipped.name);
        if is_clone(&dir) {
            installed.kept.push(shipped.name);
            continue;
        }
        if crate::retired::edited(&dir, shipped.files) {
            let to = set_aside(mods, shipped.name)?;
            installed.set_aside.push((shipped.name, to));
        }
        std::fs::create_dir_all(&dir)?;
        for file in shipped.files {
            write(&dir.join(file.name), file.text)?;
        }
        crate::retired::mark(&dir, shipped.files)?;
    }
    crate::retired::sweep(mods)?;
    Ok(installed)
}

/// Moves the edited folder of the shipped mod `name` to the first free
/// `<name>-edited`, `<name>-edited-2`, ..., and marks it a clone of `name`
/// so no later start writes over it. Returns its new folder name.
fn set_aside(mods: &Path, name: &str) -> io::Result<String> {
    let to = (1..u32::MAX)
        .map(|n| match n {
            1 => format!("{name}-edited"),
            n => format!("{name}-edited-{n}"),
        })
        .find(|to| !mods.join(to).exists())
        .ok_or_else(|| io::Error::other(format!("no free folder for `{name}`")))?;
    let dir = mods.join(&to);
    std::fs::rename(mods.join(name), &dir)?;
    let _ = std::fs::remove_file(dir.join(crate::retired::MARKER));
    let manifest = dir.join(crate::MANIFEST_FILE);
    let parsed = std::fs::read_to_string(&manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    if let Some(serde_json::Value::Object(mut fields)) = parsed {
        fields.insert("cloned_from".to_owned(), name.into());
        let text = serde_json::to_string_pretty(&fields).map_err(io::Error::other)?;
        std::fs::write(&manifest, text + "\n")?;
    }
    Ok(to)
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
