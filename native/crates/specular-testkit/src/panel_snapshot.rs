//! The toolbar and the dock as text for `insta`.

use std::fmt::Write as _;

use specular_interact::{
    Chord, Control, Dropdown, DropdownSection, Face, Key, OptionLayout, PopupModel, ToolbarModel,
    ToolbarSection,
};

/// The toolbar as stable text, one line per control or option, `---`
/// between its groups. `[x]` marks the active tool and the selected option.
pub fn toolbar_snapshot(toolbar: &ToolbarModel) -> String {
    let mut out = String::new();
    for (index, section) in toolbar.sections.iter().enumerate() {
        if index > 0 {
            out.push_str("---\n");
        }
        match section {
            ToolbarSection::Tools(tools) => {
                for tool in tools {
                    let _ = writeln!(
                        out,
                        "tool {} {} {:?} {}{} -> {:?}",
                        mark(tool.active),
                        tool.id,
                        tool.label,
                        face(&Face::icon(tool.icon).tinted(tool.tint.clone())),
                        chord(tool.chord),
                        tool.action
                    );
                }
            }
            ToolbarSection::Zoom(zoom) => dropdown(&mut out, zoom, 0),
        }
    }
    out.trim_end().to_owned()
}

/// The dock's controls, or a context menu's, as stable text, or `none`
/// when there are none.
pub fn popup_snapshot(popup: Option<&PopupModel>) -> String {
    let Some(popup) = popup else {
        return "none".to_owned();
    };
    let mut out = String::new();
    controls(&mut out, &popup.controls, 0);
    out.trim_end().to_owned()
}

fn mark(on: bool) -> &'static str {
    if on { "[x]" } else { "[ ]" }
}

fn mark_short(on: bool) -> &'static str {
    if on { "*" } else { "" }
}

fn pad(out: &mut String, depth: usize) {
    out.push_str(&"  ".repeat(depth));
}

fn controls(out: &mut String, list: &[Control], depth: usize) {
    for control in list {
        self::control(out, control, depth);
    }
}

fn control(out: &mut String, control: &Control, depth: usize) {
    if let Control::Dropdown(open) = control {
        dropdown(out, open, depth);
        return;
    }
    pad(out, depth);
    match control {
        Control::Button(button) => {
            let _ = writeln!(
                out,
                "button {} {:?} {}{}{} -> {:?}",
                button.id,
                button.label,
                face(&button.face),
                chord(button.chord),
                if button.enabled { "" } else { " disabled" },
                button.action
            );
        }
        Control::Toggle(toggle) => {
            let _ = writeln!(
                out,
                "toggle {} {} {:?} {}{}{} -> {:?}",
                mark(toggle.on),
                toggle.id,
                toggle.label,
                face(&toggle.face),
                chord(toggle.chord),
                if toggle.enabled { "" } else { " disabled" },
                toggle.action
            );
        }
        Control::Swatches(swatches) => {
            let _ = write!(
                out,
                "swatches {} {:?}/{:?}{}:",
                swatches.id,
                swatches.palette,
                swatches.role,
                if swatches.enabled { "" } else { " disabled" }
            );
            for swatch in &swatches.options {
                let name = swatch.label.to_lowercase();
                let _ = write!(out, " {}{name}", mark_short(swatch.selected));
            }
            out.push('\n');
        }
        Control::Dropdown(_) => {}
        Control::Stepper(stepper) => {
            let _ = writeln!(
                out,
                "stepper {} {:?} value={} dec{} -> {:?} inc{} -> {:?}",
                stepper.id,
                stepper.label,
                stepper.value,
                if stepper.can_decrement { "" } else { "(off)" },
                stepper.decrement,
                if stepper.can_increment { "" } else { "(off)" },
                stepper.increment
            );
        }
        Control::Field(field) => {
            let caption = (field.caption.as_ref())
                .map_or_else(String::new, |caption| format!(" caption={caption:?}"));
            let placeholder = (field.placeholder.as_ref())
                .map_or_else(String::new, |text| format!(" placeholder={text:?}"));
            let _ = writeln!(
                out,
                "field {} {:?}{caption} value={:?}{placeholder} {:?} submit={:?}",
                field.id, field.label, field.value, field.width, field.submit
            );
        }
        Control::Choices(choices) => {
            let _ = writeln!(out, "choices {} {:?}", choices.id, choices.label);
            sections(out, &choices.content, depth + 1);
        }
        Control::Separator => out.push_str("---\n"),
    }
}

