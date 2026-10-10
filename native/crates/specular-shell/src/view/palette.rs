//! The command palette: Command+K opens a Kit dialog over every command,
//! searched with the model's own fuzzy `search`.
//!
//! The Kit's [`Command`] shows what it is given. Its own filter is a plain
//! substring match, so it is turned off and each query is answered here, in
//! the model's order. With no query the commands are grouped under their
//! headings; with one they are a single ranked list, since a rank crosses
//! groups.
//!
//! The list is read when the palette opens and kept: a command acts on the
//! state the person saw, and each item's `run` depends on it. Opening takes
//! the keys into the dialog and closing hands them back to where they were,
//! which changes nothing the model holds (its focus, a text edit, an entered
//! page).

use std::rc::Rc;

use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::{Disableable as _, WindowExt as _};
use gpui_kit::{
    App, AppContext as _, Context, Entity, Global, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, WeakEntity, Window, div, px,
};
use specular_interact::{PaletteItem, PaletteRun, palette, search};

use super::palette_row::row;
use crate::{canvas, menus, theme};

/// What choosing a command does, once the palette is closed.
type Run = Rc<dyn Fn(PaletteRun, &mut Window, &mut App)>;

/// The palette that is open, to tell a second Command+K from a first.
struct Open(WeakEntity<PaletteView>);

impl Global for Open {}

/// The dialog's content.
pub(super) struct PaletteView {
    state: Entity<CommandState>,
    items: Vec<PaletteItem>,
    query: SharedString,
    run: Run,
}

impl PaletteView {
    pub(super) fn new(items: Vec<PaletteItem>, run: Run, state: Entity<CommandState>) -> Self {
        Self {
            state,
            items,
            query: SharedString::default(),
            run,
        }
    }

    /// The Kit's palette for the query, and the command of each row of each
    /// section, since a confirmed row is told by its place.
    fn listed(&self) -> (Command, Vec<Vec<PaletteRun>>) {
        let rows: Vec<(&str, CommandItem, PaletteRun)> = search(&self.items, &self.query)
            .into_iter()
            .map(|found| {
                let item = &self.items[found.item];
                let (shown, positions) = (item.clone(), found.positions);
                let entry = CommandItem::new()
                    .label(item.label.to_string())
                    .disabled(!item.enabled)
                    .child(move |_, _| row(&shown, &positions));
                (item.group, entry, item.run.clone())
            })
            .collect();
        let command = Command::new(&self.state);
        if !self.query.trim().is_empty() {
            let runs = rows.iter().map(|(_, _, run)| run.clone()).collect();
            let items = rows.into_iter().map(|(_, entry, _)| entry);
            return (command.items(items), vec![runs]);
        }
        let mut command = command;
        let mut sections = Vec::new();
        for group in rows.chunk_by(|a, b| a.0 == b.0) {
            let title = group.first().map_or("", |(title, _, _)| *title);
            let items = group.iter().map(|(_, entry, _)| entry.clone());
            command = command.group(CommandGroup::new().label(title).items(items));
            sections.push(group.iter().map(|(_, _, run)| run.clone()).collect());
        }
        (command, sections)
    }
}

impl Render for PaletteView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let view = cx.weak_entity();
        let (command, sections) = self.listed();
        let run = Rc::clone(&self.run);
        command
            .filterable(false)
            .bordered(false)
            .placeholder("Search commands")
            .max_h(px(380.0))
            .empty(|_, _, _| {
                div()
                    .p_4()
                    .text_size(px(13.0))
                    .text_color(theme::tinted(theme::text_muted()))
                    .child("No commands match")
            })
            .on_query(move |query, _, cx| {
                let query = SharedString::from(query.to_owned());
                view.update(cx, |palette, cx| {
                    palette.query = query;
                    cx.notify();
                })
                .ok();
            })
            .on_confirm(move |at, window, cx| {
                let Some(chosen) = sections.get(at.section).and_then(|rows| rows.get(at.row))
                else {
                    return;
                };
                // Closed first, so the keys are back where they were and the
                // command acts on the state the person saw.
                window.close_dialog(cx);
                run(chosen.clone(), window, cx);
            })
    }
}

/// Runs a command of the palette: an action as a chosen menu item, a shell
/// command as its menu item.
fn run_command(run: PaletteRun, _: &mut Window, cx: &mut App) {
    match run {
        PaletteRun::Action(action) => menus::run_action(action, cx),
        PaletteRun::Shell(command) => menus::run_shell(command, cx),
    }
}

