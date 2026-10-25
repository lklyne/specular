//! The space folder end to end, in a temp folder: what is found, what is
//! listed, and what each canvas operation leaves on disk.

mod finding;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specular_core::Camera;
use specular_doc::{EntityId, ItemId};
use specular_interact::{Action, App, CanvasAction, CanvasId, Effect, Event, update};

use super::files::SpaceFiles;
use super::{listing, open};

/// A fresh folder under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("specular-space-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir.canonicalize().unwrap())
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.0.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn read(&self, file: &str) -> String {
        std::fs::read_to_string(self.0.join(file)).unwrap()
    }

    fn json(&self, file: &str) -> Value {
        serde_json::from_str(&self.read(file)).unwrap()
    }

    /// The names of the `.canvas` files in the folder, sorted.
    fn canvases(&self) -> Vec<String> {
        listing::canvas_files(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A canvas file holding one sticky called `id`.
fn canvas(id: &str) -> String {
    format!(
        r#"{{"nodes":[{{"id":"{id}","type":"text","x":0,"y":0,"width":200,"height":200,"text":"{id}","specular":{{"textStyle":"sticky"}}}}],"edges":[]}}"#
    )
}

fn node_ids(text: &str) -> Vec<String> {
    let canvas: Value = serde_json::from_str(text).unwrap();
    (canvas["nodes"].as_array().unwrap().iter())
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect()
}

/// An app on a space folder, with the file effects run as the shell runs
/// them. A deleted canvas's file goes to `.trash` in the folder.
struct OnDisk {
    app: App,
    files: SpaceFiles,
}

fn fake_trash(path: &Path) -> Result<(), String> {
    let bin = path.parent().unwrap().join(".trash");
    std::fs::create_dir_all(&bin).map_err(|error| error.to_string())?;
    std::fs::rename(path, bin.join(path.file_name().unwrap())).map_err(|error| error.to_string())
}

impl OnDisk {
    fn open(folder: &Path) -> Self {
        let opened = open(folder, None, Camera::default()).unwrap();
        let mut on_disk = Self {
            app: App::new(7),
            files: SpaceFiles::new(folder).with_trash(fake_trash),
        };
        on_disk.send(Event::SpaceOpened(Box::new(opened)));
        on_disk.files.follow(on_disk.app.space());
        on_disk
    }

    fn send(&mut self, event: Event) {
        for effect in update(&mut self.app, event) {
            match effect {
                Effect::Save => {
                    let active = self.app.space().active().id.clone();
                    self.files.request_save(&active);
                }
                Effect::WriteCanvas(canvas) => self.files.write_now(&self.app, &canvas),
                Effect::RenameCanvasFile { canvas, from, to } => {
                    self.files.rename(&self.app, &canvas, &from, &to);
                }
                Effect::TrashCanvasFile { canvas, file } => self.files.trash(&canvas, &file),
                Effect::SaveSpaceMeta => self.files.save_meta(&self.app),
                _ => {}
            }
        }
    }

    fn act(&mut self, action: Action) {
        self.send(Event::Action(action));
    }

    fn id(&self, name: &str) -> CanvasId {
        self.app.space().resolve(name).unwrap().id.clone()
    }

    fn names(&self) -> Vec<&str> {
        (self.app.space().canvases().iter())
            .map(|canvas| canvas.name.as_str())
            .collect()
    }

    fn delete_entity(&mut self, id: &str) {
        self.act(Action::Select(vec![ItemId::Entity(EntityId::from(id))]));
        self.act(Action::Delete);
    }
}

#[test]
fn writing_the_index_keeps_what_this_app_does_not_use() {
    let dir = TempDir::new("meta");
    dir.write(
        ".specular/workspace-meta.json",
        r#"{"activeTabId":"tab_aaaa1111","viewMode":"browser","tabs":[
            {"id":"tab_aaaa1111","name":"Home","updatedAt":"2026-01-01T00:00:00.000Z","expanded":false}]}"#,
    );
    dir.write("Home-aaaa.canvas", &canvas("a"));
    let mut space = OnDisk::open(&dir.0);
    space.act(Action::Canvas(CanvasAction::New));
    let meta = dir.json(".specular/workspace-meta.json");
    assert_eq!(meta["viewMode"], "browser");
    let made = space.id("Canvas 2");
    assert_eq!(meta["activeTabId"], made.as_str());
    assert_eq!(
        meta["tabs"][0],
        json!({ "id": "tab_aaaa1111", "name": "Home",
                "updatedAt": "2026-01-01T00:00:00.000Z", "expanded": false })
    );
    assert_eq!(meta["tabs"][1]["name"], "Canvas 2");
    assert_eq!(meta["tabs"][1]["expanded"], true);
}

