//! The layout's tree in the window (ADR 0059): each node it shows, in the
//! box `specular_interact::place` gives it, painted as the layout says and
//! holding its block's content.
//!
//! The canvas is seen through a slot from the window's top-left corner to
//! the canvas block's bottom-right one; every other block lies over that
//! slot or beside it, and takes the clicks it gets.

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Styled as _, Window, div, linear_color_stop, linear_gradient, px,
};
use serde_json::Value;
use specular_interact::{Content, Frame, Node, Region, place};

use super::{ShellView, tabs, toolbar};
use crate::canvas::Models;
use crate::{look, theme};

/// Every node shown in a window `size` points across, and where.
pub(super) fn boxes(models: &Models, size: (f32, f32)) -> Vec<(Node, Frame)> {
    place(
        &models.layout,
        &models.toggles,
        models.shown,
        Frame::sized(size.0, size.1),
    )
    .into_iter()
    .map(|placed| (placed.node.clone(), placed.frame))
    .collect()
}

/// The canvas block's box among `boxes`.
pub(super) fn canvas_frame(boxes: &[(Node, Frame)]) -> Frame {
    (boxes.iter())
        .find(|(node, _)| node.content == Some(Content::Canvas))
        .map_or_else(Frame::default, |(_, frame)| *frame)
}

fn hsla(text: &str) -> Option<Hsla> {
    look::color(text).map(theme::solid)
}

/// `element` painted with `background`: a colour, or a gradient
/// `{"gradient": [from, to], "angle": deg}`.
fn background<E: gpui_kit::Styled>(element: E, background: Option<&Value>) -> E {
    match background {
        Some(Value::String(color)) => match hsla(color) {
            Some(color) => element.bg(color),
            None => element,
        },
        Some(Value::Object(fields)) => {
            let stops: Vec<Hsla> = (fields.get("gradient").and_then(Value::as_array))
                .into_iter()
                .flatten()
                .filter_map(|stop| hsla(stop.as_str()?))
                .collect();
            let angle = fields.get("angle").and_then(Value::as_f64).unwrap_or(180.0) as f32;
            match stops.as_slice() {
                [from, to, ..] => element.bg(linear_gradient(
                    angle,
                    linear_color_stop(*from, 0.0),
                    linear_color_stop(*to, 1.0),
                )),
                _ => element,
            }
        }
        _ => element,
    }
}

/// The box `frame`, painted as `node` says.
fn painted(node: &Node, frame: Frame) -> gpui_kit::Div {
    let mut element = div()
        .absolute()
        .left(px(frame.x))
        .top(px(frame.y))
        .w(px(frame.width))
        .h(px(frame.height));
    element = background(element, node.background.as_ref());
    if let Some(border) = &node.border
        && let Some(color) = hsla(&border.color)
    {
        element = element.border(px(border.width)).border_color(color);
    }
    if let Some(radius) = node.radius {
        element = element.rounded(px(radius)).overflow_hidden();
    }
    element
}

/// The window's background where the canvas is not: the four bands of
/// `window` round `canvas`, painted with the layout's `background`.
pub(super) fn backdrop(models: &Models, window: Frame, canvas: Frame) -> Vec<AnyElement> {
    let Some(paint) = models.layout.look.background.as_ref() else {
        return Vec::new();
    };
    let bands = [
        Frame::sized(window.width, canvas.y),
        Frame {
            x: 0.0,
            y: canvas.bottom(),
            width: window.width,
            height: window.height - canvas.bottom(),
        },
        Frame {
            x: 0.0,
            y: canvas.y,
            width: canvas.x,
            height: canvas.height,
        },
        Frame {
            x: canvas.right(),
            y: canvas.y,
            width: window.width - canvas.right(),
            height: canvas.height,
        },
    ];
    (bands.into_iter())
        .filter(|band| band.width > 0.0 && band.height > 0.0)
        .map(|band| {
            background(
                div()
                    .absolute()
                    .left(px(band.x))
                    .top(px(band.y))
                    .w(px(band.width))
                    .h(px(band.height)),
                Some(paint),
            )
            .into_any_element()
        })
        .collect()
}

impl ShellView {
    /// Every node of the tree but the canvas, painted and filled, in paint
    /// order: parents under their children.
    pub(super) fn blocks(
        &mut self,
        models: &Models,
        boxes: &[(Node, Frame)],
        zoom_in_tabs: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Vec<AnyElement> {
        let mut out = Vec::new();
        for (node, frame) in boxes {
            let content = match &node.content {
                Some(Content::Canvas) => continue,
                None if node.background.is_none() && node.border.is_none() => continue,
                None => None,
                Some(Content::Mod { name, .. }) => {
                    let id = node.id.as_deref().unwrap_or(name);
                    self.mods.copy(id, cx)
                }
                Some(Content::Panel(region)) => {
                    self.panel(*region, models, zoom_in_tabs, window, cx)
                }
            };
            let id = SharedString::from(format!(
                "block-{}",
                node.id.as_deref().unwrap_or("anonymous")
            ));
            out.push(
                painted(node, *frame)
                    .id(id)
                    .when(
                        node.content.is_some(),
                        gpui_kit::Stateful::<gpui_kit::Div>::occlude,
                    )
                    .children(content)
                    .into_any_element(),
            );
            if node.is("tabs") {
                out.push(hairline(*frame).into_any_element());
            }
        }
        out
    }

    /// The app's own panel for `region`, to fill its block.
    fn panel(
        &mut self,
        region: Region,
        models: &Models,
        zoom_in_tabs: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<AnyElement> {
        match region {
            Region::Tabs => Some(
                div()
                    .size_full()
                    .bg(theme::solid(theme::toolbar()))
                    .child(tabs::tabs(&models.strip, &models.toolbar, zoom_in_tabs))
                    .into_any_element(),
            ),
            Region::Sidebar => models
                .sidebar
                .visible
                .then(|| Self::sidebar(&models.sidebar, window, cx).into_any_element()),
            Region::Toolbar => Some(
                toolbar::docked(&models.toolbar, models.tool_options.as_ref(), window, cx)
                    .into_any_element(),
            ),
            Region::Chat => self
                .chat_panel(&models.chat, window, cx)
                .map(gpui_kit::IntoElement::into_any_element),
            Region::Dock => None,
        }
    }
}

/// The hairline under the tab row's box, drawn over what lies under it.
fn hairline(tabs: Frame) -> impl gpui_kit::IntoElement {
    div()
        .absolute()
        .top(px(tabs.bottom() - 1.0))
        .left(px(tabs.x))
        .w(px(tabs.width))
        .h(px(1.0))
        .bg(theme::solid(theme::toolbar_border()))
}
