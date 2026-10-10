//! The command palette model: it lists every bound action once, takes its
//! keys and enabled states from the menus and binding table, and its search
//! ranks as a person expects.

use std::borrow::Cow;

use specular_doc::{Rect, ShapeKind};
use specular_interact::{
    Action, BINDINGS, PaletteItem, PaletteRun, SHELL_KEYS, ShellCommand, ToolDefaultPatch,
    binding_of, palette, search,
};
use specular_testkit::{TestApp, text};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);

/// Bound actions the palette leaves out: Escape backs out of whatever is
/// happening, and a nudge needs its key's direction.
fn left_out(action: &Action) -> bool {
    matches!(action, Action::Cancel | Action::Nudge { .. })
}

fn item<'a>(items: &'a [PaletteItem], label: &str) -> &'a PaletteItem {
    (items.iter())
        .find(|item| item.label == label)
        .unwrap_or_else(|| unreachable!("no palette item {label:?}"))
}

fn plain(label: &'static str, enabled: bool) -> PaletteItem {
    PaletteItem {
        label: Cow::Borrowed(label),
        group: "Group",
        run: PaletteRun::Shell(ShellCommand::Save),
        chord: None,
        enabled,
        checked: None,
        keywords: &[],
    }
}

fn labels<'a>(items: &'a [PaletteItem], query: &str) -> Vec<&'a str> {
    (search(items, query).iter())
        .map(|found| &*items[found.item].label)
        .collect()
}

#[test]
fn every_bound_action_is_listed_unless_left_out() {
    let items = palette(TestApp::with_pages(1).app());
    for binding in BINDINGS {
        let listed = items
            .iter()
            .any(|item| item.run == PaletteRun::Action(binding.action.clone()));
        assert!(
            listed || left_out(&binding.action),
            "{:?} is bound but not in the palette",
            binding.action
        );
    }
    for (_, command) in SHELL_KEYS {
        let listed = items
            .iter()
            .any(|item| item.run == PaletteRun::Shell(*command));
        // The palette is open when it is picked.
        assert!(
            listed || *command == ShellCommand::CommandPalette,
            "{command:?}"
        );
    }
}

#[test]
fn no_two_items_run_the_same_thing() {
    let items = palette(TestApp::with_pages(2).app());
    for (index, item) in items.iter().enumerate() {
        assert!(
            items[..index].iter().all(|earlier| earlier.run != item.run),
            "{} is listed twice",
            item.label
        );
    }
}

#[test]
fn an_items_chord_is_the_binding_tables() {
    let items = palette(TestApp::with_pages(1).app());
    for item in &items {
        match &item.run {
            PaletteRun::Action(action) => {
                // The Page menu hides the bracket keys, which restack.
                if matches!(action, Action::PageBack | Action::PageForward) {
                    continue;
                }
                assert_eq!(
                    item.chord,
                    binding_of(action).map(|binding| binding.chord),
                    "{}",
                    item.label
                );
            }
            PaletteRun::Shell(command) => {
                let key = SHELL_KEYS.iter().find(|(_, bound)| bound == command);
                assert_eq!(item.chord, key.map(|(chord, _)| *chord), "{}", item.label);
            }
        }
    }
}

#[test]
fn enabled_follows_the_state_of_the_app() {
    let mut app = TestApp::with_entities([text("a", A)]);
    assert!(!item(&palette(app.app()), "Duplicate").enabled);
    assert!(!item(&palette(app.app()), "Bold").enabled);
    app.select(&["a"]);
    let items = palette(app.app());
    assert!(item(&items, "Duplicate").enabled);
    assert!(item(&items, "Save").enabled);
    // Key-only commands follow their binding's context.
    assert!(item(&items, "Ellipse").enabled);
    assert!(item(&items, "Show next tab").enabled);
}

#[test]
fn the_extra_commands_are_found_beside_their_menu() {
    let items = palette(TestApp::empty().app());
    let ellipse = item(&items, "Ellipse");
    assert_eq!(ellipse.group, "Tools");
    assert_eq!(
        ellipse.run,
        PaletteRun::Action(Action::SetToolVariant(ToolDefaultPatch::ShapeKind(
            ShapeKind::Ellipse
        )))
    );
    // Items of a group are adjacent.
    let mut seen: Vec<&str> = Vec::new();
    for item in &items {
        if seen.last() != Some(&item.group) {
            assert!(!seen.contains(&item.group), "{} is split", item.group);
            seen.push(item.group);
        }
    }
}

#[test]
fn the_empty_query_lists_everything_in_order() {
    let items = [plain("Zeta", false), plain("Alpha", true)];
    let found = search(&items, "  ");
    assert_eq!(
        found.iter().map(|found| found.item).collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn a_query_is_a_case_insensitive_subsequence_of_the_label() {
    let items = [plain("Zoom to fit", true), plain("Zoom in", true)];
    let found = search(&items, "zf");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].item, 0);
    assert_eq!(found[0].positions, [0, 8]);
    assert_eq!(search(&items, "fz").len(), 0);
}

#[test]
fn starts_and_runs_beat_scatter_and_shorter_labels_win_ties() {
    let items = [plain("Duplicate canvas", true), plain("Duplicate", true)];
    assert_eq!(labels(&items, "dup"), ["Duplicate", "Duplicate canvas"]);
    let words = [plain("Send to back", true), plain("Sandbox", true)];
    assert_eq!(labels(&words, "sa"), ["Sandbox", "Send to back"]);
    let gaps = [
        plain("Abxxxxc", true),
        plain("Abc", true),
        plain("Axbc", true),
    ];
    assert_eq!(labels(&gaps, "abc"), ["Abc", "Axbc", "Abxxxxc"]);
}

#[test]
fn enabled_items_come_first_and_keywords_come_after_labels() {
    let mut keyword = plain("Pen", true);
    keyword.keywords = &["freehand"];
    let items = [
        plain("Duplicate", false),
        plain("Duplicate canvas", true),
        keyword,
        plain("Freehand tool", true),
    ];
    assert_eq!(labels(&items, "dup"), ["Duplicate canvas", "Duplicate"]);
    let found = search(&items, "freehand");
    assert_eq!(
        found.iter().map(|found| found.item).collect::<Vec<_>>(),
        [3, 2]
    );
    assert_eq!(found[1].positions.len(), 0);
    // The group title finds an item too.
    assert_eq!(labels(&items, "group").len(), 4);
}

#[test]
fn a_real_palette_finds_zoom_to_fit_by_its_initials() {
    let app = TestApp::with_entities([text("a", A)]);
    let items = palette(app.app());
    let found = search(&items, "zf");
    assert_eq!(items[found[0].item].label, "Zoom to fit");
}
