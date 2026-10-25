//! The space folder end to end, in a temp folder: what is found, what is
//! listed, and what each canvas operation leaves on disk.

mod finding;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use specular_core::Camera;
use specular_interact::{Action, App, CanvasAction, CanvasId, Effect, Event, update};

use super::files::SpaceFiles;
use super::open;

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
