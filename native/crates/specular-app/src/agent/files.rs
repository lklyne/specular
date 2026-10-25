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
