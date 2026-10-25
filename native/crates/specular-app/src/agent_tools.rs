//! What an agent drives the app with, put where an agent finds it: the
//! `specular` command on the `PATH` and the `specular` skill in Claude
//! Code's skills folder.
//!
//! Both come from the app bundle and are installed when the user says so,
//! in the setup dialog. After that the skill follows the bundle on its own
//! for as long as nobody edits the installed copy.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::prefs;

/// The CLI, beside the executable in `Contents/MacOS`. The link on the
/// `PATH` is named `specular`; the binary is not, so it never collides
/// with the Kit shell's own `specular` in a build folder.
const WRAPPER: &str = "specular-cli";
/// The skill's folder, from the executable in `Contents/MacOS`.
const SKILL: &str = "../Resources/skills/specular";
const SKILL_FILE: &str = "SKILL.md";
const COMMAND: &str = "specular";

/// Where one of the two is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// This app's, in place. For the command, the link's path.
    Installed(Option<String>),
    /// Not there.
    Missing,
    /// There, and not this app's: a command that runs something else, or a
    /// skill that differs from the bundled one. Installing replaces it.
    Differs(String),
}

impl Status {
    /// Whether there is nothing to install.
    pub fn installed(&self) -> bool {
        matches!(self, Self::Installed(_))
    }
}

/// What the setup dialog shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    /// The `specular` command.
    pub command: Status,
    /// The `specular` skill.
    pub skill: Status,
    /// Whether there is a `~/.claude` for the skill to be read from.
    pub claude: bool,
}

/// The folders an install reads and writes.
struct Places {
    wrapper: PathBuf,
    skill: PathBuf,
    /// Where the command is linked, in the order tried.
    links: [PathBuf; 2],
    claude: PathBuf,
    preferences: PathBuf,
}

impl Places {
    /// The places for the bundle this executable is in. `Err` says why
    /// nothing can be installed from here.
    fn find() -> Result<Self, &'static str> {
        const NO_BUNDLE: &str = "This build has no command or skill to install.";
        if std::env::var_os(prefs::CONFIG_DIR_VARIABLE).is_some_and(|folder| !folder.is_empty()) {
            return Err("This run has a data folder of its own and installs nothing.");
        }
        let exe = std::env::current_exe().map_err(|_| NO_BUNDLE)?;
        let macos = exe.parent().ok_or(NO_BUNDLE)?;
        // A link to either would stop working when it goes.
        if macos.starts_with("/Volumes") || macos.iter().any(|part| part == "AppTranslocation") {
            return Err("Move Specular to the Applications folder first, then open it from there.");
        }
        let wrapper = macos.join(WRAPPER).canonicalize().map_err(|_| NO_BUNDLE)?;
        let skill = macos.join(SKILL);
        if !skill.join(SKILL_FILE).is_file() {
            return Err(NO_BUNDLE);
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(NO_BUNDLE)?;
        Ok(Self {
            wrapper,
            skill,
            links: [
                PathBuf::from("/usr/local/bin").join(COMMAND),
                home.join(".local/bin").join(COMMAND),
            ],
            claude: home.join(".claude"),
            preferences: prefs::file().ok_or(NO_BUNDLE)?,
        })
    }

    fn installed_skill(&self) -> PathBuf {
        self.claude.join("skills").join(COMMAND)
    }

    fn setup(&self) -> Setup {
        Setup {
            command: command_status(&self.wrapper, &self.links),
            skill: skill_status(&self.skill, &self.installed_skill()),
            claude: self.claude.is_dir(),
        }
    }
}

/// Where the command and the skill are now, or why this build installs
/// neither.
pub fn setup() -> Result<Setup, &'static str> {
    Ok(Places::find()?.setup())
}

/// Whether to show the setup dialog without being asked: a bundle that can
/// install, something to install, and a user who has not been through it.
pub fn wants_setup() -> bool {
    let Ok(places) = Places::find() else {
        return false;
    };
    if prefs::load_setup_done(&places.preferences) {
        return false;
    }
    let setup = places.setup();
    !(setup.command.installed() && setup.skill.installed())
}

