//! Builders the popups share: buttons, color rows, and the dropdowns that
//! several popups have a copy of.

use specular_doc::{Color, ColorPreset, TextFont};

use super::{
    Button, Control, ControlId, Dropdown, DropdownOption, DropdownSection, Face, Label,
    OptionLayout, PaintRole, Palette, Stepper, Swatch, Swatches, Toggle,
};
use crate::{Action, binding_of};

/// A text size the popup offers by name.
const SIZE_PRESETS: [(&str, f64); 5] = [
    ("Small", 14.0),
    ("Medium", 32.0),
    ("Large", 56.0),
    ("Extra large", 96.0),
    ("Huge", 144.0),
];
/// The size a text without one is drawn at.
const DEFAULT_SIZE: f64 = 14.0;
/// The smallest and largest text size.
const SIZE_RANGE: (f64, f64) = (8.0, 256.0);

/// The typefaces, in dropdown order.
const FONTS: [(&str, &str, TextFont); 3] = [
    ("sans", "Sans", TextFont::Sans),
    ("mono", "Mono", TextFont::Mono),
    ("hand", "Hand", TextFont::Hand),
];

/// The eight color slots in popup order. Blue is the Specular extension the
/// spec has no preset number for.
fn slots() -> [(&'static str, &'static str, Color); 8] {
    [
        ("neutral", "Neutral", Color::Neutral),
        ("purple", "Purple", Color::Preset(ColorPreset::Purple)),
        ("blue", "Blue", Color::Custom("7".to_owned())),
        ("cyan", "Cyan", Color::Preset(ColorPreset::Cyan)),
        ("green", "Green", Color::Preset(ColorPreset::Green)),
        ("yellow", "Yellow", Color::Preset(ColorPreset::Yellow)),
        ("orange", "Orange", Color::Preset(ColorPreset::Orange)),
        ("red", "Red", Color::Preset(ColorPreset::Red)),
    ]
}

/// A button, with the key that does the same.
pub(super) fn button(
    id: ControlId,
    label: impl Into<Label>,
    face: Face,
    action: Action,
) -> Control {
    Control::Button(Button {
        id,
        label: label.into(),
        face,
        enabled: true,
        chord: binding_of(&action).map(|binding| binding.chord),
        action,
    })
}

/// A toggle that is `on`; `action` goes to the other state.
pub(super) fn toggle(
    id: ControlId,
    label: impl Into<Label>,
    face: Face,
    on: bool,
    action: Action,
) -> Toggle {
    Toggle {
        id,
        label: label.into(),
        face,
        on,
        enabled: true,
        chord: binding_of(&action).map(|binding| binding.chord),
        action,
    }
}

/// Joins groups of controls with a separator between each pair, leaving out
/// the empty ones.
pub(super) fn groups(groups: Vec<Vec<Control>>) -> Vec<Control> {
    let mut controls = Vec::new();
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !controls.is_empty() {
            controls.push(Control::Separator);
        }
        controls.extend(group);
    }
    controls
}

/// "shape" for one, "3 shapes" for more.
pub(super) fn noun(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        one.to_owned()
    } else {
        format!("{count} {many}")
    }
}

/// The eight slots as a row of swatches, in the hues of `palette`. `selected`
/// is the current color. `clear` adds a first choice that paints nothing:
/// whether it is current, and its action.
pub(super) fn swatches(
    id: ControlId,
    palette: Palette,
    role: PaintRole,
    selected: Option<&Color>,
    clear: Option<(bool, Action)>,
    pick: impl Fn(Color) -> Action,
) -> Swatches {
    let none = clear.map(|(selected, action)| Swatch {
        id: id.child("transparent"),
        label: "Transparent".into(),
        color: None,
        selected,
        action,
    });
    // A transparent fill is the current value, so no color is.
    let cleared = none.as_ref().is_some_and(|none| none.selected);
    let colors = slots().into_iter().map(|(name, label, color)| Swatch {
        id: id.child(name),
        label: label.into(),
        selected: !cleared && selected == Some(&color),
        action: pick(color.clone()),
        color: Some(color),
    });
    Swatches {
        options: none.into_iter().chain(colors).collect(),
        id,
        palette,
        role,
        enabled: true,
    }
}

/// A dropdown holding one row of swatches, closed it shows the current color.
pub(super) fn color_dropdown(
    id: ControlId,
    noun: &str,
    palette: Palette,
    role: PaintRole,
    selected: Option<&Color>,
    clear: Option<(bool, Action)>,
    pick: impl Fn(Color) -> Action,
) -> Control {
    let summary = match &clear {
        Some((true, _)) => Face::icon(super::Icon::Ban),
        Some((false, _)) | None => Face::dot(selected.cloned()),
    };
    let row = swatches(id.child("swatches"), palette, role, selected, clear, pick);
    Control::Dropdown(Dropdown {
        label: format!("Set {noun} color").into(),
        summary,
        content: vec![DropdownSection::Controls(vec![Control::Swatches(row)])],
        id,
    })
}

/// The size dropdown: named sizes, then a stepper for any other. `value` is
/// `None` when the selection has several sizes.
pub(super) fn size_dropdown(
    id: ControlId,
    label: &'static str,
    value: Option<f64>,
    set: impl Fn(f64) -> Action,
) -> Control {
    // Several sizes in one selection leave every choice unselected; the
    // stepper then starts from the default size.
    let shared = value;
    let value = value.unwrap_or(DEFAULT_SIZE);
    let named = shared
        .and_then(|shared| (SIZE_PRESETS.iter()).find(|(_, size)| (*size - shared).abs() < 0.5));
    let options = SIZE_PRESETS
        .iter()
        .map(|&(name, size)| DropdownOption {
            id: id.child(size),
            label: name.into(),
            face: Face::text(name),
            trailing: None,
            chord: None,
            selected: named.is_some_and(|(label, _)| *label == name),
            action: set(size),
        })
        .collect();
    let stepper = Stepper {
        id: id.child("custom"),
        label: "Custom text size in pixels".into(),
        value: format!("{value}").into(),
        decrement: set((value - 1.0).max(SIZE_RANGE.0)),
        can_decrement: value > SIZE_RANGE.0,
        increment: set((value + 1.0).min(SIZE_RANGE.1)),
        can_increment: value < SIZE_RANGE.1,
    };
    Control::Dropdown(Dropdown {
        summary: Face::text(match (shared, named) {
            (None, _) => "Mixed",
            (Some(_), None) => "Custom",
            (Some(_), Some((name, _))) => name,
        }),
        content: vec![
            DropdownSection::Options {
                layout: OptionLayout::List,
                options,
            },
            DropdownSection::Controls(vec![Control::Stepper(stepper)]),
        ],
        label: label.into(),
        id,
    })
}

/// The typeface dropdown.
pub(super) fn font_dropdown(
    id: ControlId,
    label: &'static str,
    value: Option<TextFont>,
    set: impl Fn(TextFont) -> Action,
) -> Control {
    let options = FONTS
        .iter()
        .map(|&(name, text, font)| DropdownOption {
            id: id.child(name),
            label: text.into(),
            face: Face::text(text).in_font(font),
            trailing: None,
            chord: None,
            selected: value == Some(font),
            action: set(font),
        })
        .collect();
    let shown = FONTS.iter().find(|(_, _, font)| Some(*font) == value);
    let summary = match shown {
        Some((_, text, font)) => Face::text(*text).in_font(*font),
        None => Face::text("Mixed"),
    };
    Control::Dropdown(Dropdown {
        summary,
        content: vec![DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        }],
        label: label.into(),
        id,
    })
}
