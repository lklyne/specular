use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use specular_agent::{Index, Thread, parse_index};

use crate::persist::write_atomic;

fn threads_dir(space: &Path) -> PathBuf {
    space.join(".specular").join("threads")
}

/// Ids come from files other tools may write; one that could leave the
/// threads folder is refused.
fn segment(id: &str) -> io::Result<&str> {
    if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("`{id}` cannot name a thread file"),
        ));
    }
    Ok(id)
}

/// Writes `<space>/.specular/threads/<tab id>/<id>.json`.
pub fn write_thread(space: &Path, thread: &Thread) -> io::Result<()> {
    let dir = threads_dir(space).join(segment(&thread.tab_id)?);
    let file = dir.join(format!("{}.json", segment(thread.id.as_str())?));
    fs::create_dir_all(&dir)?;
    write_atomic(&file, &thread.to_json())
}

/// Writes `<space>/.specular/threads/index.json`.
pub fn write_index(space: &Path, json: &str) -> io::Result<()> {
    let dir = threads_dir(space);
    fs::create_dir_all(&dir)?;
    write_atomic(&dir.join("index.json"), json)
}

/// Every thread under `.specular/threads/` and the index. A folder's name is
/// the tab id of a file that does not say. Unreadable or invalid files are
/// skipped; a missing folder is empty.
pub fn load(space: &Path, now: &str) -> (Vec<Thread>, Index) {
    let dir = threads_dir(space);
    let index = fs::read_to_string(dir.join("index.json"))
        .map(|text| parse_index(&text))
        .unwrap_or_default();
    let mut threads = Vec::new();
    let Ok(tabs) = fs::read_dir(&dir) else {
        return (threads, index);
    };
    let mut tabs: Vec<_> = tabs.flatten().filter(|e| e.path().is_dir()).collect();
    tabs.sort_by_key(fs::DirEntry::file_name);
    for tab in tabs {
        let tab_id = tab.file_name().to_string_lossy().into_owned();
        let Ok(files) = fs::read_dir(tab.path()) else {
            continue;
        };
        let mut files: Vec<_> = files.flatten().map(|e| e.path()).collect();
        files.sort();
        for path in files {
            let is_json = path.extension().is_some_and(|ext| ext == "json");
            if !is_json || path.file_name().is_some_and(|name| name == "index.json") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            threads.extend(Thread::from_json(&text, &tab_id, now));
        }
    }
    (threads, index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specular_agent::{Status, ThreadId};

    struct Space(PathBuf);

    impl Space {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-agent-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn thread(tab: &str, id: &str) -> Thread {
        Thread {
            id: ThreadId(id.into()),
            tab_id: tab.into(),
            title: "Make it blue".into(),
            status: Status::Open,
            created_at: "2026-01-01T00:00:00.000Z".into(),
            updated_at: "2026-01-01T00:00:01.000Z".into(),
            claude_session_id: Some("s1".into()),
            annotation_ids: vec!["a1".into()],
            messages: Vec::new(),
        }
    }

    #[test]
    fn a_thread_lands_at_the_electron_path_and_loads_back_with_the_index() {
        let space = Space::new("roundtrip");
        let saved = thread("home", "t1");
        write_thread(&space.0, &saved).unwrap();
        write_index(
            &space.0,
            r#"{"activeThreadId":"t1","activeByCanvas":{"home":"t1"}}"#,
        )
        .unwrap();
        assert!(space.0.join(".specular/threads/home/t1.json").is_file());
        let (threads, index) = load(&space.0, "now");
        assert_eq!(threads, vec![saved]);
        assert_eq!(index.active, Some(ThreadId("t1".into())));
        assert_eq!(index.by_canvas["home"], ThreadId("t1".into()));
    }

    #[test]
    fn load_skips_garbage_reads_an_electron_file_and_a_missing_folder_is_empty() {
        let empty = Space::new("missing");
        assert_eq!(load(&empty.0, "now"), (Vec::new(), Index::default()));

        let space = Space::new("garbage");
        let tab = space.0.join(".specular/threads/tab9");
        fs::create_dir_all(&tab).unwrap();
        fs::write(tab.join("bad.json"), "{nope").unwrap();
        // A thread's JSON under another extension is not read.
        fs::write(
            tab.join("t3.txt"),
            r#"{"id":"t3","title":"Hi","status":"open","createdAt":"c","updatedAt":"u","messages":[]}"#,
        )
        .unwrap();
        fs::write(
            tab.join("t2.json"),
            r#"{"id":"t2","title":"Hi","status":"open","createdAt":"c","updatedAt":"u","messages":[]}"#,
        )
        .unwrap();
        let (threads, _) = load(&space.0, "now");
        assert_eq!(threads.len(), 1);
        assert_eq!(
            (threads[0].id.as_str(), threads[0].tab_id.as_str()),
            ("t2", "tab9")
        );
    }

    #[test]
    fn an_id_that_could_leave_the_folder_is_refused() {
        let space = Space::new("hostile");
        for (tab, id) in [
            ("../x", "t"),
            ("home", "a/b"),
            ("home", "..\\x"),
            ("", "t"),
            ("home", "a\\b"),
            ("home", ".."),
            ("..", "t"),
            ("home", ""),
        ] {
            assert!(
                write_thread(&space.0, &thread(tab, id)).is_err(),
                "{tab} {id}"
            );
        }
        assert!(!space.0.join(".specular/threads/home").exists());
    }
}