/// Records that the user has been through setup, installing or not.
pub fn finish_setup() {
    let Ok(places) = Places::find() else {
        return;
    };
    if let Err(error) = prefs::save_setup_done(&places.preferences) {
        tracing::warn!("setup was not recorded as done: {error}");
    }
}

/// Installs what is asked for, replacing a command or a skill that is not
/// this app's. Returns what went wrong, in words for the dialog.
pub fn install(command: bool, skill: bool) -> Result<(), String> {
    let places = Places::find().map_err(str::to_owned)?;
    if command {
        link_command(&places.wrapper, &places.links)
            .map_err(|error| format!("The command was not installed: {error}"))?;
    }
    if skill {
        copy_folder(&places.skill, &places.installed_skill())
            .and_then(|()| record_skill(&places))
            .map_err(|error| format!("The skill was not installed: {error}"))?;
    }
    Ok(())
}

/// Brings an installed skill up to the bundled one when the installed copy
/// is the one this app wrote last. A skill that was never installed, or
/// was edited since, is left as it is.
pub fn update_skill() {
    let Ok(places) = Places::find() else {
        return;
    };
    let recorded = prefs::load_skill_hash(&places.preferences, COMMAND);
    let installed = hash(&places.installed_skill().join(SKILL_FILE)).ok();
    let bundled = hash(&places.skill.join(SKILL_FILE)).ok();
    if installed.is_none() || installed != recorded || installed == bundled {
        return;
    }
    match copy_folder(&places.skill, &places.installed_skill()).and_then(|()| record_skill(&places))
    {
        Ok(()) => tracing::info!("the specular skill was brought up to date"),
        Err(error) => tracing::warn!("the specular skill was not updated: {error}"),
    }
}

fn record_skill(places: &Places) -> io::Result<()> {
    let bundled = hash(&places.skill.join(SKILL_FILE))?;
    prefs::save_skill_hash(&places.preferences, COMMAND, &bundled)
}

fn command_status(wrapper: &Path, links: &[PathBuf]) -> Status {
    let mut other = None;
    for link in links {
        match std::fs::read_link(link) {
            Ok(target) if target == wrapper => {
                return Status::Installed(Some(link.display().to_string()));
            }
            Ok(target) if link.exists() => {
                other.get_or_insert(format!("{} runs {}", link.display(), target.display()));
            }
            Err(_) if link.symlink_metadata().is_ok() => {
                other.get_or_insert(format!("{} is another program", link.display()));
            }
            // Nothing there, or a link to a file that is gone, which runs
            // nothing.
            Ok(_) | Err(_) => {}
        }
    }
    other.map_or(Status::Missing, Status::Differs)
}

fn skill_status(bundled: &Path, installed: &Path) -> Status {
    match (
        hash(&installed.join(SKILL_FILE)),
        hash(&bundled.join(SKILL_FILE)),
    ) {
        (Err(_), _) => Status::Missing,
        (Ok(installed), Ok(bundled)) if installed == bundled => Status::Installed(None),
        _ => Status::Differs("The installed skill differs from this version's".to_owned()),
    }
}

