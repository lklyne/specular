//! The run bar: one rounded strip that says what the agent is doing now,
//! with a stop button, and opens to the run's log (`AgentRunBar.tsx`).

use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::shimmer::ShimmerText;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ClickEvent, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    linear_color_stop, linear_gradient, px, relative, rgba,
};
use specular_interact::RunBar;

use crate::models::same;
use crate::theme;
use crate::ui_stats::{self, Part};
use crate::view::run;

/// The strip's wash: the ends of `GrainGradient`'s palette under its white
/// veil. The grain and the drift are a shader this renderer does not have.
const WASH_FROM: u32 = 0xf0d9_ffff;
const WASH_TO: u32 = 0xffe2_c2ff;
/// The label at rest and under the shimmer: black at 55% and black.
const LABEL: u32 = 0x0000_008c;
const LABEL_BRIGHT: u32 = 0x0000_00ff;

/// The run bar, a view of its own: the shimmer on its label asks for a
/// frame every refresh, and a frame it asks for builds this view and what
/// lays it out, and neither the conversation nor the sidebar.
pub(super) struct RunBarView {
    run: Option<Rc<RunBar>>,
    /// Whether the log is open.
    open: bool,
    /// The log's scroll, kept at its end as lines arrive.
    scroll: ScrollHandle,
}

impl RunBarView {
    pub(super) fn new() -> Self {
        Self {
            run: None,
            open: false,
            scroll: ScrollHandle::new(),
        }
    }

    /// Whether the agent is at work, so there is a bar to show.
    pub(super) fn is_running(&self) -> bool {
        self.run.is_some()
    }

    /// Shows `run`, and draws again if it is not the one shown.
    pub(super) fn set(&mut self, run: Option<&Rc<RunBar>>, cx: &mut Context<'_, Self>) {
        if same(self.run.as_ref(), run) {
            return;
        }
        let lines = |run: Option<&Rc<RunBar>>| run.map_or(0, |run| run.log.len());
        if lines(run) != lines(self.run.as_ref()) {
            self.scroll.scroll_to_bottom();
        }
        self.run = run.cloned();
        cx.notify();
    }
}

impl Render for RunBarView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        ui_stats::built(Part::RunBar);
        let Some(run) = self.run.as_deref() else {
            return div().into_any_element();
        };
        // `px-3` beside it as the conversation's rows have, and `pb-2`
        // between it and the composer.
        div()
            .px_3()
            .pb_2()
            .child(run_bar(run, self.open, &self.scroll, cx))
            .into_any_element()
    }
}

fn log(run: &RunBar, scroll: &ScrollHandle, cx: &App) -> impl IntoElement + use<> {
    v_flex()
        .id("chat-run-log")
        .max_h(px(160.0))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .px_3()
        .py_2()
        .gap_1()
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(11.0))
        .line_height(relative(1.625))
        .text_color(theme::tinted(theme::TEXT_MUTED))
        .children(
            (run.log.iter()).map(|line| div().min_w_0().child(SharedString::from(line.clone()))),
        )
}

/// The bar for `run`.
fn run_bar(
    run_bar: &RunBar,
    open: bool,
    scroll: &ScrollHandle,
    cx: &mut Context<'_, RunBarView>,
) -> impl IntoElement + use<> {
    let stop = run_bar.stop.clone();
    let toggle = |this: &mut RunBarView,
                  _: &ClickEvent,
                  _: &mut Window,
                  cx: &mut Context<'_, RunBarView>| {
        this.open = !this.open;
        cx.notify();
    };
    let strip = h_flex()
        .h(px(36.0))
        .w_full()
        .pl_3()
        .pr_2()
        .gap_1()
        .items_center()
        // A clip is a rectangle, so the wash rounds its own corners inside
        // the bar's edge.
        .rounded_t(px(15.0))
        .when(!open, |this| this.rounded_b(px(15.0)))
        .bg(linear_gradient(
            90.0,
            linear_color_stop(rgba(WASH_FROM), 0.0),
            linear_color_stop(rgba(WASH_TO), 1.0),
        ))
        .child(
            div()
                .id("chat-run-label")
                .flex_1()
                .min_w_0()
                .cursor_pointer()
                .on_click(cx.listener(toggle))
                .child(
                    ShimmerText::new(SharedString::from(run_bar.label.clone()))
                        .id("chat-run-shimmer")
                        .highlight_color(rgba(LABEL_BRIGHT))
                        .truncate()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(LABEL)),
                ),
        )
        .child(
            Button::new("chat-run-stop")
                .ghost()
                .xsmall()
                .tooltip("Stop")
                .child(Icon::new(IconName::Square).size(px(11.0)))
                .on_click(move |_, window, cx| run(&stop, window, cx)),
        )
        .child(
            Button::new("chat-run-toggle")
                .ghost()
                .xsmall()
                .tooltip(if open { "Hide log" } else { "Show log" })
                .child(
                    Icon::new(if open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .size(px(12.0)),
                )
                .on_click(cx.listener(toggle)),
        );
    Collapsible::new()
        .open(open)
        .overflow_hidden()
        .rounded(px(16.0))
        .border_1()
        .border_color(theme::solid(theme::INPUT_BORDER))
        .bg(theme::solid(theme::INPUT))
        .child(strip)
        .content(log(run_bar, scroll, cx))
}
