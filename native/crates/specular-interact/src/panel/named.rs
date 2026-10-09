//! A control by its name, with nothing laid out.
//!
//! The models say which controls exist and what each one does, so a caller
//! with no layout (a headless script, a test) can do what a click on a
//! control does by naming it: [`named_controls`] lists the names and
//! [`Event::Control`](crate::Event) activates one. The names are the models'
//! own, the ones every renderer tracks a control by.
//!
//! With no layout there is nothing to see, so what a renderer shows only at
//! times is always there: an option of a list is named whether or not its
//! list is open, every row of the sidebar is named while its section and
//! the rows it is inside are unfolded, and the items of the context menu
//! are those of the open menu, or of the selection when none is open. The
//! list and the menu that are open are still kept, because they change what
//! the next key and the next press do: Escape closes one before it does
//! anything else, and activating a control outside one closes it and does
//! nothing more, as a press outside does.

use glam::Vec2;
use specular_core::Modifiers;

use super::{
    Control, ControlId, Dropdown, DropdownSection, MenuTarget, ToolbarSection, context_menu, dock,
    toolbar, view_strip,
};
use crate::update::run_action;
use crate::{Action, App, Effect, SidebarRow, edit, hit};

/// A name that no control has now, with the names that controls do have.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("no control `{name}` is shown; these are: {}", shown.join(" "))]
pub struct UnknownControl {
    /// The name asked for.
    pub name: String,
    /// The names of the controls there are, in the models' order.
    pub shown: Vec<String>,
}

/// What activating a control does.
#[derive(Debug, Clone, PartialEq)]
enum Does {
    /// Runs an action, unless the control is disabled.
    Run {
        action: Action,
        enabled: bool,
        /// Whether it closes the list it is in.
        closes: bool,
    },
    /// Opens the list under it, or closes it.
    Opens,
    /// Takes the keys, to be typed in.
    Edits,
}

/// What a control is inside, which must be open for a press to reach it
/// while something else is.
#[derive(Debug, Clone, PartialEq)]
enum Inside {
    Nothing,
    /// The list under this dropdown.
    List(ControlId),
    /// The context menu of this target.
    Menu(MenuTarget),
}

#[derive(Debug, Clone, PartialEq)]
struct Named {
    id: ControlId,
    does: Does,
    inside: Inside,
}

fn run(id: &ControlId, action: &Action, enabled: bool, closes: bool, inside: &Inside) -> Named {
    Named {
        id: id.clone(),
        does: Does::Run {
            action: action.clone(),
            enabled,
            closes,
        },
        inside: inside.clone(),
    }
}

fn sections(content: &[DropdownSection], inside: &Inside, out: &mut Vec<Named>) {
    for section in content {
        match section {
            DropdownSection::Options { options, .. } => {
                out.extend(
                    (options.iter()).map(|option| {
                        run(&option.id, &option.action, option.enabled, true, inside)
                    }),
                );
            }
            DropdownSection::Controls(controls) => {
                for inner in controls {
                    control(inner, inside, out);
                }
            }
        }
    }
}

fn dropdown(model: &Dropdown, inside: &Inside, out: &mut Vec<Named>) {
    out.push(Named {
        id: model.id.clone(),
        does: Does::Opens,
        inside: inside.clone(),
    });
    // A list inside a list is reached through the outer one.
    let list = match inside {
        Inside::Nothing => Inside::List(model.id.clone()),
        Inside::List(_) | Inside::Menu(_) => inside.clone(),
    };
    sections(&model.content, &list, out);
}

fn control(model: &Control, inside: &Inside, out: &mut Vec<Named>) {
    match model {
        Control::Button(button) => {
            out.push(run(
                &button.id,
                &button.action,
                button.enabled,
                false,
                inside,
            ));
        }
        Control::Toggle(toggle) => {
            out.push(run(
                &toggle.id,
                &toggle.action,
                toggle.enabled,
                false,
                inside,
            ));
        }
        Control::Swatches(row) => out.extend(
            (row.options.iter())
                .map(|swatch| run(&swatch.id, &swatch.action, row.enabled, false, inside)),
        ),
        Control::Dropdown(model) => dropdown(model, inside, out),
        Control::Stepper(stepper) => {
            let halves = [
                ("dec", &stepper.decrement, stepper.can_decrement),
                ("inc", &stepper.increment, stepper.can_increment),
            ];
            for (part, action, enabled) in halves {
                out.push(run(&stepper.id.child(part), action, enabled, false, inside));
            }
        }
        Control::Field(field) => out.push(Named {
            id: field.id.clone(),
            does: Does::Edits,
            inside: inside.clone(),
        }),
        Control::Choices(choices) => sections(&choices.content, inside, out),
        Control::Separator => {}
    }
}

fn menu(app: &App, target: &MenuTarget, out: &mut Vec<Named>) {
    let inside = Inside::Menu(target.clone());
    for model in context_menu(app, target)
        .iter()
        .flat_map(|menu| &menu.controls)
    {
        control(model, &inside, out);
    }
}

fn rows(rows: &[SidebarRow], out: &mut Vec<Named>) {
    for row in rows {
        out.push(run(&row.id, &row.action, true, true, &Inside::Nothing));
        if let Some(toggle) = &row.toggle {
            let id = row.id.child("toggle");
            out.push(run(&id, toggle, true, true, &Inside::Nothing));
        }
        if row.expanded == Some(true) {
            self::rows(&row.children, out);
        }
    }
}

