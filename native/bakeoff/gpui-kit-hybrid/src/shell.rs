//! The GPUI Kit window content: a toolbar, a resizable sidebar and the slot
//! the canvas view sits in.

use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::resizable::{h_resizable, resizable_panel};
use gpui_kit::component::sidebar::{
    Sidebar, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem,
};
use gpui_kit::component::{
    ActiveTheme as _, IconName, Selectable as _, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use specular_core::{ImeEvent, InputEvent};
use specular_interact::{Action, ToolbarSection};

use crate::canvas::{self, Drive};
use crate::log::{self, Pacing};
use crate::native::Order;

/// When set, GPUI's input-method calls are passed on to the focused page.
pub static FORWARD_IME: AtomicBool = AtomicBool::new(false);

actions!(spike, [Duplicate, Delete, BringToFront]);

/// How the canvas view relates to GPUI's view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub order: Order,
    pub drive: Drive,
}

pub struct Shell {
    pub focus: FocusHandle,
    layout: Layout,
    /// Redraws GPUI every frame, so its pacing can be measured.
    pub animate: bool,
    pub pacing: Pacing,
    started: Instant,
    marked: String,
    text: String,
}

impl Shell {
    pub fn new(layout: Layout, animate: bool, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            layout,
            animate,
            pacing: Pacing::default(),
            started: Instant::now(),
            marked: String::new(),
            text: String::new(),
        }
    }

    pub fn open_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog
                .title("Dialog over the canvas")
                .child("A GPUI Kit dialog. Its scrim and panel must cover the canvas view.")
        });
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.started.elapsed().as_secs_f32();
        let bar = 60.0 + 80.0 * (0.5 + 0.5 * (t * 4.0).sin());
        h_flex()
            .h(px(44.))
            .flex_shrink_0()
            .px_3()
            .gap_2()
            .items_center()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w(px(300.)).child("GPUI Kit toolbar"))
            .child(
                div().w(px(110.)).child(
                    Button::new("dialog")
                        .label("Dialog")
                        .on_click(cx.listener(|this, _, window, cx| this.open_dialog(window, cx))),
                ),
            )
            .child(
                div()
                    .w(px(110.))
                    .child(
                        Button::new("menu")
                            .label("Menu")
                            .dropdown_menu(|menu, _, _| {
                                menu.item(PopupMenuItem::new("Zoom to fit"))
                                    .item(PopupMenuItem::new("Zoom to selection"))
                                    .separator()
                                    .item(PopupMenuItem::new("Show grid"))
                                    .item(PopupMenuItem::new("Show comments"))
                                    .item(PopupMenuItem::new("Preferences"))
                            }),
                    ),
            )
            .child(
                div().w(px(110.)).child(
                    Popover::new("popover")
                        .trigger(Button::new("popover-trigger").label("Popover"))
                        .content(|_, _, cx| {
                            v_flex()
                                .w(px(260.))
                                .h(px(150.))
                                .gap_2()
                                .child("A GPUI Kit popover.")
                                .child(
                                    div()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Shadow and rounded corners blend with the canvas."),
                                )
                        }),
                ),
            )
            .child(
                div()
                    .w(px(bar))
                    .h(px(8.))
                    .rounded_sm()
                    .bg(cx.theme().primary),
            )
    }

    /// The sidebar, built from `specular_interact`'s pure models: the
    /// canvas's rows from `SidebarModel`, the tools from `ToolbarModel`.
    /// A click runs the model's own `Action` through `update`.
    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, tools) = canvas::with(|canvas| {
            let app = canvas.app().app();
            let sidebar = specular_interact::sidebar(app);
            let rows: Vec<_> = (sidebar.pages.iter().chain(&sidebar.notes))
                .take(6)
                .map(|row| (row.label.clone(), row.selected, row.action.clone()))
                .collect();
            let tools: Vec<_> = (specular_interact::toolbar(app).sections.iter())
                .filter_map(|section| match section {
                    ToolbarSection::Tools(tools) => Some(tools),
                    ToolbarSection::Zoom(_) => None,
                })
                .flatten()
                .map(|tool| (tool.label.to_string(), tool.active, tool.action.clone()))
                .collect();
            (rows, tools)
        })
        .unwrap_or_default();
        let run = |action: Action| {
            cx.listener(move |_, _: &ClickEvent, _, cx| {
                canvas::with(|canvas| {
                    canvas.app().act(action.clone());
                });
                cx.notify();
            })
        };
        let items: Vec<_> = rows
            .into_iter()
            .map(|(label, selected, action)| {
                SidebarMenuItem::new(label)
                    .icon(IconName::Folder)
                    .active(selected)
                    .on_click(run(action))
            })
            .collect();
        let tool_buttons: Vec<_> = tools
            .into_iter()
            .enumerate()
            .map(|(index, (label, active, action))| {
                Button::new(("tool", index))
                    .label(label)
                    .small()
                    .selected(active)
                    .on_click(run(action))
            })
            .collect();
        h_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .child(
                        div().h(px(330.)).child(
                            Sidebar::new("sidebar")
                                .w_full()
                                .header(SidebarHeader::new().child("Space"))
                                .child(
                                    SidebarGroup::new("Items (SidebarModel)")
                                        .child(SidebarMenu::new().children(items)),
                                ),
                        ),
                    )
                    .child(
                        v_flex()
                            .p_3()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Tools (ToolbarModel)"),
                            )
                            .child(h_flex().flex_wrap().gap_1().children(tool_buttons)),
                    ),
            )
            // The edge marker the lockstep captures look for.
            .child(div().w(px(4.)).h_full().flex_shrink_0().bg(rgb(0x00ff00)))
    }

    /// Runs the action of the model's `index`th tool, as its button does.
    pub fn press_tool(&mut self, index: usize, cx: &mut Context<Self>) -> Option<String> {
        let label = canvas::with(|canvas| {
            let model = specular_interact::toolbar(canvas.app().app());
            let tool = (model.sections.iter())
                .filter_map(|section| match section {
                    ToolbarSection::Tools(tools) => Some(tools),
                    ToolbarSection::Zoom(_) => None,
                })
                .flatten()
                .nth(index)?
                .clone();
            canvas.app().act(tool.action);
            Some(tool.label.to_string())
        })
        .flatten();
        cx.notify();
        label
    }

    /// Runs the action of the model's first sidebar row, as a click does.
    pub fn press_first_row(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let label = canvas::with(|canvas| {
            let model = specular_interact::sidebar(canvas.app().app());
            let row = model.pages.first().or(model.notes.first())?.clone();
            canvas.app().act(row.action);
            Some(row.label)
        })
        .flatten();
        cx.notify();
        label
    }

    fn canvas_slot(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = self.layout;
        let focus = self.focus.clone();
        let entity = cx.entity();
        div()
            .id("canvas-slot")
            .size_full()
            .track_focus(&self.focus)
            .on_key_down(|event: &KeyDownEvent, _, _| {
                log::line(format!(
                    "gpui   keyDown key={:?} key_char={:?} mods={:?} held={}",
                    event.keystroke.key,
                    event.keystroke.key_char,
                    event.keystroke.modifiers,
                    event.is_held
                ));
            })
            .on_key_up(|event: &KeyUpEvent, _, _| {
                log::line(format!("gpui   keyUp key={:?}", event.keystroke.key));
            })
            .on_modifiers_changed(|event: &ModifiersChangedEvent, _, _| {
                log::line(format!("gpui   modifiersChanged {:?}", event.modifiers));
            })
            .context_menu(|menu, _, _| {
                menu.menu("Duplicate", Box::new(Duplicate))
                    .menu("Bring to front", Box::new(BringToFront))
                    .separator()
                    .menu("Delete", Box::new(Delete))
            })
            .child(
                canvas(
                    move |bounds, window, _| {
                        let viewport = window.viewport_size();
                        canvas::with(|canvas| match layout.order {
                            // Below: the view fills the window and never
                            // moves; GPUI's unpainted slot is the hole.
                            Order::Below => canvas.set_rect(
                                0.0,
                                0.0,
                                f64::from(viewport.width),
                                f64::from(viewport.height),
                            ),
                            Order::Above => canvas.set_rect(
                                f64::from(bounds.origin.x),
                                f64::from(bounds.origin.y),
                                f64::from(bounds.size.width),
                                f64::from(bounds.size.height),
                            ),
                        });
                        window.insert_hitbox(bounds, HitboxBehavior::Normal)
                    },
                    move |bounds, hitbox, window, cx| {
                        let over = hitbox.clone();
                        let focus_on_press = focus.clone();
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                            if phase == DispatchPhase::Bubble && over.is_hovered(window) {
                                log::line(format!(
                                    "gpui   mouseDown {:?} at={:?} clicks={} mods={:?}",
                                    event.button,
                                    event.position,
                                    event.click_count,
                                    event.modifiers
                                ));
                                window.focus(&focus_on_press, cx);
                            }
                        });
                        let over = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, _| {
                            if phase == DispatchPhase::Bubble && over.is_hovered(window) {
                                log::line(format!(
                                    "gpui   mouseUp {:?} at={:?}",
                                    event.button, event.position
                                ));
                            }
                        });
                        let over = hitbox.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
                            if phase == DispatchPhase::Bubble
                                && over.is_hovered(window)
                                && event.pressed_button.is_some()
                            {
                                log::line(format!("gpui   mouseDrag at={:?}", event.position));
                            }
                        });
                        let over = hitbox.clone();
                        window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, _| {
                            if phase == DispatchPhase::Bubble && over.is_hovered(window) {
                                log::line(format!(
                                    "gpui   scroll delta={:?} phase={:?} mods={:?}",
                                    event.delta, event.touch_phase, event.modifiers
                                ));
                            }
                        });
                        let over = hitbox.clone();
                        window.on_mouse_event(move |event: &PinchEvent, phase, window, _| {
                            if phase == DispatchPhase::Bubble && over.is_hovered(window) {
                                log::line(format!(
                                    "gpui   pinch delta={} phase={:?}",
                                    event.delta, event.phase
                                ));
                            }
                        });
                        window.handle_input(
                            &focus,
                            ElementInputHandler::new(bounds, entity.clone()),
                            cx,
                        );
                        if layout.drive == Drive::Gpui {
                            canvas::with(|canvas| canvas.frame());
                            window.request_animation_frame();
                        }
                    },
                )
                .size_full(),
            )
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.animate {
            self.pacing.mark();
            window.request_animation_frame();
        }
        v_flex()
            .size_full()
            // Above: anything showing through a gap between the sidebar and
            // the canvas view is magenta. Below: nothing is painted, so the
            // canvas view shows through the slot.
            .when(self.layout.order == Order::Above, |this| {
                this.bg(rgb(0xff00ff))
            })
            .child(self.toolbar(cx))
            .child(
                div().flex_1().min_h_0().child(
                    h_resizable("split")
                        .child(
                            resizable_panel()
                                .size(px(260.))
                                .size_range(px(120.)..px(700.))
                                .child(self.sidebar(cx)),
                        )
                        .child(resizable_panel().child(self.canvas_slot(cx))),
                ),
            )
    }
}

