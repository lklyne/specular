//! The command palette's model: every command a person can pick by name,
//! and the fuzzy search over them.
//!
//! The list is the menu bar's items, so a label, a key, and whether the item
//! can run now are the menu's, plus what has a key and no menu item (the
//! formatting keys, the tool variants, the tab keys) and the window's own
//! commands. Every label is the command registry's. The view renders [`palette`] and filters it with [`search`];
//! choosing an item sends its [`PaletteRun`].

use std::borrow::Cow;

use specular_doc::{BrushType, ShapeKind};

use crate::commands::{Run, naming};
use crate::menu::{command_item, has_target};
use crate::{
    Action, App, BINDINGS, Chord, Format, MenuEntry, SHELL_KEYS, ShellCommand, ToolDefaultPatch,
    menus,
};

/// What choosing a palette item does.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteRun {
    /// Send it to the app as an [`Event::Action`](crate::Event).
    Action(Action),
    /// The shell does it itself.
    Shell(ShellCommand),
    /// Run the command `name` with `args`, by name: what a mod's palette
    /// entry or own command does.
    Named {
        /// The command.
        name: String,
        /// Its arguments.
        args: serde_json::Value,
    },
}

/// One command in the palette.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteItem {
    /// The text, and what [`search`] matches first.
    pub label: Cow<'static, str>,
    /// The menu it comes from, or the title of the group of the commands
    /// the menus lack. Items of a group are adjacent.
    pub group: &'static str,
    /// What choosing it does.
    pub run: PaletteRun,
    /// The key that does the same, from the binding table.
    pub chord: Option<Chord>,
    /// Whether it can be chosen now. A disabled item stays listed, dimmed.
    pub enabled: bool,
    /// For an item with a check mark, whether it is checked.
    pub checked: Option<bool>,
    /// Other words that find it.
    pub keywords: &'static [&'static str],
}

/// The registry commands with a key and no menu item, and the palette group
/// each joins. A command a menu item already runs is skipped, so a row here
/// can never list one twice.
///
/// `edit.cancel` and `edit.nudge` are in no row: Escape means "back out" of
/// whatever is happening, which a command that closes the palette cannot
/// say, and a nudge needs the key's direction and a held selection to mean
/// anything.
const EXTRAS: &[(&str, &str)] = &[
    ("Tools", "tool.ellipse"),
    ("Tools", "tool.diamond"),
    ("Tools", "tool.highlighter"),
    ("Page", "tab.next"),
    ("Page", "tab.previous"),
    ("Text", "text.bold"),
    ("Text", "text.italic"),
    ("Text", "text.code"),
    ("Text", "text.strikethrough"),
    ("Text", "text.bullet-list"),
    ("Text", "text.numbered-list"),
    ("Text", "text.task-list"),
    ("Text", "text.body"),
    ("Text", "text.heading-1"),
    ("Text", "text.heading-2"),
    ("Text", "text.heading-3"),
    ("Text", "text.heading-4"),
    ("Text", "text.heading-5"),
    ("Text", "text.heading-6"),
];

/// The group of the window's own commands.
const SHELL_GROUP: &str = "App";

/// The group of the entries the loaded mods add.
const MODS_GROUP: &str = "Mods";

/// Words besides the label that find an action: what a person calls it
/// where the menu calls it something else.
fn keywords(action: &Action) -> &'static [&'static str] {
    match action {
        Action::SetToolVariant(ToolDefaultPatch::ShapeKind(ShapeKind::Rectangle)) => {
            &["rectangle", "square", "box"]
        }
        Action::SetToolVariant(ToolDefaultPatch::ShapeKind(ShapeKind::Ellipse)) => {
            &["circle", "oval", "shape"]
        }
        Action::SetToolVariant(ToolDefaultPatch::ShapeKind(ShapeKind::Diamond)) => {
            &["rhombus", "shape"]
        }
        Action::SetToolVariant(ToolDefaultPatch::Brush(BrushType::Pen)) => {
            &["pen", "brush", "freehand", "pencil"]
        }
        Action::SetToolVariant(ToolDefaultPatch::Brush(BrushType::Highlight)) => {
            &["marker", "brush", "draw"]
        }
        Action::ZoomToFit => &["fit", "all", "overview"],
        Action::ZoomReset => &["actual size", "reset"],
        Action::Delete => &["remove", "trash"],
        Action::Format(Format::Strike) => &["strike", "cross out"],
        Action::Format(Format::Heading(_)) => &["title", "heading"],
        Action::NewPageTab => &["browser", "tab", "page"],
        Action::EditPageUrl => &["url", "navigate", "go to"],
        Action::PageReload => &["refresh"],
        Action::AutoLayout => &["tidy", "align", "grid"],
        _ => &[],
    }
}

