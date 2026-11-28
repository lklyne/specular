//! `mod clone <name> [new-name] [--use]`: a copy of a mod in the app's mods
//! folder that the person owns. Its `mod.json` says `cloned_from`, so the
//! app never writes over it on start. With `--use` the layout file names
//! the copy wherever it named the original.
//!
//! The copy is plain file work in the app's home folder, so it needs no
//! running app: the folder is found as the app finds it
//! (`SPECULAR_NATIVE_CONFIG_DIR`, else Application Support on macOS, else
//! the XDG config folder).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::args::Args;
use crate::{output, refuse, usage};

const USAGE: &str = "specular mod clone <name> [new-name] [--use]";
/// The person's own layout, which the app never writes over.
const MINE: &str = "mine";

/// Files a mod's runtime writes into its folder on load, which a copy
/// gets again when it loads.
const WRITTEN_ON_LOAD: [&str; 2] = ["jsconfig.json", "gpui-kit.d.ts"];

/// Whether `args` is `mod clone`, which needs no running app.
pub(crate) fn is_clone(args: &Args) -> bool {
    args.verb == "mod" && args.nth(0) == Some("clone")
}

/// The app's own data folder, which holds `mods/` and `layouts/`, and
/// whether it was named by `SPECULAR_NATIVE_CONFIG_DIR`.
fn app_home() -> Option<(PathBuf, bool)> {
    let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if let Some(folder) = set("SPECULAR_NATIVE_CONFIG_DIR") {
        return Some((PathBuf::from(folder), true));
    }
    standard_home().map(|home| (home, false))
}