fn sidebar(app: &App, out: &mut Vec<Named>) {
    let model = crate::sidebar(app);
    if !model.visible {
        return;
    }
    let plain = Inside::Nothing;
    let head = |head: &crate::SectionHead| run(&head.id, &head.toggle, true, true, &plain);
    out.push(head(&model.canvases_head));
    let add = ControlId::new("sidebar.add");
    out.push(run(&add, &model.add_canvas, true, true, &plain));
    if !model.canvases_head.folded {
        for canvas in &model.canvases {
            out.push(run(&canvas.control, &canvas.action, true, true, &plain));
            out.push(Named {
                id: canvas.rename.id.clone(),
                does: Does::Edits,
                inside: Inside::Nothing,
            });
            menu(app, &MenuTarget::Canvas(canvas.id.clone()), out);
        }
    }
    for (section, listed) in [
        (&model.notes_head, &model.notes),
        (&model.pages_head, &model.pages),
    ] {
        out.push(head(section));
        if !section.folded {
            rows(listed, out);
        }
    }
}

/// Every control there is now, in the models' order: the tab row, the
/// toolbar, the dock, the sidebar, the context menu.
fn named(app: &App) -> Vec<Named> {
    let mut out = Vec::new();
    let plain = Inside::Nothing;
    let strip = view_strip(app);
    for tab in &strip.tabs {
        out.push(run(&tab.id, &tab.action, true, true, &plain));
    }
    out.push(run(&strip.add.id, &strip.add.action, true, true, &plain));
    let bar = toolbar(app);
    out.extend((bar.chat.iter()).map(|button| run(&button.id, &button.action, true, true, &plain)));
    for section in &bar.sections {
        match section {
            ToolbarSection::Tools(tools) => {
                out.extend(
                    (tools.iter()).map(|tool| run(&tool.id, &tool.action, true, true, &plain)),
                );
            }
            ToolbarSection::Zoom(zoom) => {
                // The theme button is drawn just before the zoom readout.
                out.push(run(&bar.theme.id, &bar.theme.action, true, true, &plain));
                dropdown(zoom, &plain, &mut out);
            }
        }
    }
    for model in &bar.view {
        control(model, &plain, &mut out);
    }
    for model in dock(app).iter().flat_map(|dock| &dock.controls) {
        control(model, &plain, &mut out);
    }
    sidebar(app, &mut out);
    match &app.session.panel.menu {
        Some(open) if !matches!(open.target, MenuTarget::Canvas(_)) => {
            menu(app, &open.target, &mut out);
        }
        Some(_) => {}
        None => {
            let items = app.session.selection.items().to_vec();
            let target = if items.is_empty() {
                MenuTarget::Empty
            } else {
                MenuTarget::Selection(items)
            };
            menu(app, &target, &mut out);
        }
    }
    out
}

/// The names of the controls there are now, whether or not anything draws
/// them: what [`Event::Control`](crate::Event) takes.
pub fn named_controls(app: &App) -> Vec<ControlId> {
    named(app).into_iter().map(|named| named.id).collect()
}

/// `name` as the name of a control there is now, or the names there are.
pub fn control_named(app: &App, name: &str) -> Result<ControlId, UnknownControl> {
    let shown = named_controls(app);
    match shown.iter().find(|id| id.as_str() == name) {
        Some(id) => Ok(id.clone()),
        None => Err(UnknownControl {
            name: name.to_owned(),
            shown: (shown.iter().map(|id| id.as_str().to_owned())).collect(),
        }),
    }
}

/// Does what a click on the control named `id` does, with `keys` held. A
/// name no control has does nothing.
pub(crate) fn activate(app: &mut App, id: &ControlId, keys: Modifiers, effects: &mut Vec<Effect>) {
    // A press on anything but the field being edited ends the edit first,
    // keeping what was typed, as a blur does.
    let editing = (app.text_edit().filter(|edit| edit.is_field()))
        .map(|edit| ControlId::from(edit.entity().as_str().to_owned()));
    if editing.as_ref().is_some_and(|editing| editing != id) {
        edit::end(app, effects);
    }
    let Some(found) = named(app).into_iter().find(|named| named.id == *id) else {
        return;
    };
    // A press anywhere but on the open menu, or in the open list or on its
    // trigger, closes it, and that is all it does.
    let ui = &mut app.session.panel;
    if let Some(open) = &ui.menu
        && found.inside != Inside::Menu(open.target.clone())
    {
        ui.menu = None;
        return;
    }
    if let Some(open) = &ui.open
        && found.id != *open
        && found.inside != Inside::List(open.clone())
    {
        ui.open = None;
        return;
    }
    let menu = ui.menu.take();
    match found.does {
        Does::Opens => {
            let open = &mut app.session.panel.open;
            *open = (open.as_ref() != Some(id)).then(|| id.clone());
        }
        Does::Edits => edit::begin_field(app, id, effects),
        Does::Run { enabled: false, .. } => {}
        Does::Run {
            action,
            enabled: true,
            closes,
        } => {
            let action = super::builtin::picked(app, id, action, keys);
            // A paste lands where the menu was opened.
            if let (Action::Paste, Some(menu)) = (&action, &menu) {
                app.session.pointer = Some(menu.at);
            }
            run_action(app, action, effects);
            if closes {
                app.session.panel.open = None;
            }
        }
    }
}

/// A right press at `screen` from a caller with no layout: opens the
/// context menu for what is there, after selecting it as the press does.
/// A point the sidebar covers is not on the canvas, and opens nothing; nor
/// does a press that belongs to a page, a tool, a drag or a text edit.
pub(crate) fn open_menu(app: &mut App, screen: Vec2) {
    app.session.panel.menu = None;
    if screen.x < app.covered_left() {
        return;
    }
    let under = hit::hit_test(app, screen);
    super::builtin::open_menu_for_press(app, screen, &under);
}
