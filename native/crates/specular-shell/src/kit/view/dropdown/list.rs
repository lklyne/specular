//! The sections of an open list: options, rows of fields, segmented choices and controls.

use std::rc::Rc;

use gpui_kit::component::separator::Separator;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use specular_interact::{Chord, Control, DropdownOption, DropdownSection, Icon, OptionLayout};

use super::super::controls::{CONTROL, element_id, face, in_list};
use super::super::glyphs::{glyph, ink};
use super::super::named::mark;
use super::super::{field, run};
use crate::theme;

/// The room above and below a field in a row of a list.
const FIELD_INSET: f32 = 3.0;

/// How a list's rows are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rows {
    /// Words marked with a check: every row in the full text colour, and
    /// the current one with a check at its far end.
    Menu,
    /// Rows with small text or a key at the far end: the current one
    /// filled, the others muted.
    Presets,
}

/// Which way `content` is drawn: a list with any far-end text or key is
/// presets, and any other a menu.
fn rows_of(content: &[DropdownSection]) -> Rows {
    let detailed = content.iter().any(|section| match section {
        DropdownSection::Options { options, .. } => {
            (options.iter()).any(|option| option.trailing.is_some() || option.chord.is_some())
        }
        DropdownSection::Controls(_) => false,
    });
    if detailed { Rows::Presets } else { Rows::Menu }
}

/// The fields of `model` at the far end of its row, or nothing for a row
/// with none. They keep their room and are drawn while the row is the
/// current value, is under the pointer, or one of them is being typed in.
fn row_fields(model: &DropdownOption, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    if model.fields.is_empty() {
        return None;
    }
    let typing = (model.fields.iter()).any(|one| field::is_focused(&one.id, window, cx));
    let shown = model.selected || typing;
    let fields: Vec<AnyElement> = (model.fields.iter())
        .map(|one| field::field(one, window, cx))
        .collect();
    Some(
        h_flex()
            .gap_2()
            .items_center()
            // A press in a field is typing, and not a choice of the row,
            // which would hand the keys back to the canvas.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when(!shown, |this| {
                this.invisible()
                    .group_hover(element_id(&model.id), gpui_kit::Styled::visible)
            })
            .children(fields)
            .into_any_element(),
    )
}

/// One choice of a dropdown: a row of a list, with its small text or key at
/// the far end, or a cell of a row or grid of glyphs.
fn option(
    model: &DropdownOption,
    list: Option<Rows>,
    dismiss: Dismiss,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let action = model.action.clone();
    let wide = list.is_some();
    let menu = list == Some(Rows::Menu);
    let height = match (list, model.face.font) {
        (None, _) => 32.0,
        (Some(_), Some(_)) => 28.0,
        (Some(_), None) => CONTROL,
    };
    let trailing = (model.trailing.as_ref()).map(|text| SharedString::from(text.to_string()));
    let keys = model.chord.map(Chord::text);
    let closes = model.closes();
    let fields = row_fields(model, window, cx);
    div()
        .id(element_id(&model.id))
        .group(element_id(&model.id))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .h(px(height))
        .when(wide, |this| this.w_full().px_2())
        // Fields end a row as far from its edge as they are from its top.
        .when(fields.is_some(), |this| this.pr(px(FIELD_INSET)))
        .when(!wide, |this| this.w(px(32.0)).justify_center())
        .rounded(px(6.0))
        .when(model.selected && !menu, |this| {
            this.bg(theme::solid(theme::control_on()))
        })
        .when(!model.enabled, |this| this.opacity(0.4))
        .when(model.enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(theme::solid(theme::control_hover())))
                .on_click(move |_, window, cx| {
                    run(&action, window, cx);
                    if closes {
                        dismiss(window, cx);
                    }
                })
        })
        .child(face(&model.face, model.selected || menu))
        .child(mark(&model.id))
        .when(model.selected && menu, |this| {
            this.child(glyph(Icon::Check, ink(theme::text()), None, false, 12.0))
        })
        .when_some(trailing, |this, text| {
            this.child(
                div()
                    .text_color(theme::tinted(theme::text_muted()))
                    .child(text),
            )
        })
        .children(fields)
        .when_some(keys, |this, keys| {
            this.child(
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(4.0))
                    .bg(theme::solid(theme::control_on()))
                    .text_color(theme::solid(theme::key_text()))
                    .child(SharedString::from(keys)),
            )
        })
        .into_any_element()
}