/// Every command, grouped, for `app` as it is now: one group per menu in
/// the menu bar's order, in which the extra commands that belong with a
/// menu come last; then a group for each of the others; then the window's;
/// then the loaded mods' entries. The list's membership never changes with
/// the state of the app, except for the Canvas group's canvases and the
/// mods loaded. Only `enabled` and `checked` do.
pub fn palette(app: &App) -> Vec<PaletteItem> {
    let mut groups: Vec<(&'static str, Vec<PaletteItem>)> = Vec::new();
    for menu in menus(app) {
        let items = (menu.entries.into_iter())
            .filter_map(|entry| match entry {
                MenuEntry::Item(item) => Some(PaletteItem {
                    keywords: keywords(&item.action),
                    label: item.label,
                    group: menu.title,
                    chord: item.chord,
                    enabled: item.enabled,
                    checked: item.checked,
                    run: PaletteRun::Action(item.action),
                }),
                MenuEntry::Separator => None,
            })
            .collect();
        groups.push((menu.title, items));
    }
    for &(group, name) in EXTRAS {
        let Some(palette_item) = extra_item(app, group, name) else {
            continue;
        };
        if (groups.iter().flat_map(|(_, items)| items)).any(|it| it.run == palette_item.run) {
            continue;
        }
        match groups.iter_mut().find(|(title, _)| *title == group) {
            Some((_, items)) => items.push(palette_item),
            None => groups.push((group, vec![palette_item])),
        }
    }
    let shell = (SHELL_KEYS.iter())
        .filter(|(_, command)| *command != ShellCommand::CommandPalette)
        .filter_map(|&(chord, command)| {
            Some(PaletteItem {
                label: Cow::Borrowed(naming(&Run::Shell(command))?.label),
                group: SHELL_GROUP,
                run: PaletteRun::Shell(command),
                chord: Some(chord),
                enabled: true,
                checked: None,
                keywords: &[],
            })
        })
        .collect();
    groups.push((SHELL_GROUP, shell));
    groups.push((MODS_GROUP, mod_items(app)));
    groups.into_iter().flat_map(|(_, items)| items).collect()
}

/// What the running mods add: their `palette` entries, each enabled while
/// its registry command could run (one naming no command is listed,
/// disabled, so its author sees it and the palette does not hide the
/// mistake), then their own commands with their keys.
fn mod_items(app: &App) -> Vec<PaletteItem> {
    let mods = crate::mods::running_mods(app);
    let entries = (mods.iter()).flat_map(|installed| installed.manifest.palette.iter());
    let entries = entries.map(|entry| PaletteItem {
        label: Cow::Owned(entry.title.clone()),
        group: MODS_GROUP,
        run: PaletteRun::Named {
            name: entry.run.clone(),
            args: entry.args.clone(),
        },
        chord: None,
        enabled: crate::commands::run_named(app, &entry.run, &entry.args).is_ok(),
        checked: None,
        keywords: &[],
    });
    let commands = (mods.iter()).flat_map(|installed| {
        (installed.manifest.commands.iter()).map(|spec| PaletteItem {
            label: Cow::Owned(spec.title.clone()),
            group: MODS_GROUP,
            run: PaletteRun::Named {
                name: crate::mods::word(installed, spec),
                args: serde_json::Value::Null,
            },
            chord: spec.key.as_deref().and_then(Chord::parse),
            enabled: app.session.gesture.is_none(),
            checked: None,
            keywords: &[],
        })
    });
    entries.chain(commands).collect()
}

/// The registry command `name` as a palette item of `group`.
fn extra_item(app: &App, group: &'static str, name: &str) -> Option<PaletteItem> {
    let menu_item = command_item(app, name)?;
    // A key bound in two contexts (the tab keys, on the canvas and in an
    // entered page) works wherever either holds.
    let enabled = app.session.gesture.is_none()
        && has_target(app, &menu_item.action)
        && (BINDINGS.iter())
            .filter(|binding| binding.action == menu_item.action)
            .any(|binding| binding.context.holds(app));
    Some(PaletteItem {
        label: menu_item.label,
        group,
        chord: menu_item.chord,
        enabled,
        checked: None,
        keywords: keywords(&menu_item.action),
        run: PaletteRun::Action(menu_item.action),
    })
}

/// An item that matched a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteMatch {
    /// The index of the item in the slice that was searched.
    pub item: usize,
    /// How well it matched. Only comparable between matches of one query.
    pub(crate) score: i32,
    /// The characters (not bytes) of the label that matched, ascending,
    /// for highlighting. Empty when the label did not match and a keyword
    /// or the group did, and for the empty query.
    pub positions: Vec<usize>,
}

