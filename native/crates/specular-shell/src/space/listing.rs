//! Which canvases a space folder holds: the space's index
//! (`.specular/workspace-meta.json`, the Electron app's file and shape)
//! reconciled with the `.canvas` files that are really there.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use specular_interact::{CanvasId, Space, canvas_file_name, legacy_canvas_file_name};

use crate::persist::write_atomic;

const META_DIR: &str = ".specular";
const META_FILE: &str = "workspace-meta.json";
const CANVAS_EXTENSION: &str = ".canvas";

/// One canvas the folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Listed {
    pub(crate) id: CanvasId,
    pub(crate) name: String,
    /// The file's name inside the folder.
    pub(crate) file: String,
}

/// The canvases of a folder and the one last shown.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Listing {
    pub(crate) canvases: Vec<Listed>,
    pub(crate) active: Option<CanvasId>,
}

/// The names of the `.canvas` files directly inside `folder`, sorted.
pub(crate) fn canvas_files(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(CANVAS_EXTENSION) && !name.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// The canvases of `folder`: the ones its index lists, in the index's
/// order, each with the file that holds it, then every `.canvas` file the
/// index does not account for, by name. A listed canvas whose file is gone
/// is left out.
pub(crate) fn list(folder: &Path) -> Listing {
    let files = canvas_files(folder);
    let meta = read_meta(folder);
    let mut listing = Listing {
        active: (meta.get("activeTabId").and_then(Value::as_str)).map(CanvasId::new),
        ..Listing::default()
    };
    let mut referenced = HashSet::new();
    let tabs = meta.get("tabs").and_then(Value::as_array);
    for tab in tabs.into_iter().flatten() {
        let (Some(id), Some(name)) = (
            tab.get("id").and_then(Value::as_str),
            tab.get("name").and_then(Value::as_str),
        ) else {
            continue;
        };
        let id = CanvasId::new(id);
        // A file written before names carried part of the id is still the
        // canvas's file.
        let candidates = [canvas_file_name(name, &id), legacy_canvas_file_name(name)];
        let found = (candidates.iter())
            .find(|file| files.contains(file))
            .cloned();
        referenced.extend(candidates);
        if let Some(file) = found
            && !listing.canvases.iter().any(|listed| listed.file == file)
        {
            listing.canvases.push(Listed {
                id,
                name: name.to_owned(),
                file,
            });
        }
    }
    for file in files {
        if !referenced.contains(&file) {
            listing.canvases.push(adopted(file));
        }
    }
    listing
}

/// A canvas for a file the index does not list: one that arrived from
/// outside the app. Its id is made from the file's name, so it is the same
/// on every launch until an index is written. A name that ends in a hyphen
/// and four hex digits is read as `<name>-<start of id>`, so the id
/// resolves back to this file the way the Electron app resolves it.
fn adopted(file: String) -> Listed {
    let stem = file.strip_suffix(CANVAS_EXTENSION).unwrap_or(&file);
    let hash = format!("{:016x}", fnv1a(stem.as_bytes()));
    let suffixed = stem.rsplit_once('-').filter(|(name, suffix)| {
        !name.is_empty()
            && suffix.len() == 4
            && suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    });
    let (name, id) = match suffixed {
        Some((name, suffix)) => (name, format!("tab_{suffix}{}", &hash[4..])),
        None => (stem, format!("tab_{hash}")),
    };
    Listed {
        id: CanvasId::new(id),
        name: name.to_owned(),
        file,
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn meta_path(folder: &Path) -> PathBuf {
    folder.join(META_DIR).join(META_FILE)
}

/// The index as a JSON object. Empty when there is none or it cannot be
/// read. The file at the folder's root is where older versions kept it.
fn read_meta(folder: &Path) -> Map<String, Value> {
    let text = std::fs::read_to_string(meta_path(folder))
        .or_else(|_| std::fs::read_to_string(folder.join(META_FILE)));
    match text.map(|text| serde_json::from_str(&text)) {
        Ok(Ok(Value::Object(meta))) => meta,
        _ => Map::new(),
    }
}

/// Writes the index for `space`: the active canvas and every canvas's id
/// and name, in order. Keys this app does not use, and each tab's
/// `updatedAt` and `expanded`, are kept as the file has them. A tab that
/// is new to the index is stamped `now`.
pub(crate) fn write_meta(folder: &Path, space: &Space, now: &str) -> io::Result<()> {
    let mut meta = read_meta(folder);
    let old_tabs = meta.remove("tabs");
    let old = |id: &str, key: &str| {
        (old_tabs.as_ref().and_then(Value::as_array))
            .and_then(|tabs| tabs.iter().find(|tab| tab["id"].as_str() == Some(id)))
            .and_then(|tab| tab.get(key).cloned())
    };
    let tabs: Vec<Value> = (space.canvases().iter())
        .map(|canvas| {
            let id = canvas.id.as_str();
            json!({
                "id": id,
                "name": canvas.name,
                "updatedAt": old(id, "updatedAt").unwrap_or_else(|| json!(now)),
                "expanded": old(id, "expanded").unwrap_or(json!(true)),
            })
        })
        .collect();
    meta.insert("activeTabId".to_owned(), json!(space.active().id.as_str()));
    meta.insert("tabs".to_owned(), Value::Array(tabs));
    let text = serde_json::to_string_pretty(&Value::Object(meta))?;
    std::fs::create_dir_all(folder.join(META_DIR))?;
    write_atomic(&meta_path(folder), &text)
}
