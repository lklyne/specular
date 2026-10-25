//! The context menu a right press on the canvas opens, as a Kit popup menu
//! with its corner at the press.
//!
//! The app owns whether the menu is open, what it is about and where it
//! opened; this draws [`canvas::open_menu`] each frame. A pick presses the
//! item by its name, as a script does, so a paste lands where the menu
//! opened. Closing it any other way (a press outside, Escape) tells the
//! app. The menu has the keys while it is open: the arrows move through
//! it, Return picks and Escape closes.
//!
//! The menu is fixed to a screen point, not to the item it is about. While
//! it is open a layer over the window takes the pointer, the wheel and a
//! pinch, as a macOS context menu does, so the canvas cannot move under it.

use glam::Vec2;
use gpui_kit::base::POPUP_PRIORITY;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::{
    AnyElement, App, Context, DismissEvent, Entity, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement as _, ParentElement as _, Styled as _, Subscription,
    Window, anchored, deferred, div, point, px,
};
use specular_interact::panel::builtin::ContextMenu;
use specular_interact::{ControlsModel, Event};

use super::menu::{self, Pick};
use super::{ShellView, focus_canvas};
use crate::canvas;

/// The open menu as drawn: what the app said and the Kit menu showing it.
pub(super) struct CanvasMenu {
    open: ContextMenu,
    model: ControlsModel,
    view: Entity<PopupMenu>,
    _dismissed: Subscription,
}

/// A pick from the canvas's menu: the item pressed by its name.
fn press(
    id: &specular_interact::ControlId,
    _: &specular_interact::Action,
    window: &mut Window,
    cx: &mut App,
) {
    canvas::dispatch(Event::Control(
        id.clone(),
        specular_core::Modifiers::default(),
    ));
    focus_canvas(window, cx);
}

/// A Kit popup menu of `model`, holding the keys, that gives them back to
/// `canvas` when it closes.
fn popup(
    model: &ControlsModel,
    canvas: FocusHandle,
    pick: Pick,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    let view = PopupMenu::build(window, cx, |popup, _, _| {
        menu::filled(popup.action_context(canvas), model, pick)
    });
    view.focus_handle(cx).focus(window, cx);
    view
}

/// `menu` with its corner at `corner`, in points from the window's corner,
/// over a layer that keeps the pointer, the wheel and a pinch from what is
/// under it.
fn layer(menu: Entity<PopupMenu>, corner: Vec2, window: &Window) -> AnyElement {
    let viewport = window.viewport_size();
    deferred(
        anchored().child(
            div()
                .id("canvas-menu-layer")
                .occlude()
                .w(viewport.width)
                .h(viewport.height)
                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                .child(
                    anchored()
                        .position(point(px(corner.x), px(corner.y)))
                        .snap_to_window_with_margin(px(8.0))
                        .child(menu),
                ),
        ),
    )
    .with_priority(POPUP_PRIORITY)
    .into_any_element()
}

impl ShellView {
    /// The open context menu over the window, or nothing while none is open.
    pub(super) fn canvas_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<AnyElement> {
        let Some((open, model)) = canvas::open_menu() else {
            // The app closed it: an item was pressed by name, or what it
            // was about changed.
            if self.menu.take().is_some() {
                focus_canvas(window, cx);
            }
            return None;
        };
        let current =
            (self.menu.as_ref()).is_some_and(|held| held.open == open && held.model == model);
        if !current {
            let view = popup(&model, self.focus.clone(), press, window, cx);
            let dismissed =
                cx.subscribe_in(&view, window, |this, _, _: &DismissEvent, window, cx| {
                    // A pick has closed the app's menu already; telling it
                    // again does nothing.
                    if this.menu.take().is_some() {
                        canvas::dispatch(Event::MenuClosed);
                    }
                    focus_canvas(window, cx);
                    cx.notify();
                });
            self.menu = Some(CanvasMenu {
                open,
                model,
                view,
                _dismissed: dismissed,
            });
        }
        let held = self.menu.as_ref()?;
        let origin = canvas::with(|canvas| {
            (canvas.runtime.window()).map(crate::surface::CanvasSurface::slot_origin)
        })
        .flatten()
        .unwrap_or_default();
        Some(layer(held.view.clone(), origin + held.open.at, window))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AppContext as _, MouseButton, Render, TestAppContext, WindowHandle, base::Root,
    };
    use specular_interact::{
        Action, Choices, Chord, Control, ControlId, DropdownOption, DropdownSection, Face,
        OptionLayout,
    };

    use super::*;
    use crate::view::named;

