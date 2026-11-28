//! The layout's tree in the window (ADR 0059): each node it shows, in the
//! box `specular_interact::place` gives it, painted as the layout says and
//! holding its block's content.
//!
//! The canvas is seen through a slot from the window's top-left corner to
//! the canvas block's bottom-right one; every other block lies over that
//! slot or beside it, and takes the clicks it gets. A block with a
//! `material` holds the system's glass or blur under its paint
//! (`material.rs`), and the canvas under it shows through.

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, Styled as _, Window, div, px,
};
use specular_interact::{Content, Frame, Node, Region, place};

use super::{ShellView, material, tabs, toolbar};
use crate::canvas::Models;
use crate::paint::{hsla, paint};
use crate::theme;

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

/// The box `frame`, painted as `node` says.
fn painted(node: &Node, frame: Frame) -> gpui_kit::Div {
    let mut element = div()
        .absolute()
        .left(px(frame.x))
        .top(px(frame.y))
        .w(px(frame.width))
        .h(px(frame.height));
    element = paint(
        element,
        node.background.as_ref(),
        (frame.width, frame.height),
    );
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

/// The layout's line round the whole window, over every block, rounded to
/// `radius` so it follows the edge macOS cuts the window's corners on.
pub(super) fn window_border(models: &Models, radius: f32) -> Option<AnyElement> {
    let border = models.layout.look.window_border.as_ref()?;
    let color = hsla(&border.color)?;
    Some(
        div()
            .absolute()
            .inset_0()
            .rounded(px(radius))
            .border(px(border.width))
            .border_color(color)
            .into_any_element(),
    )
}

/// The window's background where the canvas is not: the four bands of
/// `window` round `canvas`, painted with the layout's `background`.
pub(super) fn backdrop(models: &Models, window: Frame, canvas: Frame) -> Vec<AnyElement> {
    let Some(fill) = models.layout.look.background.as_ref() else {
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
            paint(
                div()
                    .absolute()
                    .left(px(band.x))
                    .top(px(band.y))
                    .w(px(band.width))
                    .h(px(band.height)),
                Some(fill),
                (band.width, band.height),
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
                None if node.background.is_none()
                    && node.border.is_none()
                    && node.material.is_none() =>
                {
                    continue;
                }
                None => None,
                Some(Content::Mod { name, .. }) => {
                    let id = node.id.as_deref().unwrap_or(name);
                    self.mods.copy(id, cx)
                }
                Some(Content::Panel(region)) => material::built_on(node.material.is_some(), || {
                    self.panel(*region, models, zoom_in_tabs, window, cx)
                }),
            };
            let name = node.id.as_deref().unwrap_or("anonymous");
            let surface = (node.material).map(|kind| {
                material::material(format!("material-{name}"), kind, node.radius.unwrap_or(0.0))
            });
            out.push(
                painted(node, *frame)
                    .id(SharedString::from(format!("block-{name}")))
                    .when(
                        node.content.is_some() || node.material.is_some(),
                        gpui_kit::Stateful::<gpui_kit::Div>::occlude,
                    )
                    .children(surface)
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
                    .bg(super::material::panel_fill(theme::toolbar()))
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