/// Where the app keeps its data when nothing says otherwise.
fn standard_home() -> Option<PathBuf> {
    let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    let home = set("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return Some(home?.join("Library/Application Support/Specular"));
    }
    let config = set("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".config")));
    Some(config?.join("Specular"))
}

/// What a clone made.
#[derive(Debug)]
struct Cloned {
    /// The new mod's folder.
    dir: PathBuf,
    /// The layout file, when `--use` pointed it at the copy, and the
    /// regions it now names the copy for.
    layout: Option<(PathBuf, Vec<String>)>,
}

/// Whether `name` can be a mod's folder name.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Copies the folder `from` into `to`, which must not exist, leaving out
/// what the runtime writes on load.
fn copy_folder(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir(to).map_err(|error| format!("creating {}: {error}", to.display()))?;
    let entries =
        std::fs::read_dir(from).map_err(|error| format!("reading {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if WRITTEN_ON_LOAD.iter().any(|written| name == *written) {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        if source.is_dir() {
            copy_folder(&source, &target)?;
        } else {
            std::fs::copy(&source, &target)
                .map_err(|error| format!("copying {}: {error}", source.display()))?;
        }
    }
    Ok(())
}

/// Reads a JSON object from `path`.
fn read_object(path: &Path) -> Result<Map<String, Value>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(format!("{} is not a JSON object", path.display())),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

/// Writes `object` to `path` whole, as the app writes its files.
fn write_object(path: &Path, object: Map<String, Value>) -> Result<(), String> {
    let mut text =
        serde_json::to_string_pretty(&Value::Object(object)).map_err(|error| error.to_string())?;
    text.push('\n');
    let staged = path.with_extension("writing");
    std::fs::write(&staged, text)
        .and_then(|()| std::fs::rename(&staged, path))
        .map_err(|error| format!("writing {}: {error}", path.display()))
}

/// Points `layout` at `to` wherever it names `from`: a block of its tree
/// whose content is the mod, or an overlay at its top level. When it names
/// `from` nowhere, the block or overlay `place` gets `to`. Returns the
/// blocks and overlays it now names `to` for.
fn point_layout(
    layout: &Path,
    from: &str,
    to: &str,
    place: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut file = read_object(layout)?;
    let mut named = Vec::new();
    for (key, node) in &mut file {
        if let Some(overlay) = node.as_object_mut()
            && names_mod(overlay, from)
        {
            overlay.insert("mod".to_owned(), Value::String(to.to_owned()));
            named.push(key.clone());
        }
    }
    rename_in_tree(&mut file, from, to, &mut named);
    if named.is_empty() {
        let Some(place) = place else {
            return Err(format!(
                "the layout does not name `{from}` and its mod.json has no `place`; set a block's \"content\" to {{\"mod\": \"{to}\"}} in {}",
                layout.display()
            ));
        };
        if !fill_block(&mut file, place, to) {
            match file.get_mut(place).and_then(Value::as_object_mut) {
                Some(overlay) => {
                    overlay.insert("mod".to_owned(), Value::String(to.to_owned()));
                }
                None => {
                    return Err(format!(
                        "the layout has no block or overlay `{place}` for `{to}`; set a block's \"content\" to {{\"mod\": \"{to}\"}} in {}",
                        layout.display()
                    ));
                }
            }
        }
        named.push(place.to_owned());
    }
    write_object(layout, file)?;
    Ok(named)
}

/// Whether `object` (a block's content or an overlay) is the mod `name`.
fn names_mod(object: &Map<String, Value>, name: &str) -> bool {
    object.get("mod").and_then(Value::as_str) == Some(name)
}

/// The children of a tree node, if it has any.
fn children(node: &mut Map<String, Value>) -> impl Iterator<Item = &mut Map<String, Value>> {
    (node.get_mut("children").and_then(Value::as_array_mut))
        .into_iter()
        .flatten()
        .filter_map(Value::as_object_mut)
}

/// Names `to` in every block under `node` whose content is `from`, and
/// adds each block's id to `named`.
fn rename_in_tree(node: &mut Map<String, Value>, from: &str, to: &str, named: &mut Vec<String>) {
    let id = node.get("id").and_then(Value::as_str).map(str::to_owned);
    if let Some(content) = node.get_mut("content").and_then(Value::as_object_mut)
        && names_mod(content, from)
    {
        content.insert("mod".to_owned(), Value::String(to.to_owned()));
        named.push(id.unwrap_or_else(|| from.to_owned()));
    }
    for child in children(node) {
        rename_in_tree(child, from, to, named);
    }
}

/// Makes the block `id` under `node` show the mod `to`, keeping its props.
/// Returns whether there is such a block.
fn fill_block(node: &mut Map<String, Value>, id: &str, to: &str) -> bool {
    if node.get("id").and_then(Value::as_str) == Some(id)
        && node.get("type").and_then(Value::as_str) == Some("block")
    {
        let props = (node.get("content").and_then(Value::as_object))
            .and_then(|content| content.get("props"))
            .cloned();
        let mut content = Map::new();
        content.insert("mod".to_owned(), Value::String(to.to_owned()));
        if let Some(props) = props {
            content.insert("props".to_owned(), props);
        }
        node.insert("content".to_owned(), Value::Object(content));
        return true;
    }
    children(node).any(|child| fill_block(child, id, to))
}

/// Copies the mod `from` in `home`'s mods folder to `to`, marks the copy
/// as cloned from `from`, and with `use_it` points the layout at it.
fn clone_mod(home: &Path, from: &str, to: &str, use_it: bool) -> Result<Cloned, String> {
    if !plain_name(to) {
        return Err(format!(
            "`{to}` is not a mod name: use letters, digits, - and _"
        ));
    }
    let mods = home.join("mods");
    let source = mods.join(from);
    if !plain_name(from) || !source.join("mod.json").is_file() {
        return Err(format!(
            "no mod `{from}` in {}; start Specular once to write the shipped mods",
            mods.display()
        ));
    }
    let dir = mods.join(to);
    if dir.exists() {
        return Err(format!("{} is there already", dir.display()));
    }
    copy_folder(&source, &dir)?;
    let manifest_path = dir.join("mod.json");
    let mut manifest = read_object(&manifest_path)?;
    let place = manifest
        .get("place")
        .and_then(Value::as_str)
        .map(str::to_owned);
    manifest.insert("cloned_from".to_owned(), Value::String(from.to_owned()));
    write_object(&manifest_path, manifest)?;
    let layout = if use_it {
        let path = own_layout(home)?;
        let named = point_layout(&path, from, to, place.as_deref())?;
        Some((path, named))
    } else {
        None
    };
    Ok(Cloned { dir, layout })
}

/// The person's own layout, `layouts/mine.json`, started from the shipped
/// one when there is none yet. The app rewrites `default.json` on every
/// start and never touches `mine.json`, so a layout pointed at a clone has
/// to be this one.
fn own_layout(home: &std::path::Path) -> Result<PathBuf, String> {
    let folder = home.join("layouts");
    let mine = folder.join(format!("{MINE}.json"));
    if !mine.exists() {
        std::fs::copy(folder.join("default.json"), &mine).map_err(|error| {
            format!(
                "{} not started from the shipped layout: {error}",
                mine.display()
            )
        })?;
    }
    Ok(mine)
}

/// Asks a running app to show `mine`, and says what came of it. A home
/// named by `SPECULAR_NATIVE_CONFIG_DIR` may not be the running app's, so
/// that app is left alone.
fn show_mine(named_home: bool) -> String {
    let how = format!("`specular run layout.switch '{{\"name\":\"{MINE}\"}}'` shows it");
    if named_home {
        return format!("in the app that uses this folder, {how}");
    }
    let client = crate::client::Client::new(None);
    let body = serde_json::json!({ "name": "layout.switch", "args": { "name": MINE } });
    match client.post("/run", &body) {
        Ok(_) => format!("the app now shows the `{MINE}` layout"),
        Err(_) => format!("the app is not running; once it is, {how}"),
    }
}

/// `mod clone <name> [new-name] [--use]`.
pub(crate) fn clone(args: &Args) -> anyhow::Result<i32> {
    let Some(from) = args.nth(1) else {
        return usage(USAGE);
    };
    let to = args
        .nth(2)
        .map_or_else(|| format!("my-{from}"), str::to_owned);
    let Some((home, named)) = app_home() else {
        return refuse("no home folder to find the app's mods in; set SPECULAR_NATIVE_CONFIG_DIR");
    };
    match clone_mod(&home, from, &to, args.switch("use")) {
        Ok(cloned) => {
            output::text_line(&format!("{}", cloned.dir.display()));
            if let Some((layout, regions)) = cloned.layout {
                output::text_line(&format!(
                    "{} now names `{to}` for {}",
                    layout.display(),
                    regions.join(", ")
                ));
                output::text_line(&show_mine(named));
            }
            Ok(0)
        }
        Err(reason) => refuse(&reason),
    }
}