    thread_local! {
        static PICKED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static UNDER: RefCell<u32> = const { RefCell::new(0) };
    }

    fn item(name: &str, label: &'static str, enabled: bool) -> DropdownOption {
        DropdownOption {
            id: ControlId::new("menu").child(name),
            label: label.into(),
            face: Face::text(label),
            trailing: None,
            fields: Vec::new(),
            chord: (name == "copy").then(|| Chord::char('c').cmd()),
            selected: false,
            enabled,
            action: Action::Copy,
        }
    }

    fn model() -> ControlsModel {
        let list = |options| DropdownSection::Options {
            layout: OptionLayout::List,
            options,
        };
        ControlsModel {
            controls: vec![Control::Choices(Choices {
                label: "Menu".into(),
                content: vec![
                    list(vec![
                        item("copy", "Copy", true),
                        item("paste", "Paste", false),
                    ]),
                    list(vec![item("delete", "Delete", true)]),
                ],
                id: ControlId::new("menu"),
            })],
        }
    }

    fn picked(id: &ControlId, _: &Action, _: &mut Window, _: &mut App) {
        PICKED.with(|picked| picked.borrow_mut().push(id.as_str().to_owned()));
    }

    /// A window whose whole area counts presses, under the menu opened at
    /// (300, 200).
    struct Host {
        menu: Entity<PopupMenu>,
    }

    impl Render for Host {
        fn render(
            &mut self,
            window: &mut Window,
            _: &mut Context<'_, Self>,
        ) -> impl gpui_kit::IntoElement {
            named::begin_frame();
            div()
                .size_full()
                .child(
                    div()
                        .id("under")
                        .size_full()
                        .on_mouse_down(MouseButton::Left, |_, _, _| {
                            UNDER.with(|under| *under.borrow_mut() += 1);
                        }),
                )
                .child(layer(self.menu.clone(), Vec2::new(300.0, 200.0), window))
        }
    }

    fn open(cx: &mut TestAppContext) -> WindowHandle<Root> {
        cx.update(gpui_kit::init);
        let window = cx.update(|cx| {
            let (window, _) =
                gpui_kit::open_window(gpui_kit::WindowOptions::default(), cx, |window, cx| {
                    let focus = cx.focus_handle();
                    let menu = popup(&model(), focus, picked, window, cx);
                    cx.new(|_| Host { menu })
                })
                .expect("a test window");
            window.downcast::<Root>().expect("the Kit's root")
        });
        cx.update_window(window.into(), |_, window, cx| window.render_frame(cx))
            .expect("the window");
        window
    }

    fn click(cx: &mut TestAppContext, window: WindowHandle<Root>, at: Vec2) {
        let at = point(px(at.x), px(at.y));
        cx.update_window(window.into(), |_, window, cx| window.drag(at, at, cx))
            .expect("the window");
        cx.run_until_parked();
    }

    fn shown(name: &str) -> Option<Vec2> {
        named::shown()
            .into_iter()
            .find(|(shown, _)| shown == name)
            .map(|(_, at)| at)
    }

    #[gpui_kit::test]
    fn the_menu_opens_at_the_press_and_picks_an_enabled_item_by_name(cx: &mut TestAppContext) {
        let window = open(cx);
        let copy = shown("menu.copy").expect("the copy item is drawn");
        let delete = shown("menu.delete").expect("the delete item is drawn");
        // Its corner is the press, and the items run down from it.
        assert!(
            copy.x > 300.0 && copy.y > 200.0 && delete.y > copy.y,
            "{copy} {delete}"
        );
        // A disabled item is drawn and picks nothing.
        click(
            cx,
            window,
            shown("menu.paste").expect("the paste item is drawn"),
        );
        click(cx, window, copy);
        assert_eq!(PICKED.with(|picked| picked.borrow().clone()), ["menu.copy"]);
        assert_eq!(UNDER.with(|under| *under.borrow()), 0);
    }

    #[gpui_kit::test]
    fn the_arrows_and_return_pick_and_a_press_outside_reaches_nothing_under(
        cx: &mut TestAppContext,
    ) {
        let window = open(cx);
        cx.update_window(window.into(), |_, window, cx| {
            window.press("down", cx);
            window.press("enter", cx);
        })
        .expect("the window");
        cx.run_until_parked();
        assert_eq!(PICKED.with(|picked| picked.borrow().clone()), ["menu.copy"]);
        click(cx, window, Vec2::new(40.0, 40.0));
        assert_eq!(UNDER.with(|under| *under.borrow()), 0);
    }
}