/// The canvas as a text input client: what an entered page or a text item
/// would need from GPUI's IME plumbing.
impl EntityInputHandler for Shell {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let units: Vec<u16> = self.text.encode_utf16().collect();
        let end = range.end.min(units.len());
        Some(String::from_utf16_lossy(&units[range.start.min(end)..end]))
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let end = self.text.encode_utf16().count();
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let end = self.text.encode_utf16().count();
        let marked = self.marked.encode_utf16().count();
        (marked > 0).then(|| end - marked..end)
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        log::line("ime    unmark".to_owned());
        self.marked.clear();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        log::line(format!("ime    commit text={text:?} replacing={range:?}"));
        if FORWARD_IME.load(Ordering::Relaxed) {
            canvas::with(|canvas| {
                canvas.send_to_page(&InputEvent::Ime(ImeEvent::Commit {
                    text: text.to_owned(),
                    replacement: None,
                }))
            });
        }
        let keep = self.text.len() - self.marked.len();
        self.text.truncate(keep);
        self.text.push_str(text);
        self.marked.clear();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        log::line(format!(
            "ime    compose marked={new_text:?} selection={new_selected_range:?} replacing={range:?}"
        ));
        let keep = self.text.len() - self.marked.len();
        self.text.truncate(keep);
        self.text.push_str(new_text);
        self.marked = new_text.to_owned();
        if FORWARD_IME.load(Ordering::Relaxed) {
            let caret = new_selected_range.map_or(0, |range| range.end as u32);
            canvas::with(|canvas| {
                canvas.send_to_page(&InputEvent::Ime(ImeEvent::SetComposition {
                    text: new_text.to_owned(),
                    selection: caret..caret,
                    replacement: None,
                }))
            });
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        log::line(format!("ime    candidate bounds asked for {range_utf16:?}"));
        Some(Bounds::new(
            element_bounds.origin + point(px(200.), px(200.)),
            size(px(2.), px(20.)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}
