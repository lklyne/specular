//! The tab row: the first row of the chrome. It is the window's title bar,
//! with the traffic lights in its left padding, then a tab for the canvas
//! and one for each page and Document.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, BoxShadow, ClickEvent, Image, ImageFormat, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    div, img, linear_color_stop, linear_gradient, point, px, rgba,
};
use specular_interact::{Action, Button, ControlId, Event, Icon, ToolbarModel, ViewStrip, ViewTab};

use super::controls::element_id;
use super::glyphs::{glyph, ink};
use super::named::mark;
use super::run;
use super::toolbar::{panel_toggle, zoom};
use crate::{canvas, theme};

thread_local! {
    /// Whether a press on the bare strip is held and has not moved yet.
    static DRAG_ARMED: Cell<bool> = const { Cell::new(false) };
    /// The tab the button went down on, whose page is laid out for it
    /// until the click shows it or the button comes up somewhere else.
    static PRESSED: Cell<Option<ControlId>> = const { Cell::new(None) };
    /// Each tab's favicon as it was last given, with its image: GPUI caches
    /// a decoded image by its identity, so the same `Arc` must come back
    /// each frame.
    static FAVICONS: RefCell<HashMap<ControlId, Favicon>> = RefCell::new(HashMap::new());
}

/// A favicon's PNG as the model gave it, and the image made of it.
type Favicon = (Arc<[u8]>, Arc<Image>);

/// The button came up off the tab `id`: if its press prepared a page, the
/// page goes back to the size the canvas has it at.
fn release_off(id: &ControlId) {
    let pressed = PRESSED.take();
    if pressed.as_ref() == Some(id) {
        canvas::dispatch(Event::Action(Action::PrepareShow(None)));
    } else {
        PRESSED.set(pressed);
    }
}

/// The room the traffic lights take, `toolbarPaddingLeft` on macOS.
const TRAFFIC_LIGHTS: f32 = 86.0;
/// A tab: its width at rest, the least it shrinks to and its height.
const TAB: (f32, f32, f32) = (180.0, 28.0, 28.0);
/// How much of the row's far end stays bare, to drag the window by.
const BARE: f32 = 80.0;
const TAB_GLYPH: f32 = 14.0;
/// A tab's close button, and the glyph in it.
const CLOSE: f32 = 18.0;
const CLOSE_GLYPH: f32 = 12.0;
/// The width of the rim around the tab that is showing.
const RIM: f32 = 1.0;
/// How much of a label's far end fades out, where a long one is cut.
const FADE: f32 = 20.0;
/// The group a tab's face reads the pointer from: the whole tab.
const HOVERED: &str = "tab";

/// The shadow under the tab that is showing: two short ones, each drawn in
/// from the tab's sides so it shows below the tab and not around it.
fn lift() -> Vec<BoxShadow> {
    let layer = |drop: f32, blur: f32, color: u32| BoxShadow {
        color: theme::tinted(color),
        offset: point(px(0.0), px(drop)),
        blur_radius: px(blur),
        spread_radius: px(-drop),
        inset: false,
    };
    let (near, far) = theme::tab_shadow();
    vec![layer(1.0, 2.0, near), layer(2.0, 4.0, far)]
}

/// The glyph before a tab's label: the page's own icon when it has one.
fn lead(model: &ViewTab, text: u32) -> AnyElement {
    let Some(png) = &model.favicon else {
        return glyph(model.icon, ink(text), None, false, TAB_GLYPH).into_any_element();
    };
    let image = FAVICONS.with_borrow_mut(|favicons| {
        let kept = favicons
            .get(&model.id)
            .filter(|(bytes, _)| Arc::ptr_eq(bytes, png));
        if let Some((_, image)) = kept {
            return Arc::clone(image);
        }
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, png.to_vec()));
        favicons.insert(model.id.clone(), (Arc::clone(png), Arc::clone(&image)));
        image
    });
    img(image)
        .size(px(TAB_GLYPH))
        .flex_shrink_0()
        .rounded(px(3.0))
        .into_any_element()
}

/// A tab's label on one line. A long one is cut at the tab's edge and fades
/// into `fill`, the colour under it, which a hovered tab swaps for
/// `hovered`.
fn label(model: &ViewTab, fill: u32, hovered: Option<u32>) -> impl IntoElement {
    let fade = |fill: u32| {
        linear_gradient(
            90.0,
            linear_color_stop(rgba(fill & 0xFFFF_FF00), 0.0),
            linear_color_stop(rgba(fill), 1.0),
        )
    };
    div()
        .relative()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap()
        .child(SharedString::from(model.label.to_string()))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .w(px(FADE))
                .bg(fade(fill))
                .when_some(hovered, |this, hovered| {
                    this.group_hover(HOVERED, move |this| this.bg(fade(hovered)))
                }),
        )
}

/// A stretch of the row with nothing on it. The row is taller than the
/// title bar macOS drags and zooms by itself: here a press that moves drags
/// the window, and a double click does what the system setting says a
/// title bar's does.
fn bare_strip() -> impl IntoElement {
    h_flex()
        .id("tabs-bare")
        .flex_1()
        .flex_shrink_0()
        .min_w(px(BARE))
        .h_full()
        .on_mouse_down(MouseButton::Left, |_, _, _| DRAG_ARMED.set(true))
        .on_mouse_up(MouseButton::Left, |_, _, _| DRAG_ARMED.set(false))
        .on_mouse_down_out(|_, _, _| DRAG_ARMED.set(false))
        .on_mouse_move(|_, window, _| {
            if DRAG_ARMED.replace(false) {
                window.start_window_move();
            }
        })
        .on_click(|event: &ClickEvent, window, _| {
            if event.click_count() == 2 {
                window.titlebar_double_click();
            }
        })
}