/// Makes the first of `links` that can be made point at `wrapper`. A link
/// already there is replaced, wherever it points. A file that is not a
/// link is someone's program and is not touched.
fn link_command(wrapper: &Path, links: &[PathBuf]) -> io::Result<PathBuf> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "nowhere to link it");
    for link in links {
        match std::fs::read_link(link) {
            Ok(target) if target == wrapper => return Ok(link.clone()),
            Ok(_) => {
                if let Err(error) = std::fs::remove_file(link) {
                    last = error;
                    continue;
                }
            }
            Err(_) if link.symlink_metadata().is_ok() => {
                last = io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} is another program", link.display()),
                );
                continue;
            }
            Err(_) => {}
        }
        if let Some(folder) = link.parent() {
            // Only ever makes the folder under the home: the system one
            // needs a password this does not ask for.
            let _ = std::fs::create_dir_all(folder);
        }
        match std::os::unix::fs::symlink(wrapper, link) {
            Ok(()) => return Ok(link.clone()),
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn hash(file: &Path) -> io::Result<String> {
    let digest = Sha256::digest(std::fs::read(file)?);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

fn copy_folder(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_folder(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "specular-agent-tools-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, path: &str, text: &str) -> PathBuf {
            let file = self.0.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, text).unwrap();
            file
        }

        fn link(&self, path: &str, target: &Path) -> PathBuf {
            let link = self.0.join(path);
            std::fs::create_dir_all(link.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(target, &link).unwrap();
            link
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_command_is_linked_at_the_first_place_that_takes_it() {
        let dir = TempDir::new("first");
        let wrapper = dir.file("app/specular-cli.sh", "");
        // A file where the folder should be: nothing can be linked in it.
        dir.file("system", "");
        let links = [
            dir.0.join("system/specular"),
            dir.0.join("home/bin/specular"),
        ];
        assert_eq!(command_status(&wrapper, &links), Status::Missing);
        assert_eq!(link_command(&wrapper, &links).unwrap(), links[1]);
        assert_eq!(std::fs::read_link(&links[1]).unwrap(), wrapper);
        assert_eq!(
            command_status(&wrapper, &links),
            Status::Installed(Some(links[1].display().to_string()))
        );
    }

    #[test]
    fn a_link_to_a_file_that_is_gone_reads_as_missing_and_is_replaced() {
        let dir = TempDir::new("dangling");
        let wrapper = dir.file("app/specular-cli.sh", "");
        let link = dir.link("bin/specular", &dir.0.join("old-app/specular-cli.sh"));
        let links = std::slice::from_ref(&link);
        assert_eq!(command_status(&wrapper, links), Status::Missing);
        assert_eq!(link_command(&wrapper, links).unwrap(), link);
        assert_eq!(std::fs::read_link(&link).unwrap(), wrapper);
    }

    #[test]
    fn a_working_link_elsewhere_is_named_and_an_install_replaces_it() {
        let dir = TempDir::new("theirs");
        let wrapper = dir.file("app/specular-cli.sh", "");
        let theirs = dir.file("repo/specular-cli.sh", "");
        let link = dir.link("bin/specular", &theirs);
        let links = std::slice::from_ref(&link);
        assert!(matches!(
            command_status(&wrapper, links),
            Status::Differs(_)
        ));
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            theirs,
            "looking changes nothing"
        );
        assert_eq!(link_command(&wrapper, links).unwrap(), link);
        assert_eq!(std::fs::read_link(&link).unwrap(), wrapper);
    }

    #[test]
    fn a_program_that_is_not_a_link_is_never_replaced() {
        let dir = TempDir::new("program");
        let wrapper = dir.file("app/specular-cli.sh", "");
        let file = dir.file("bin/specular", "#!/bin/sh");
        let links = std::slice::from_ref(&file);
        assert!(matches!(
            command_status(&wrapper, links),
            Status::Differs(_)
        ));
        assert!(link_command(&wrapper, links).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "#!/bin/sh");
    }

    #[test]
    fn the_skill_is_missing_then_installed_then_differs_when_either_side_changes() {
        let dir = TempDir::new("skill");
        let bundled = dir.0.join("skill");
        let installed = dir.0.join("claude/skills/specular");
        dir.file("skill/SKILL.md", "one");
        dir.file("skill/references/apply.md", "apply");
        assert_eq!(skill_status(&bundled, &installed), Status::Missing);

        copy_folder(&bundled, &installed).unwrap();
        assert_eq!(skill_status(&bundled, &installed), Status::Installed(None));
        assert_eq!(
            std::fs::read_to_string(installed.join("references/apply.md")).unwrap(),
            "apply"
        );

        dir.file("skill/SKILL.md", "two");
        assert!(matches!(
            skill_status(&bundled, &installed),
            Status::Differs(_)
        ));
    }
}
