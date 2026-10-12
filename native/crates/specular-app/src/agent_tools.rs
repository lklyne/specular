//! What an agent drives the app with, put where an agent finds it: the
//! `specular` command on the `PATH` and the `specular` skill in Claude
//! Code's skills folder.
//!
//! Both come from the app bundle. The canvas agent's prompt names the
//! command and the skill, so a bundle installs them when it starts and
//! nobody has to ask.

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::prefs;

/// The command's wrapper script, from the executable in `Contents/MacOS`.
/// The Electron app kept its wrapper at the same place in its bundle, so a
/// link it made still points at one.
const WRAPPER: &str = "../Resources/specular-cli.sh";
/// The skill's folder, from the executable in `Contents/MacOS`.
const SKILL: &str = "../Resources/skills/specular";
const SKILL_FILE: &str = "SKILL.md";
const COMMAND: &str = "specular";

/// Installs the command and the skill from the bundle this executable is
/// in. Does nothing in a build that is not a bundle, in a run given a
/// config folder of its own, and in an app opened from its disk image or
/// from the folder macOS runs a quarantined app in: a link to either would
/// stop working when it goes.
pub fn install() {
    if std::env::var_os(prefs::CONFIG_DIR_VARIABLE).is_some_and(|folder| !folder.is_empty()) {
        return;
    }
    let Some(macos) = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.to_owned()))
    else {
        return;
    };
    if macos.starts_with("/Volumes") || macos.iter().any(|part| part == "AppTranslocation") {
        return;
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let wrapper = macos.join(WRAPPER);
    if let Ok(wrapper) = wrapper.canonicalize() {
        let links = [
            PathBuf::from("/usr/local/bin").join(COMMAND),
            home.join(".local/bin").join(COMMAND),
        ];
        if let Some(link) = link_command(&wrapper, &links) {
            tracing::info!(link = %link.display(), "the specular command is linked");
        } else {
            tracing::info!(
                "the specular command was not linked: another one is on the PATH, or no folder took the link"
            );
        }
    }
    let skill = macos.join(SKILL);
    if let (true, Some(preferences)) = (skill.join(SKILL_FILE).is_file(), prefs::file()) {
        let recorded = prefs::load_skill_hash(&preferences, COMMAND);
        match sync_skill(&skill, &home.join(".claude"), recorded.as_deref()) {
            Ok(Some(hash)) if recorded.as_deref() != Some(&hash) => {
                if let Err(error) = prefs::save_skill_hash(&preferences, COMMAND, &hash) {
                    tracing::warn!("the skill's hash was not recorded: {error}");
                }
            }
            Ok(_) => {}
            Err(error) => tracing::warn!("the specular skill was not installed: {error}"),
        }
    }
}

/// Makes the first of `links` that can be made point at `wrapper`, and
/// returns the link that does. A link or file that someone else put there
/// and that still works is theirs and stays; a link to a file that is gone
/// is replaced.
fn link_command(wrapper: &Path, links: &[PathBuf]) -> Option<PathBuf> {
    for link in links {
        match std::fs::read_link(link) {
            Ok(target) if target == wrapper => return Some(link.clone()),
            Ok(_) if link.exists() => return None,
            Ok(_) => {
                if std::fs::remove_file(link).is_err() {
                    continue;
                }
            }
            Err(_) if link.symlink_metadata().is_ok() => return None,
            Err(_) => {}
        }
        if let Some(folder) = link.parent() {
            // Only ever makes the folder under the home: the system one
            // needs a password this does not ask for.
            let _ = std::fs::create_dir_all(folder);
        }
        if std::os::unix::fs::symlink(wrapper, link).is_ok() {
            return Some(link.clone());
        }
    }
    None
}