/// Opens the palette over the window, listing the commands as they are now.
pub(crate) fn open(window: &mut Window, cx: &mut App) {
    let Some(items) = canvas::with(|canvas| palette(canvas.runtime.app())) else {
        return;
    };
    let state = cx.new(|cx| CommandState::new(window, cx));
    let view = cx.new(|_| PaletteView::new(items, Rc::new(run_command), state.clone()));
    cx.set_global(Open(view.downgrade()));
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(560.0))
            .margin_top(px(96.0))
            .close_button(false)
            // The list keeps its own inset, so the dialog's would double it.
            .p_0()
            .child(view.clone())
    });
    state.update(cx, |state, cx| state.focus(window, cx));
}

/// Command+K: opens the palette, or closes it when it is open.
pub(crate) fn toggle(window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        let ours = (cx.try_global::<Open>()).is_some_and(|open| open.0.upgrade().is_some());
        if ours {
            window.close_dialog(cx);
        }
        return;
    }
    open(window, cx);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui_kit::component::command::CommandState;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, WindowHandle, base::Root};
    use specular_interact::{Action, PaletteItem, PaletteRun, ShellCommand};

    use super::*;

    fn item(
        label: &'static str,
        group: &'static str,
        run: PaletteRun,
        enabled: bool,
    ) -> PaletteItem {
        PaletteItem {
            label: label.into(),
            group,
            run,
            chord: None,
            enabled,
            checked: None,
            keywords: &[],
        }
    }

    /// A palette in a headless window, and what it ran.
    fn open_palette(
        cx: &mut TestAppContext,
    ) -> (
        WindowHandle<Root>,
        Entity<CommandState>,
        Rc<RefCell<Vec<PaletteRun>>>,
    ) {
        cx.update(gpui_kit::init);
        let ran = Rc::new(RefCell::new(Vec::new()));
        let items = vec![
            item("Copy", "Edit", PaletteRun::Action(Action::Copy), true),
            item("Paste", "Edit", PaletteRun::Action(Action::Paste), false),
            item("Save", "File", PaletteRun::Shell(ShellCommand::Save), true),
        ];
        let record = Rc::clone(&ran);
        let run: Run = Rc::new(move |run, _, _| record.borrow_mut().push(run));
        let mut state = None;
        let window = cx.update(|cx| {
            let (window, _) =
                gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |window, cx| {
                    let command = cx.new(|cx| CommandState::new(window, cx));
                    command.update(cx, |command, cx| command.focus(window, cx));
                    state = Some(command.clone());
                    cx.new(|_| PaletteView::new(items, run, command))
                })
                .expect("a test window");
            window.downcast::<Root>().expect("the Kit's root")
        });
        (window, state.expect("the palette's state"), ran)
    }

    fn type_and_confirm(cx: &mut TestAppContext, window: WindowHandle<Root>, query: &str) {
        // A frame passes between the typing and the key, as it does for a person.
        cx.update_window(window.into(), |_, window, cx| window.input(query, cx))
            .expect("the window");
        cx.run_until_parked();
        cx.update_window(window.into(), |_, window, cx| window.press("enter", cx))
            .expect("the window");
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn the_palette_lists_every_command(cx: &mut TestAppContext) {
        let (window, state, _) = open_palette(cx);
        cx.update_window(window.into(), |_, window, cx| window.render_frame(cx))
            .expect("the window");
        assert_eq!(cx.read_entity(&state, |state, _| state.matched_count()), 3);
    }

    #[gpui_kit::test]
    fn a_query_narrows_the_list_and_ranks_an_enabled_command_first(cx: &mut TestAppContext) {
        let (window, state, ran) = open_palette(cx);
        type_and_confirm(cx, window, "p");
        // "Copy" and "Paste" hold a p, and the enabled one is the one confirmed.
        assert_eq!(cx.read_entity(&state, |state, _| state.matched_count()), 2);
        assert_eq!(*ran.borrow(), [PaletteRun::Action(Action::Copy)]);
    }

    #[gpui_kit::test]
    fn the_query_is_matched_by_the_models_fuzzy_search_not_the_kits(cx: &mut TestAppContext) {
        let (window, state, ran) = open_palette(cx);
        type_and_confirm(cx, window, "cpy");
        assert_eq!(cx.read_entity(&state, |state, _| state.matched_count()), 1);
        assert_eq!(*ran.borrow(), [PaletteRun::Action(Action::Copy)]);
    }

    #[gpui_kit::test]
    fn a_command_that_cannot_run_cannot_be_confirmed(cx: &mut TestAppContext) {
        let (window, _, ran) = open_palette(cx);
        type_and_confirm(cx, window, "paste");
        assert!(ran.borrow().is_empty(), "{:?}", ran.borrow());
    }

    #[gpui_kit::test]
    fn confirming_with_no_query_runs_the_first_command(cx: &mut TestAppContext) {
        let (window, _, ran) = open_palette(cx);
        type_and_confirm(cx, window, "");
        assert_eq!(*ran.borrow(), [PaletteRun::Action(Action::Copy)]);
    }
}
