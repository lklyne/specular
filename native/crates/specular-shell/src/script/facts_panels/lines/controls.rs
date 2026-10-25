//! The `controls` topic: every control the toolbar, dock and view strip offer.

use std::fmt::Write as _;

use specular_interact::App;
use specular_interact::panel::{
    Control, ControlId, DropdownSection, MenuTarget, ToolbarSection, context_menu, dock, toolbar,
    view_strip,
};

/// A control's line: its name, `on` when it is on or chosen, `disabled`,
/// `= value` for what it shows, and its label in quotes.
fn line(id: &ControlId, on: bool, enabled: bool, value: Option<&str>, label: &str) -> String {
    let mut line = id.as_str().to_owned();
    if on {
        line.push_str(" on");
    }
    if !enabled {
        line.push_str(" disabled");
    }
    if let Some(value) = value {
        let _ = write!(line, " = {value}");
    }
    let _ = write!(line, " \"{label}\"");
    line
}

fn sections(content: &[DropdownSection], out: &mut Vec<String>) {
    for section in content {
        match section {
            DropdownSection::Options { options, .. } => {
                for option in options {
                    let value = option.trailing.as_deref();
                    let label = &option.label;
                    out.push(line(
                        &option.id,
                        option.selected,
                        option.enabled,
                        value,
                        label,
                    ));
                    for field in &option.fields {
                        out.push(line(
                            &field.id,
                            false,
                            true,
                            Some(&field.value),
                            &field.label,
                        ));
                    }
                }
            }
            DropdownSection::Controls(controls) => {
                for control in controls {
                    control_lines(control, out);
                }
            }
        }
    }
}

fn control_lines(control: &Control, out: &mut Vec<String>) {
    match control {
        Control::Button(button) => {
            out.push(line(&button.id, false, button.enabled, None, &button.label));
        }
        Control::Toggle(toggle) => {
            out.push(line(
                &toggle.id,
                toggle.on,
                toggle.enabled,
                None,
                &toggle.label,
            ));
        }
        Control::Swatches(row) => {
            for swatch in &row.options {
                out.push(line(
                    &swatch.id,
                    swatch.selected,
                    row.enabled,
                    None,
                    &swatch.label,
                ));
            }
        }
        Control::Dropdown(dropdown) => {
            let value = dropdown.summary.text.as_deref();
            out.push(line(&dropdown.id, false, true, value, &dropdown.label));
            sections(&dropdown.content, out);
        }
        Control::Stepper(stepper) => {
            out.push(line(
                &stepper.id,
                false,
                true,
                Some(&stepper.value),
                &stepper.label,
            ));
            let halves = [
                ("dec", stepper.can_decrement),
                ("inc", stepper.can_increment),
            ];
            for (part, enabled) in halves {
                out.push(line(&stepper.id.child(part), false, enabled, None, part));
            }
        }
        Control::Field(field) => {
            out.push(line(
                &field.id,
                false,
                true,
                Some(&field.value),
                &field.label,
            ));
        }
        Control::Choices(choices) => sections(&choices.content, out),
        Control::Separator => {}
    }
}

pub(super) fn controls(app: &App, out: &mut Vec<String>) {
    let strip = view_strip(app);
    for tab in &strip.tabs {
        out.push(line(&tab.id, tab.active, true, None, &tab.label));
    }
    out.push(line(
        &strip.add.id,
        false,
        strip.add.enabled,
        None,
        &strip.add.label,
    ));
    let bar = toolbar(app);
    if let Some(button) = &bar.chat {
        out.push(line(&button.id, button.open, true, None, &button.label));
    }
    out.push(line(&bar.theme.id, false, true, None, &bar.theme.label));
    for section in &bar.sections {
        match section {
            ToolbarSection::Tools(tools) => {
                for tool in tools {
                    out.push(line(&tool.id, tool.active, true, None, &tool.label));
                }
            }
            ToolbarSection::Zoom(zoom) => control_lines(&Control::Dropdown(zoom.clone()), out),
        }
    }
    for control in &bar.view {
        control_lines(control, out);
    }
    for control in dock(app).iter().flat_map(|dock| &dock.controls) {
        control_lines(control, out);
    }
    if let Some(open) = &app.session().panel.menu {
        out.push(format!("menu {:?}", open.target));
        let target: &MenuTarget = &open.target;
        for control in context_menu(app, target)
            .iter()
            .flat_map(|menu| &menu.controls)
        {
            control_lines(control, out);
        }
    }
}