/// Copies the skill in `source` into `claude`'s skills folder when it is
/// missing there, or when the one there is the one this app wrote last
/// (`recorded`). An installed skill that is neither was edited by hand and
/// is left alone. Returns the hash to record: the one now installed by us.
fn sync_skill(source: &Path, claude: &Path, recorded: Option<&str>) -> io::Result<Option<String>> {
    if !claude.is_dir() {
        return Ok(None);
    }
    let bundled = hash(&source.join(SKILL_FILE))?;
    let installed_dir = claude.join("skills").join(COMMAND);
    let installed = match hash(&installed_dir.join(SKILL_FILE)) {
        Ok(installed) => Some(installed),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    if installed.as_deref() == Some(&bundled) {
        return Ok(Some(bundled));
    }
    if installed.is_some() && installed.as_deref() != recorded {
        tracing::info!("the installed specular skill was edited; leaving it as it is");
        return Ok(None);
    }
    copy_folder(source, &installed_dir)?;
    Ok(Some(bundled))
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
        assert_eq!(link_command(&wrapper, &links), Some(links[1].clone()));
        assert_eq!(std::fs::read_link(&links[1]).unwrap(), wrapper);
        // A second launch finds it there and changes nothing.
        assert_eq!(link_command(&wrapper, &links), Some(links[1].clone()));
    }

    #[test]
    fn a_link_to_a_file_that_is_gone_is_replaced() {
        let dir = TempDir::new("dangling");
        let wrapper = dir.file("app/specular-cli.sh", "");
        let link = dir.0.join("bin/specular");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(dir.0.join("old-app/specular-cli.sh"), &link).unwrap();
        assert_eq!(
            link_command(&wrapper, std::slice::from_ref(&link)),
            Some(link.clone())
        );
        assert_eq!(std::fs::read_link(&link).unwrap(), wrapper);
    }

    #[test]
    fn a_working_link_or_a_file_someone_else_made_stays() {
        let dir = TempDir::new("theirs");
        let wrapper = dir.file("app/specular-cli.sh", "");
        let theirs = dir.file("repo/specular-cli.sh", "");
        let link = dir.0.join("bin/specular");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&theirs, &link).unwrap();
        let later = dir.0.join("later/specular");
        assert_eq!(link_command(&wrapper, &[link.clone(), later.clone()]), None);
        assert_eq!(std::fs::read_link(&link).unwrap(), theirs);
        assert!(!later.exists(), "a second command is not put behind theirs");

        let file = dir.file("other/specular", "#!/bin/sh");
        assert_eq!(link_command(&wrapper, std::slice::from_ref(&file)), None);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "#!/bin/sh");
    }

    #[test]
    fn a_missing_skill_is_installed_with_its_references() {
        let dir = TempDir::new("missing");
        dir.file("skill/SKILL.md", "one");
        dir.file("skill/references/apply.md", "apply");
        std::fs::create_dir_all(dir.0.join("claude")).unwrap();
        let hash = sync_skill(&dir.0.join("skill"), &dir.0.join("claude"), None).unwrap();
        assert!(hash.is_some());
        let installed = dir.0.join("claude/skills/specular");
        assert_eq!(
            std::fs::read_to_string(installed.join("SKILL.md")).unwrap(),
            "one"
        );
        assert_eq!(
            std::fs::read_to_string(installed.join("references/apply.md")).unwrap(),
            "apply"
        );
    }

    #[test]
    fn nothing_is_installed_for_someone_without_claude_code() {
        let dir = TempDir::new("no-claude");
        dir.file("skill/SKILL.md", "one");
        let claude = dir.0.join("claude");
        assert_eq!(
            sync_skill(&dir.0.join("skill"), &claude, None).unwrap(),
            None
        );
        assert!(!claude.exists());
    }

    #[test]
    fn the_skill_we_wrote_is_updated_and_an_edited_one_is_left() {
        let dir = TempDir::new("update");
        let source = dir.0.join("skill");
        let claude = dir.0.join("claude");
        dir.file("skill/SKILL.md", "one");
        std::fs::create_dir_all(&claude).unwrap();
        let first = sync_skill(&source, &claude, None).unwrap();

        dir.file("skill/SKILL.md", "two");
        let second = sync_skill(&source, &claude, first.as_deref()).unwrap();
        assert_ne!(second, first);
        let installed = claude.join("skills/specular/SKILL.md");
        assert_eq!(std::fs::read_to_string(&installed).unwrap(), "two");

        std::fs::write(&installed, "mine").unwrap();
        dir.file("skill/SKILL.md", "three");
        assert_eq!(
            sync_skill(&source, &claude, second.as_deref()).unwrap(),
            None
        );
        assert_eq!(std::fs::read_to_string(&installed).unwrap(), "mine");
    }
}
