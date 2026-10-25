//! The agent's row at the end of the transcript: the thread's avatar, at
//! rest, or tumbling at the head of the run bar, which says what the agent
//! is doing now, has a stop button, and opens to the run's log
//! (`AgentRunBar.tsx`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, Div, Entity, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _,
    WeakEntity, Window, div, px, relative, rgba,
};
use specular_interact::RunBar;

use super::avatar::ThreadAvatar;
use crate::theme;
use crate::view::{ShellView, run};

/// What the bar needs from the view.
#[derive(Clone)]
pub(super) struct RunBarView {
    /// Whether the log is open.
    pub(super) open: bool,
    /// The log's scroll, kept at its end as lines arrive.
    pub(super) scroll: ScrollHandle,
    /// The view that owns `open`.
    pub(super) view: WeakEntity<ShellView>,
    /// The thread's avatar, which leads the strip.
    pub(super) avatar: Option<Entity<ThreadAvatar>>,
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
        .text_color(theme::tinted(theme::text_muted()))
        .children(
            (run.log.iter()).map(|line| div().min_w_0().child(SharedString::from(line.clone()))),
        )
}

/// The row led by the avatar, the same whether the agent works or not, so
/// the cube holds still in place as a run starts and ends.
fn strip(bar: &RunBarView) -> Div {
    h_flex()
        .h(px(36.0))
        .w_full()
        .pr_2()
        .gap_1()
        .items_center()
        // The avatar's picture is wider than the cube at rest, for the
        // corners to swing out into: the cube lines up with the
        // transcript's edge and the picture overhangs the row.
        .children(bar.avatar.clone().map(|avatar| {
            div()
                .flex_shrink_0()
                .my(px(-2.0))
                .ml(px(-6.0))
                .mr_1()
                .child(avatar)
        }))
}

/// The agent's row: the avatar alone while it waits, and the bar for `run`
/// while it works.
pub(super) fn agent_row(run: Option<&RunBar>, bar: &RunBarView, cx: &App) -> AnyElement {
    match run {
        Some(run) => run_bar(run, bar, cx).into_any_element(),
        None => strip(bar).into_any_element(),
    }
}

fn run_bar(run_bar: &RunBar, bar: &RunBarView, cx: &App) -> impl IntoElement + use<> {
    let stop = run_bar.stop.clone();
    let (by_label, by_chevron) = (bar.view.clone(), bar.view.clone());
    let strip = strip(bar)
        .child(
            div()
                .id("chat-run-label")
                .flex_1()
                .min_w_0()
                .cursor_pointer()
                .on_click(move |_, window, cx| toggle(&by_label, window, cx))
                .child(
                    div()
                        .child(SharedString::from(run_bar.label.clone()))
                        .truncate()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(theme::label())),
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
        .child(strip)
        .content(log(run_bar, bar, cx))
}