/// A line of a list that sets one thing: `title`, and at the far end the
/// values it can have as the segments of one Kit control.
fn segmented(title: &'static str, options: &[DropdownOption]) -> AnyElement {
    let actions: Vec<_> = (options.iter())
        .map(|option| option.action.clone())
        .collect();
    let tabs: Vec<Tab> = (options.iter())
        .map(|option| {
            let word = option.face.text.as_ref().map(ToString::to_string);
            Tab::new()
                .label(word.unwrap_or_default())
                .disabled(!option.enabled)
                .child(mark(&option.id))
        })
        .collect();
    let bar = TabBar::new(SharedString::from(format!("segmented.{title}")))
        .segmented()
        .xsmall()
        .children(tabs)
        .on_click(move |index, window, cx| {
            if let Some(action) = actions.get(*index) {
                run(action, window, cx);
            }
        });
    // No segment is on while the pages chosen differ.
    let bar = match options.iter().position(|option| option.selected) {
        Some(index) => bar.selected_index(index),
        None => bar,
    };
    h_flex()
        .w_full()
        .h(px(CONTROL))
        .pl_2()
        .gap_4()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_color(theme::tinted(theme::text_muted()))
                .child(title),
        )
        .child(bar)
        .into_any_element()
}

/// Closes the list a chosen option was in.
pub(super) type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

fn section(
    model: &DropdownSection,
    rows: Rows,
    dismiss: &Dismiss,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    match model {
        DropdownSection::Options { layout, options } => match layout {
            OptionLayout::List => v_flex()
                .gap_0p5()
                .min_w(px(140.0))
                .children(
                    (options.iter())
                        .map(|one| option(one, Some(rows), dismiss.clone(), window, cx))
                        .collect::<Vec<_>>(),
                )
                .into_any_element(),
            OptionLayout::Row => h_flex()
                .gap_0p5()
                .children(
                    (options.iter())
                        .map(|one| option(one, None, dismiss.clone(), window, cx))
                        .collect::<Vec<_>>(),
                )
                .into_any_element(),
            OptionLayout::Segmented { title } => segmented(title, options),
            OptionLayout::Grid { columns } => {
                let width = f32::from(*columns) * 32.0 + f32::from(columns.saturating_sub(1)) * 2.0;
                h_flex()
                    .flex_wrap()
                    .gap(px(2.0))
                    .w(px(width))
                    .children(
                        (options.iter())
                            .map(|one| option(one, None, dismiss.clone(), window, cx))
                            .collect::<Vec<_>>(),
                    )
                    .into_any_element()
            }
        },
        DropdownSection::Controls(controls) => {
            let controls: Vec<AnyElement> = (controls.iter())
                .map(|model| in_list(model, window, cx))
                .collect();
            h_flex()
                .gap_1()
                .items_center()
                .children(controls)
                .into_any_element()
        }
    }
}

/// Sections one under another with a line between them, as an open list
/// has them.
pub(super) fn sections(
    content: &[DropdownSection],
    dismiss: &Dismiss,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let rows = rows_of(content);
    let segmented = |section: Option<&DropdownSection>| {
        matches!(
            section,
            Some(DropdownSection::Options {
                layout: OptionLayout::Segmented { .. },
                ..
            })
        )
    };
    // A row that opens more of a list is the end of the rows over it.
    let opens_more = |section: Option<&DropdownSection>| match section {
        Some(DropdownSection::Controls(controls)) => {
            (controls.iter()).all(|control| matches!(control, Control::Dropdown(_)))
        }
        Some(DropdownSection::Options { .. }) | None => false,
    };
    let blocks: Vec<AnyElement> = (content.iter().enumerate())
        .map(|(index, one)| {
            let next = content.get(index + 1);
            // Lines that each set one thing read as one block.
            let joined = (segmented(Some(one)) && segmented(next)) || opens_more(next);
            let ruled = next.is_some() && !joined;
            v_flex()
                .gap_1()
                .child(section(one, rows, dismiss, window, cx))
                .when(ruled, |this| this.child(Separator::horizontal()))
                .into_any_element()
        })
        .collect();
    v_flex()
        .gap_1()
        .text_size(px(12.0))
        .children(blocks)
        .into_any_element()
}
