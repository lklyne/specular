//! Repo bindings: linking a page's origin to a folder, the write target that
//! follows, and the chips that show it.

use specular_core::{CssSize, synthetic_element_at};
use specular_doc::{Document, Kind, Rect};
use specular_interact::{Action, Effect, RepoAction, RunRequest, Tool, repos_pane};
use specular_testkit::insta::assert_snapshot;
use specular_testkit::{TestApp, document, pages};

const ORIGIN: &str = "https://example.com";
const REPO: &str = "/scratch/site";

fn repo_lines(app: &TestApp) -> String {
    (app.popup_snapshot().lines())
        .map(str::trim_start)
        .filter(|line| line.contains("page.repo"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_page_is_linked_to_a_repo_through_its_popup_and_the_pane_lists_it() {
    let mut app = TestApp::with_pages(1);
    app.with_panels();
    app.select(&["p1"]);
    assert_snapshot!(repo_lines(&app), @r#"
    dropdown page.repo "Link a repo" shows icon=Repo
    option [ ] page.repo.origin "https://example.com" disabled -> Repo(Pick(Some("https://example.com")))
    option [ ] page.repo.folder "No repo linked" trailing="Choose…" -> Repo(Pick(Some("https://example.com")))
    "#);

    app.click_control("page.repo").take_effects();
    app.click_control("page.repo.folder");
    assert_eq!(
        app.take_effects(),
        [Effect::PickRepoFolder {
            origin: Some(ORIGIN.to_owned())
        }]
    );

    app.act(Action::Repo(RepoAction::Bind {
        origin: ORIGIN.to_owned(),
        path: "/Users/ada/dev/site".to_owned(),
    }));
    assert_eq!(app.take_effects(), [Effect::SaveRepos]);
    let pane = repos_pane(app.app());
    let rows: Vec<_> = (pane.repos.iter())
        .map(|row| (row.label.as_str(), row.path.as_str(), row.origins.len()))
        .collect();
    assert_eq!(rows, [("site", "/Users/ada/dev/site", 1)]);
    assert_snapshot!(repo_lines(&app), @r#"
    dropdown page.repo "Repo: ~/dev/site" shows icon=Repo
    option [ ] page.repo.origin "https://example.com" disabled -> Repo(Pick(Some("https://example.com")))
    option [ ] page.repo.folder "~/dev/site" trailing="Change…" -> Repo(Pick(Some("https://example.com")))
    option [ ] page.repo.unlink "Unlink" -> Repo(Unlink("https://example.com"))
    "#);

    // The pane's switch turns auto-fix on, and then offers to turn it off.
    app.act(pane.repos[0].origins[0].toggle_auto_fix.clone());
    assert_eq!(app.take_effects(), [Effect::SaveRepos]);
    let row = &repos_pane(app.app()).repos[0].origins[0];
    assert!(row.auto_fix);
    assert_eq!(
        row.toggle_auto_fix,
        Action::Repo(RepoAction::SetAutoFix {
            origin: ORIGIN.to_owned(),
            on: false
        })
    );

    app.act(pane.repos[0].origins[0].remove.clone());
    assert_eq!(app.take_effects(), [Effect::SaveRepos]);
    app.act(Action::Repo(RepoAction::Unlink(ORIGIN.to_owned())));
    assert_eq!(app.take_effects(), [], "nothing changed, nothing saved");
    assert_eq!(repos_pane(app.app()).repos[0].origins.len(), 0);
}

#[test]
fn a_page_with_no_origin_has_no_repo_control() {
    let mut page = specular_testkit::page("d", Rect::new(100.0, 100.0, 400.0, 300.0));
    if let Kind::Page(inner) = &mut page.kind {
        inner.url = "data:text/html,hi".to_owned();
    }
    let mut app = TestApp::with_entities([page]);
    app.with_panels();
    app.select(&["d"]);
    assert_eq!(repo_lines(&app), "");
}

/// A bound pair of pages with a comment of each shape to place.
#[derive(Clone, Copy, Debug)]
enum Case {
    PageComment { bound: bool },
    CanvasComment,
    Selection,
    Nothing,
}

fn requests(effects: &[Effect]) -> Vec<RunRequest> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::RunAgent(request) => Some((**request).clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_write_target_decides_the_chips_the_folder_and_the_prompt() {
    // (case, folder chip, auto chip (origin, on), run cwd, first prompt line)
    let table = [
        (
            Case::PageComment { bound: true },
            "site",
            Some((ORIGIN, false)),
            Some(REPO),
            "Working directory (linked repo): /scratch/site",
        ),
        (
            Case::PageComment { bound: false },
            "space",
            None,
            None,
            "Working directory (space folder): /space",
        ),
        (
            Case::CanvasComment,
            "space",
            None,
            None,
            "Working directory (space folder): /space",
        ),
        (
            Case::Selection,
            "space",
            None,
            None,
            "Working directory (space folder): /space",
        ),
        (
            Case::Nothing,
            "space",
            None,
            None,
            "Working directory (space folder): /space",
        ),
    ];
    for (case, folder, auto, cwd, first_line) in table {
        let mut app = TestApp::with_space([("Home", Document::new())]);
        app.open(document(pages(1))).with_chat_panel();
        if matches!(case, Case::PageComment { bound: true } | Case::Selection) {
            app.bind(ORIGIN, REPO, false);
        }
        app.take_effects();
        match case {
            Case::PageComment { .. } => {
                app.tool(Tool::Comment).click((200.0, 200.0));
                app.answer_element(synthetic_element_at(
                    CssSize::new(400, 300),
                    (100.0, 100.0).into(),
                ));
                app.send_chat("fix it");
            }
            Case::CanvasComment => {
                app.tool(Tool::Comment)
                    .click((900.0, 700.0))
                    .send_chat("fix it");
            }
            Case::Selection => {
                app.select(&["p1"]).send_chat("fix it");
            }
            Case::Nothing => {
                app.send_chat("fix it");
            }
        }
        let composer = app.chat().composer;
        let seen = (
            composer.folder.as_deref(),
            composer.auto.as_ref().map(|a| (a.origin.as_str(), a.on)),
        );
        assert_eq!(seen, (Some(folder), auto), "{case:?}");
        // A comment waits for Send; typing into the composer sends at once.
        let mut effects = app.take_effects();
        app.send_chat("");
        effects.extend(app.take_effects());
        let [request] = requests(&effects).try_into().expect("one run");
        assert_eq!(request.cwd.as_deref(), cwd, "{case:?}");
        assert_eq!(request.prompt.lines().next(), Some(first_line), "{case:?}");
    }
}

#[test]
fn the_auto_chip_turns_auto_fix_on_and_off_for_its_origin() {
    let mut app = TestApp::with_space([("Home", Document::new())]);
    app.open(document(pages(1))).with_chat_panel();
    app.bind(ORIGIN, REPO, false);
    app.tool(Tool::Comment).click((200.0, 200.0));
    app.answer_element(synthetic_element_at(
        CssSize::new(400, 300),
        (100.0, 100.0).into(),
    ));
    app.send_chat("fix it").take_effects();
    let chip = app.chat().composer.auto.expect("a repo turn");
    assert!(!chip.on);
    app.act(chip.toggle);
    assert_eq!(app.take_effects(), [Effect::SaveRepos]);
    let chip = app.chat().composer.auto.expect("still a repo turn");
    assert!(chip.on);
    assert_eq!(
        chip.toggle,
        Action::Repo(RepoAction::SetAutoFix {
            origin: ORIGIN.to_owned(),
            on: false
        })
    );
    assert_eq!(
        app.app().repos().binding(ORIGIN).map(|b| b.auto_fix),
        Some(true)
    );
}
