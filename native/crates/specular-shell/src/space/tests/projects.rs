//! Several projects open at once (ADR 0062): a repo beside the space keeps
//! its canvases, its index and its git choice in its own folder.

use specular_interact::{
    Action, CanvasAction, Effect, Event, GitChoice, ProjectAction, TabRefError,
};

use super::super::open;
use super::{OnDisk, TempDir, canvas};

/// The space with one canvas, and a repo beside it with none, in folders
/// named for the test.
fn space_and_repo(test: &str) -> (TempDir, TempDir, OnDisk) {
    let space = TempDir::new(&format!("{test}-space"));
    space.write("Home-aaaa.canvas", &canvas("a"));
    let repo = TempDir::new(&format!("{test}-repo"));
    repo.write(".git/HEAD", "ref: refs/heads/main\n");
    let mut on_disk = OnDisk::open(&space.0);
    let opened = open(&repo.0, None, specular_core::Camera::default(), None).unwrap();
    on_disk.send(Event::ProjectOpened(Box::new(opened)));
    on_disk.files.follow(on_disk.app.space());
    (space, repo, on_disk)
}

#[test]
fn a_repo_keeps_its_canvases_index_and_git_choice_in_its_own_folder() {
    let (space, repo, mut on_disk) = space_and_repo("repo-folder");
    let projects = on_disk.app.space().projects().to_vec();
    assert_eq!(projects.len(), 2);
    let site = &projects[1];
    assert!(site.repo);
    assert_eq!(site.branch.as_deref(), Some("main"));
    assert!(
        site.canvases.ends_with(".specular/canvases"),
        "{}",
        site.canvases
    );

    // Its first canvas is written in the repo, and each index lists only
    // its own project's canvases.
    let in_repo = TempDir(repo.0.join(".specular/canvases"));
    assert_eq!(in_repo.canvases().len(), 1);
    assert_eq!(space.canvases(), ["Home-aaaa.canvas"]);
    let names = |dir: &TempDir| -> Vec<String> {
        let meta = dir.json(".specular/workspace-meta.json");
        (meta["tabs"].as_array().unwrap().iter())
            .map(|tab| tab["name"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(names(&in_repo), ["Canvas 1"]);

    // A name is unique in its project, so a ref to it may need the
    // project's name.
    on_disk.act(Action::Canvas(CanvasAction::New(Some(site.id.clone()))));
    let made = on_disk.app.space().active().id.clone();
    on_disk.act(Action::Canvas(CanvasAction::Rename {
        canvas: Some(made.clone()),
        name: "Home".into(),
    }));
    assert_eq!(on_disk.app.space().active().name, "Home");
    assert!(matches!(
        on_disk.app.space().resolve("Home"),
        Err(TabRefError::Ambiguous { .. })
    ));
    let by_project = format!("{}/Home", site.label);
    assert_eq!(on_disk.app.space().resolve(&by_project).unwrap().id, made);
    assert_eq!(names(&in_repo), ["Canvas 1", "Home"]);
    assert_eq!(names(&space), ["Home"]);
    let active = on_disk.app.space().active_project().unwrap();
    assert!(active.canvases.ends_with(".specular/canvases"));

    // The git choice is the person's, and only then written.
    assert!(!repo.0.join(".specular/.gitignore").exists());
    on_disk.act(Action::Project(ProjectAction::SetGit {
        project: site.id.clone(),
        choice: GitChoice::Ignore,
    }));
    assert_eq!(repo.read(".specular/.gitignore").lines().last(), Some("*"));

    // Closing it shows the space's canvas and leaves its files.
    on_disk.act(Action::Project(ProjectAction::Close(site.id.clone())));
    assert_eq!(on_disk.app.space().projects().len(), 1);
    assert_eq!(on_disk.app.space().active().name, "Home");
    assert_eq!(on_disk.names(), ["Home"]);
    assert_eq!(in_repo.canvases().len(), 2);
}

#[test]
fn switching_to_another_projects_canvas_says_so_first() {
    let (_space, _repo, mut on_disk) = space_and_repo("entered");
    let other = on_disk.app.space().canvases()[1].id.clone();
    let effects = specular_interact::update(
        &mut on_disk.app,
        Event::Action(Action::Canvas(CanvasAction::Switch(other))),
    );
    let entered = effects
        .iter()
        .position(|effect| *effect == Effect::ProjectEntered);
    assert_eq!(entered, Some(0), "{effects:?}");
}
