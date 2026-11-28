//! The composer's box without its owner: the pasted images, the field and
//! the row under it (the context pill, the folder, the send mode, the model
//! and Send). The right panel draws it for its open thread, and a mod's
//! `Composer` host component draws the same box for any thread.
//!
//! What a box does is handed in as callbacks, so it holds no state of its
//! own: the owner keeps the field and the images.

use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable as _, Icon, IconName, Sizable as _, h_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, ClipboardItem, Entity, InteractiveElement as _, IntoElement,
    ObjectFit, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    StyledImage as _, Window, div, img, px,
};
use specular_interact::Composer;

use super::chips::{auto_chip, chip, model_chip, muted, pill_icon, remove_button};
use super::paste::Pasted;
use crate::theme;

/// The field is two lines tall when empty and grows to six: `min-h-[48px]`
/// and `max-h-[160px]` at a 24 px line.
pub(crate) const MIN_ROWS: usize = 2;
pub(crate) const MAX_ROWS: usize = 6;
const LINE: f32 = 24.0;

/// Sends what the box holds.
pub(crate) type OnSend = Rc<dyn Fn(&mut Window, &mut App)>;
/// Takes away the pasted image at an index.
pub(crate) type OnRemove = Rc<dyn Fn(usize, &mut Window, &mut App)>;
/// Keeps the images of a paste; `true` when the paste is spent.
pub(crate) type OnPaste = Rc<dyn Fn(&ClipboardItem, &mut Window, &mut App) -> bool>;

/// What one composer box shows.
pub(crate) struct ComposerBox<'a> {
    /// What the next message goes with, and the model.
    pub(crate) model: &'a Composer,
    /// The field.
    pub(crate) input: &'a Entity<TextareaState>,
    /// The images waiting to go.
    pub(crate) images: &'a [Pasted],
    /// Whether there is something to send.
    pub(crate) ready: bool,
}

impl ComposerBox<'_> {
    /// Thumbnails of the pasted images, each removable (`PastedImages`).
    pub(crate) fn thumbnails(&self, on_remove: &OnRemove) -> Option<AnyElement> {
        if self.images.is_empty() {
            return None;
        }
        let thumbnails = (self.images.iter().enumerate()).map(|(index, pasted)| {
            let group = SharedString::from(format!("chat-pasted-{index}"));
            let on_remove = on_remove.clone();
            div()
                .group(group.clone())
                .relative()
                .size(px(48.0))
                .flex_shrink_0()
                .overflow_hidden()
                .rounded(crate::theme::corner(6.0))
                .border_1()
                .border_color(theme::solid(theme::zinc_300()))
                .child(
                    img(pasted.thumbnail.clone())
                        .size_full()
                        .object_fit(ObjectFit::Cover),
                )
                .child(
                    remove_button(
                        SharedString::from(format!("chat-pasted-remove-{index}")),
                        group,
                        0x0000_00b3,
                        0xffff_ffff,
                    )
                    .right(px(2.0))
                    .top(px(2.0))
                    .tooltip(|window, cx| Tooltip::new("Remove image").build(window, cx))
                    .on_click(move |_, window, cx| on_remove(index, window, cx)),
                )
        });
        let row = h_flex()
            .flex_wrap()
            .gap(px(6.0))
            .pb(px(6.0))
            .children(thumbnails);
        Some(row.into_any_element())
    }

    /// The field, which hands a paste to `on_paste` before it inserts text.
    pub(crate) fn field(&self, on_paste: OnPaste) -> AnyElement {
        let input = self.input.clone();
        // The smallest size has the smallest inset around the text; the type
        // is the composer's own.
        let field = Textarea::new(self.input)
            .xsmall()
            .appearance(false)
            .bordered(false)
            .text_size(px(12.0))
            .line_height(px(LINE))
            .on_paste(move |item, window, cx| on_paste(item, window, cx));
        div()
            .id("chat-field")
            .min_h(px(LINE * MIN_ROWS as f32))
            .p(px(2.0))
            .cursor_text()
            .on_click(move |_: &ClickEvent, window, cx| {
                input.update(cx, |state, cx| state.focus(window, cx));
            })
            .child(field)
            .into_any_element()
    }

    /// The round Send button: filled once there is something to send.
    fn send_button(&self, on_send: OnSend) -> AnyElement {
        let arrow = div().text_size(px(12.0)).child("↑");
        let button = Button::new("chat-send")
            .size(px(28.0))
            .rounded(crate::theme::corner(14.0))
            .flex_shrink_0()
            .tooltip("Send")
            .child(arrow)
            .on_click(move |_, window, cx| on_send(window, cx));
        if self.ready {
            button.primary().into_any_element()
        } else {
            button
                .ghost()
                .disabled(true)
                .bg(theme::solid(theme::zinc_100()))
                .text_color(muted())
                .into_any_element()
        }
    }

    /// The row under the field: what a message goes with, the folder it
    /// writes to, the send mode, the model, and Send.
    pub(crate) fn chips(&self, on_send: OnSend) -> AnyElement {
        let model = self.model;
        let folder = model.folder.as_ref().map(|folder| {
            let target = model.folder_path.as_ref().unwrap_or(folder);
            let written = SharedString::from(format!("Changes are written to {target}"));
            chip("chat-folder", Icon::new(IconName::FolderOpen), folder)
                .tooltip(move |window, cx| Tooltip::new(written.clone()).build(window, cx))
        });
        h_flex()
            .min_w_0()
            .gap_1()
            .pt(px(2.0))
            .items_center()
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .items_center()
                    .child(chip(
                        "chat-pill",
                        pill_icon(model.pill.kind),
                        &model.pill.label,
                    ))
                    .children(folder)
                    .children(model.auto.as_ref().map(auto_chip)),
            )
            .child(model_chip(&model.model, self.input))
            .child(self.send_button(on_send))
            .into_any_element()
    }
}

/// The rounded box the parts sit in.
pub(crate) fn frame(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    gpui_kit::component::v_flex()
        .id(id)
        .rounded(crate::theme::corner(16.0))
        .border_1()
        .border_color(theme::solid(theme::input_border()))
        .bg(theme::solid(theme::input()))
        .px_2()
        .py(px(6.0))
}
