//! The run bar: one rounded strip that says what the agent is doing now,
//! with a stop button, and opens to the run's log (`AgentRunBar.tsx`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::shimmer::ShimmerText;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div,
    linear_color_stop, linear_gradient, px, relative, rgba,
};
use specular_interact::RunBar;

use crate::theme;
use crate::view::{ShellView, run};

/// The strip's wash: the ends of `GrainGradient`'s palette under its white
/// veil. The grain and the drift are a shader this renderer does not have.
const WASH_FROM: u32 = 0xf0d9_ffff;
const WASH_TO: u32 = 0xffe2_c2ff;
/// The label at rest and under the shimmer: black at 55% and black.
const LABEL: u32 = 0x0000_008c;
const LABEL_BRIGHT: u32 = 0x0000_00ff;

/// What the bar needs from the view.
#[derive(Clone)]
pub(super) struct RunBarView {
    /// Whether the log is open.
    pub(super) open: bool,
    /// The log's scroll, kept at its end as lines arrive.
    pub(super) scroll: ScrollHandle,
    /// The view that owns `open`.
    pub(super) view: WeakEntity<ShellView>,
}

fn toggle(view: &WeakEntity<ShellView>, _: &mut Window, cx: &mut App) {
    let _ = view.update(cx, |this, cx| {
        this.chat.log_open = !this.chat.log_open;
        cx.notify();
    });
}

fn log(run: &RunBar, bar: &RunBarView, cx: &App) -> impl IntoElement + use<> {
    v_flex()
        .id("chat-run-log")
        .max_h(px(160.0))
        .overflow_y_scroll()
        .track_scroll(&bar.scroll)
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
pub(super) fn run_bar(run_bar: &RunBar, bar: &RunBarView, cx: &App) -> impl IntoElement + use<> {
    let stop = run_bar.stop.clone();
    let (by_label, by_chevron) = (bar.view.clone(), bar.view.clone());
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
        .when(!bar.open, |this| this.rounded_b(px(15.0)))
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
                .on_click(move |_, window, cx| toggle(&by_label, window, cx))
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
                .tooltip(if bar.open { "Hide log" } else { "Show log" })
                .child(
                    Icon::new(if bar.open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .size(px(12.0)),
                )
                .on_click(move |_, window, cx| toggle(&by_chevron, window, cx)),
        );
    Collapsible::new()
        .open(bar.open)
        .overflow_hidden()
        .rounded(px(16.0))
        .border_1()
        .border_color(theme::solid(theme::INPUT_BORDER))
        .bg(theme::solid(theme::INPUT))
        .child(strip)
        .content(log(run_bar, bar, cx))
}