/// The button at a tab's far end that closes it, there while the pointer is
/// over the tab. It sits on `fill`, the colour the tab has then, over the
/// end of the label. Its press is its own: the tab under it neither lays
/// its page out nor shows it.
fn close(id: &ControlId, action: Action, fill: u32, hover: u32) -> impl IntoElement {
    let id = id.child("close");
    h_flex()
        .id(element_id(&id))
        .invisible()
        .group_hover(HOVERED, gpui_kit::Styled::visible)
        .absolute()
        .right(px(RIM + 4.0))
        .top(px((TAB.2 - CLOSE) / 2.0))
        .size(px(CLOSE))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(theme::solid(fill))
        .hover(move |this| this.bg(theme::solid(hover)))
        .tooltip(crate::tip::view("Close tab".to_owned()))
        .child(glyph(
            Icon::Close,
            ink(theme::toolbar_text()),
            None,
            false,
            CLOSE_GLYPH,
        ))
        .child(mark(&id))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            run(&action, window, cx);
        })
}

/// One tab: its glyph and its label. The one showing is filled, rimmed and
/// lifted; the others are bare, in the muted text colour. A click takes some 80 ms from the button going
/// down to it coming up, and a page needs half of that to paint at the size
/// of its tab, so the press lays the page out and the click shows it.
fn tab(model: &ViewTab) -> impl IntoElement {
    let action = model.action.clone();
    let (pressed, released) = (model.id.clone(), model.id.clone());
    let text = if model.active {
        theme::toolbar_text_strong()
    } else {
        theme::toolbar_text()
    };
    // The fills are opaque, so the rim under the face shows only around it
    // and a long label has one colour to fade into.
    let (fill, hovered) = if model.active {
        (theme::tab_fill(), None)
    } else {
        (theme::toolbar(), Some(theme::tab_hover()))
    };
    // The face is inset by the rim's width, so the gradient under it shows
    // as a border: lit along the top, shaded along the bottom.
    let face = h_flex()
        .size_full()
        .px(px(6.0))
        .gap_1p5()
        .items_center()
        .overflow_hidden()
        .rounded(px(5.0))
        .when(model.active, |this| this.bg(theme::solid(fill)))
        .when_some(hovered, |this, hovered| {
            this.group_hover(HOVERED, move |this| this.bg(theme::solid(hovered)))
        })
        .child(lead(model, text))
        .child(label(model, fill, hovered));
    let (lit, shaded) = theme::tab_rim();
    h_flex()
        .id(element_id(&model.id))
        .group(HOVERED)
        .relative()
        .w(px(TAB.0))
        .min_w(px(TAB.1))
        .h(px(TAB.2))
        .flex_shrink(1.0)
        .p(px(RIM))
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(theme::solid(text))
        .when(model.active, |this| {
            this.shadow(lift()).bg(linear_gradient(
                180.0,
                linear_color_stop(rgba(lit), 0.0),
                linear_color_stop(rgba(shaded), 1.0),
            ))
        })
        .child(face)
        .child(mark(&model.id))
        .children(model.close.clone().map(|action| {
            let (under, over) = if model.active {
                (theme::tab_fill(), theme::tab_hover())
            } else {
                (theme::tab_hover(), theme::tab_fill())
            };
            close(&model.id, action, under, over)
        }))
        .when_some(model.prepare.clone(), |this, prepare| {
            this.on_mouse_down(MouseButton::Left, move |_, _, _| {
                PRESSED.set(Some(pressed.clone()));
                canvas::dispatch(Event::Action(prepare.clone()));
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, _| release_off(&released))
        })
        .on_click(move |_, window, cx| {
            PRESSED.set(None);
            run(&action, window, cx);
        })
}

/// The button after the last tab: a new page, shown in its own tab.
fn add(model: &Button) -> impl IntoElement {
    let action = model.action.clone();
    let text = theme::toolbar_text();
    h_flex()
        .id(element_id(&model.id))
        .relative()
        .size(px(TAB.2))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .cursor_pointer()
        .hover(|this| this.bg(theme::solid(theme::tab_hover())))
        .tooltip(crate::tip::view(model.label.to_string()))
        .children((model.face.icon).map(|icon| glyph(icon, ink(text), None, false, TAB_GLYPH)))
        .child(mark(&model.id))
        .on_click(move |_, window, cx| run(&action, window, cx))
}

/// The tab row of `model`. The tabs share what the traffic lights, the add
/// button and the bare strip leave, shrinking together; past their least
/// width the row cuts them off, so the strip is always there to drag the
/// window by. The zoom readout and the right panel's toggle, both of
/// `toolbar`, end the row.
pub(super) fn tabs(model: &ViewStrip, toolbar: &ToolbarModel) -> impl IntoElement {
    FAVICONS.with_borrow_mut(|favicons| {
        favicons.retain(|id, _| model.tabs.iter().any(|tab| tab.id == *id));
    });
    h_flex()
        .h(px(theme::TAB_ROW))
        .flex_shrink_0()
        .pl(px(TRAFFIC_LIGHTS))
        .items_center()
        .child(
            h_flex()
                .min_w_0()
                .flex_shrink(1.0)
                .gap_1()
                .items_center()
                .overflow_hidden()
                .children(model.tabs.iter().map(tab)),
        )
        .child(div().pl_1().flex_shrink_0().child(add(&model.add)))
        .child(bare_strip())
        .child(
            h_flex()
                .pr_3()
                .gap_1()
                .flex_shrink_0()
                .items_center()
                .text_color(theme::solid(theme::toolbar_text()))
                .children(zoom(toolbar))
                .children(toolbar.chat.as_ref().map(panel_toggle)),
        )
}