#[test]
fn new_rename_duplicate_and_delete_are_each_on_disk() {
    let dir = TempDir::new("ops");
    dir.write("Home-aaaa.canvas", &canvas("a"));
    let mut space = OnDisk::open(&dir.0);
    let home = space.id("Home");

    space.act(Action::Canvas(CanvasAction::New));
    let made = space.app.space().active().file.clone();
    assert!(made.starts_with("Canvas 2-"), "{made}");
    assert_eq!(dir.canvases(), [made.as_str(), "Home-aaaa.canvas"]);
    assert_eq!(node_ids(&dir.read(&made)), [] as [&str; 0]);

    // A rename carries what was not saved yet.
    space.act(Action::Canvas(CanvasAction::Switch(home.clone())));
    space.delete_entity("a");
    assert_eq!(
        node_ids(&dir.read("Home-aaaa.canvas")),
        ["a"],
        "the autosave has not run"
    );
    space.act(Action::Canvas(CanvasAction::Rename {
        canvas: None,
        name: "Landing".to_owned(),
    }));
    assert!(!dir.0.join("Home-aaaa.canvas").exists());
    assert_eq!(node_ids(&dir.read("Landing-aaaa.canvas")), [] as [&str; 0]);

    space.act(Action::Undo);
    space.act(Action::Canvas(CanvasAction::Duplicate(None)));
    let copy = space.app.space().active().file.clone();
    assert_eq!(space.names(), ["Landing", "Landing Copy", "Canvas 2"]);
    assert_eq!(node_ids(&dir.read(&copy)), ["a"]);

    let copy_id = space.app.space().active().id.clone();
    space.act(Action::Canvas(CanvasAction::Delete(None)));
    assert!(!dir.0.join(&copy).exists());
    space.files.request_save(&copy_id);
    assert!(
        !space.files.has_unsaved(&copy_id),
        "a deleted canvas is no longer followed"
    );
    assert!(
        dir.0.join(".trash").join(&copy).is_file(),
        "it went to the trash"
    );
    assert_eq!(space.names(), ["Landing", "Canvas 2"]);
    assert_eq!(space.app.space().active().name, "Canvas 2");

    // The folder opens again as it was left.
    let again = OnDisk::open(&dir.0);
    assert_eq!(again.names(), ["Landing", "Canvas 2"]);
    assert_eq!(again.app.space().active().name, "Canvas 2");
    assert_eq!(again.id("Landing"), home);
}

#[test]
fn a_save_goes_to_the_canvas_that_changed_after_a_switch() {
    let dir = TempDir::new("save");
    dir.write("A-aaaa.canvas", &canvas("a"));
    dir.write("B-bbbb.canvas", &canvas("b"));
    let mut space = OnDisk::open(&dir.0);
    let b = space.id("B");
    space.delete_entity("a");
    space.act(Action::Canvas(CanvasAction::Switch(b)));
    space.files.flush(&space.app);
    assert_eq!(node_ids(&dir.read("A-aaaa.canvas")), [] as [&str; 0]);
    assert_eq!(node_ids(&dir.read("B-bbbb.canvas")), ["b"]);
}
