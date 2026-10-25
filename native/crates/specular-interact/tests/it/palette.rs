//! The command palette's search ranks as a person expects. What the palette
//! lists is read in `fixtures/scenarios/menus-and-settings.txt`.

use std::borrow::Cow;

use specular_doc::Rect;
use specular_interact::{PaletteItem, PaletteRun, ShellCommand, palette, search};
use specular_testkit::{TestApp, text};

const A: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);

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