fn dropdown(out: &mut String, open: &Dropdown, depth: usize) {
    pad(out, depth);
    let _ = writeln!(
        out,
        "dropdown {} {:?} shows {}",
        open.id,
        open.label,
        face(&open.summary)
    );
    sections(out, &open.content, depth + 1);
}

fn sections(out: &mut String, content: &[DropdownSection], depth: usize) {
    for section in content {
        pad(out, depth);
        match section {
            DropdownSection::Options { layout, options } => {
                let _ = writeln!(out, "options {}", layout_name(*layout));
                for option in options {
                    pad(out, depth + 1);
                    let trailing = option
                        .trailing
                        .as_ref()
                        .map_or_else(String::new, |text| format!(" trailing={text:?}"));
                    let repeats = option.face.icon.is_none()
                        && option.face.font.is_none()
                        && option.face.text.as_deref() == Some(&*option.label);
                    let shown = if repeats {
                        String::new()
                    } else {
                        format!(" {}", face(&option.face))
                    };
                    let _ = writeln!(
                        out,
                        "option {} {} {:?}{shown}{trailing}{}{} -> {:?}",
                        mark(option.selected),
                        option.id,
                        option.label,
                        chord(option.chord),
                        if option.enabled { "" } else { " disabled" },
                        option.action
                    );
                }
            }
            DropdownSection::Controls(row) => {
                out.push_str("controls\n");
                controls(out, row, depth + 1);
            }
        }
    }
}

fn layout_name(layout: OptionLayout) -> String {
    match layout {
        OptionLayout::List => "list".to_owned(),
        OptionLayout::Row => "row".to_owned(),
        OptionLayout::Grid { columns } => format!("grid({columns})"),
    }
}

fn face(face: &Face) -> String {
    let mut parts = Vec::new();
    if let Some(icon) = face.icon {
        parts.push(format!("icon={icon:?}"));
    }
    if let Some(text) = &face.text {
        parts.push(format!("text={text:?}"));
    }
    if let Some(color) = &face.color {
        parts.push(format!("color={}", color.as_str()));
    }
    if let Some(font) = face.font {
        parts.push(format!("font={font:?}"));
    }
    if parts.is_empty() {
        "hollow".to_owned()
    } else {
        parts.join(" ")
    }
}

fn chord(chord: Option<Chord>) -> String {
    let Some(chord) = chord else {
        return String::new();
    };
    let key = match chord.key {
        Key::Char(character) => character.to_string(),
        other => format!("{other:?}").to_lowercase(),
    };
    let mut parts = Vec::new();
    for (held, name) in [
        (chord.cmd, "cmd"),
        (chord.shift, "shift"),
        (chord.alt, "alt"),
    ] {
        if held {
            parts.push(name.to_owned());
        }
    }
    parts.push(key);
    format!(" chord={}", parts.join("+"))
}

/// Asserts the toolbar of a [`TestApp`](crate::TestApp) against an inline
/// snapshot.
#[macro_export]
macro_rules! assert_toolbar_snapshot {
    ($app:expr, $($rest:tt)*) => {
        $crate::insta::assert_snapshot!($app.toolbar_snapshot(), $($rest)*)
    };
}

/// Asserts what the dock shows for a [`TestApp`](crate::TestApp) against
/// an inline snapshot, `none` for an empty dock.
#[macro_export]
macro_rules! assert_popup_snapshot {
    ($app:expr, $($rest:tt)*) => {
        $crate::insta::assert_snapshot!($app.popup_snapshot(), $($rest)*)
    };
}