/// The items `query` finds, best first.
///
/// A query matches when its characters, ignoring case and spaces, appear in
/// order in the label, or failing that in a keyword or the group title.
/// Enabled items come before disabled ones, then label matches before
/// keyword and group matches, then the higher score (a match at the start or at a word start, and runs of
/// adjacent characters, score highest), then the shorter label.
///
/// The empty query matches every item in the order given, enabled or not.
pub fn search(items: &[PaletteItem], query: &str) -> Vec<PaletteMatch> {
    let query: Vec<char> = (query.chars())
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if query.is_empty() {
        return (0..items.len())
            .map(|item| PaletteMatch {
                item,
                score: 0,
                positions: Vec::new(),
            })
            .collect();
    }
    // (enabled, matched the label, match)
    let mut found: Vec<(bool, bool, PaletteMatch)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let (label, score, positions) =
                if let Some((score, positions)) = fuzzy(&query, &item.label) {
                    (true, score, positions)
                } else {
                    let other = item.keywords.iter().copied().chain([item.group]);
                    let score = other.filter_map(|text| fuzzy(&query, text)).map(|m| m.0);
                    (false, score.max()?, Vec::new())
                };
            let matched = PaletteMatch {
                item: index,
                score,
                positions,
            };
            Some((item.enabled, label, matched))
        })
        .collect();
    found.sort_by_key(|(enabled, label, matched)| {
        (
            !enabled,
            !label,
            -matched.score,
            items[matched.item].label.chars().count(),
            matched.item,
        )
    });
    found.into_iter().map(|(_, _, matched)| matched).collect()
}

const WORD_START: i32 = 10;
const PREFIX: i32 = 8;
const ADJACENT: i32 = 6;
const GAP: i32 = 1;

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// The best way `query` (already folded) is a subsequence of `text`: its
/// score and the character positions used.
fn fuzzy(query: &[char], text: &str) -> Option<(i32, Vec<usize>)> {
    let chars: Vec<char> = text.chars().collect();
    let folded: Vec<char> = chars.iter().copied().map(fold).collect();
    if query.len() > chars.len() {
        return None;
    }
    let bonus = |at: usize| {
        let word_start = at == 0 || !chars[at - 1].is_alphanumeric();
        let prefix = if at == 0 { PREFIX } else { 0 };
        (if word_start { WORD_START } else { 0 }) + prefix
    };
    // best[i][j]: the best score with query[i] matched at text[j], and the
    // position query[i - 1] took to get it.
    let mut best: Vec<Vec<Option<(i32, usize)>>> = vec![vec![None; chars.len()]; query.len()];
    for (j, &c) in folded.iter().enumerate() {
        if c == query[0] {
            best[0][j] = Some((bonus(j), 0));
        }
    }
    for i in 1..query.len() {
        for j in i..chars.len() {
            if folded[j] != query[i] {
                continue;
            }
            let from = (i - 1..j).filter_map(|k| {
                let (score, _) = best[i - 1][k]?;
                let link = if k + 1 == j {
                    ADJACENT
                } else {
                    -GAP * i32::try_from(j - k - 1).unwrap_or(i32::MAX)
                };
                Some((score + link, k))
            });
            if let Some((score, k)) = from.max_by_key(|&(score, _)| score) {
                best[i][j] = Some((score + bonus(j), k));
            }
        }
    }
    let last = query.len() - 1;
    let (mut at, (score, _)) = best[last]
        .iter()
        .enumerate()
        .filter_map(|(j, cell)| Some((j, (*cell)?)))
        .max_by_key(|&(_, (score, _))| score)?;
    let mut positions = vec![at];
    for i in (1..query.len()).rev() {
        at = best[i][at]?.1;
        positions.push(at);
    }
    positions.reverse();
    Some((score, positions))
}
