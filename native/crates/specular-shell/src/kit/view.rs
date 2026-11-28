//! The window's content: the Kit's chrome and sidebar, and the slot the
//! canvas shows through.
//!
//! ```text
//! ┌───────────────────────────────────────────────────────┐
//! │ tab row (Kit): the title bar, the zoom, the panel     │
//! ├──────────┬───┬───────────────────────────┬────────────┤
//! │ sidebar  │ t │ dock (Kit), while it has  │ right      │
//! │ (Kit)    │ o │ something to hold         │ panel      │
//! │          │ o ├───────────────────────────┤ (Kit)      │
//! │          │ l │ canvas slot: nothing      │            │
//! │          │ s │ painted, so the canvas    │            │
//! │          │   │ view under GPUI shows     │            │
//! └──────────┴───┴───────────────────────────┴────────────┘
//! ```
//!
//! The slot fills the window beside the right panel, as the app's own
//! layout assumes, and everything else lies over it. What the app counts
//! as covered is `App::insets`, which the models carry: the tab row, the
//! sidebar while its model says it is visible, and the tools while they are
//! docked. The tools can float over the slot's top or bottom instead,
//! covering nothing the app counts. A region the layout hides is not drawn
//! and the insets do not count it (`Models::regions`), so the canvas has
//! its room.
//!
//! The dock is a bar over the top of the slot. In a tab that shows its
//! item it is always there, holding the lens, and the app counts it. On the
//! canvas it comes with a selection and goes
//! with it, over the canvas, so nothing under it moves. The right panel is
//! beside the slot, so opening it or dragging its edge makes the app's
//! viewport narrower.

mod canvas_menu;
mod chat;
mod controls;
mod dock;
mod dropdown;
mod field;
mod frame;
mod glyphs;
mod ime;
mod material;
mod menu;
mod mod_bar;
mod named;
mod onboarding;
mod overlay_layer;
mod palette;
mod palette_row;
mod raised;
mod share;
mod sidebar;
mod slot;
mod squircle;
mod tabs;
mod toolbar;

use std::path::PathBuf;
use std::rc::Rc;

use glam::Vec2;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, ExternalPaths, FocusHandle, Global,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _, Window, div, px,
    size,
};
use specular_interact::{Action, CanvasAction, Event, SidebarAction, ToolsPlace};

pub(crate) use self::field::{FieldStyle, field_styled as model_field, typed as typed_field};
pub(crate) use self::material::material;
pub(crate) use self::named::{mark, shown as shown_controls};
pub(crate) use self::palette::toggle as toggle_palette;
pub(crate) use self::share::share_slot;
use self::slot::Pointer;
pub(crate) use self::tabs::{favicon, window_drag};
pub(crate) use self::toolbar::{tool_options, zoom_readout};
use crate::canvas::{self, Models};
use crate::shell;
use crate::surface::WindowAsks;
use crate::theme;

/// The canvas slot's focus, so a click on a Kit control can hand the keys
/// back to the canvas.
pub(crate) struct CanvasFocus(pub(crate) FocusHandle);

impl Global for CanvasFocus {}

/// Runs `action` through `update` and gives the keys back to the canvas,
/// unless a list it was chosen in is still open and has them.
///
/// Renaming a canvas is the one action a renderer answers itself: the model
/// asks for the row's name field to be typed in, and here that field is the
/// Kit's.
pub(crate) fn run(action: &Action, window: &mut Window, cx: &mut App) {
    if let Action::Canvas(CanvasAction::BeginRename(canvas)) = action {
        begin_rename(canvas.as_ref(), window, cx);
        return;
    }
    canvas::dispatch(Event::Action(action.clone()));
    give_back_keys(window, cx);
}

/// Gives the keys back to the canvas after a control ran, unless a list it
/// was chosen in is still open and has them, or a text field does: a mod's
/// field that ran a command on Return keeps them for the next line.
pub(crate) fn give_back_keys(window: &mut Window, cx: &mut App) {
    if !dropdown::list_open(window, cx) && !in_text_field(window) {
        focus_canvas(window, cx);
    }
}

/// Whether the keys are in a text field, the Kit's own or a mod's: every
/// one is under the `Input` key context.
pub(crate) fn in_text_field(window: &Window) -> bool {
    (window.context_stack().iter()).any(|context| context.contains("Input"))
}

/// Opens the name field on the row of `canvas`, or of the canvas showing,
/// with the sidebar shown first if it was hidden.
fn begin_rename(canvas: Option<&specular_interact::CanvasId>, window: &mut Window, cx: &mut App) {
    if canvas::models().is_some_and(|models| !models.sidebar.visible) {
        canvas::dispatch(Event::Action(Action::Sidebar(SidebarAction::Toggle)));
    }
    let Some(models) = canvas::models() else {
        return;
    };
    let row = (models.sidebar.canvases.iter()).find(|row| match canvas {
        Some(canvas) => row.id == *canvas,
        None => row.active,
    });
    if let Some(row) = row {
        field::begin_inline(&row.rename, window, cx);
    }
}

/// Files dropped on the window at `at`, in points from the content's
/// corner: the end of a drag from Finder, and a script's `drop` step.
pub(crate) fn drop_files(paths: Vec<PathBuf>, at: Vec2) {
    canvas::with(|canvas| {
        let origin = (canvas.runtime.window())
            .map_or(Vec2::ZERO, crate::surface::CanvasSurface::slot_origin);
        canvas.runtime.drop_files(paths, Some(at - origin));
    });
}

/// What lies over the slot and moves with its edges: the dock's bar, and
/// the tools while they float, drawn by `ribbon` when a mod draws them.
fn over_slot(
    models: &Models,
    (ribbon, dock_mod): (Option<AnyElement>, Option<AnyElement>),
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let left = models.insets.left;
    let wanted = models.dock.is_some() || !models.toolbar.view.is_empty();
    let dock = match dock_mod {
        _ if !models.regions.dock => None,
        Some(dock_mod) => wanted.then(|| dock::mod_bar(dock_mod, left).into_any_element()),
        None => dock::dock(models.dock.as_ref(), &models.toolbar, left, window, cx)
            .map(IntoElement::into_any_element),
    };
    let mut top = theme::chrome_top();
    if dock.is_some() {
        top += theme::DOCK_ROW;
    }
    let mut out: Vec<AnyElement> = dock.into_iter().collect();
    if models.regions.toolbar && models.toolbar.tools != ToolsPlace::Docked {
        let shown = models.tool_options.as_ref();
        out.push(match ribbon {
            Some(ribbon) => {
                mod_bar::floating(ribbon, models.toolbar.tools, (left, top)).into_any_element()
            }
            None => toolbar::floating(&models.toolbar, shown, (left, top), window, cx)
                .into_any_element(),
        });
    }
    out
}

/// Gives the keys back to the canvas.
pub(crate) fn focus_canvas(window: &mut Window, cx: &mut App) {
    if let Some(focus) = cx.try_global::<CanvasFocus>().map(|focus| focus.0.clone()) {
        window.focus(&focus, cx);
    }
}

/// A space opened from the first-run view: the window, still at its
/// welcome size, takes the canvas window's place and size. One the user
/// resized is left as it is, and so is one opened at a size asked for, for
/// a benchmark or for a script.
fn grow_from_welcome(window: &Window, cx: &mut App) {
    if !cx
        .try_global::<shell::PlacesWindow>()
        .is_some_and(|places| places.0)
    {
        return;
    }
    let (width, height) = shell::WELCOME_SIZE;
    let viewport = window.viewport_size();
    if viewport != size(px(width as f32), px(height as f32)) {
        return;
    }
    // Not from inside the render: the resize calls back into the window.
    // That callback cannot reach the window while the app is borrowed, as
    // it still is in a deferred call, so the window learns its new size here.
    window.defer(cx, |window, cx| {
        let placed = canvas::with(|canvas| {
            canvas
                .runtime
                .window()
                .map(|surface| surface.place_main_window(shell::DEFAULT_SIZE))
        })
        .flatten();
        if placed.is_some() {
            window.bounds_changed(cx);
        }
    });
}

/// The root view: it holds what only a view can (focus, the input method's
/// marked text, the sidebar row a shift-click runs from) and draws
/// everything else from the models.
pub(crate) struct ShellView {
    focus: FocusHandle,
    asks: Rc<WindowAsks>,
    pointer: Rc<Pointer>,
    /// The input method's marked text, which GPUI asks back for.
    marked: String,
    /// What the right panel keeps between frames.
    chat: chat::ChatUi,
    /// The first-run view's Prism, while that view is shown.
    hero: Option<Entity<onboarding::HeroPrism>>,
    /// The context menu a right press on the canvas opened, while it is.
    menu: Option<canvas_menu::CanvasMenu>,
    /// The mods the layout names, drawn in place of the app's own panels.
    mods: crate::mods::Mods,
}

impl ShellView {
    pub(crate) fn new(
        asks: Rc<WindowAsks>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        cx.set_global(CanvasFocus(focus.clone()));
        window.focus(&focus, cx);
        Self {
            focus,
            asks,
            pointer: Rc::new(Pointer::default()),
            marked: String::new(),
            chat: chat::ChatUi::new(window, cx),
            hero: None,
            menu: None,
            mods: crate::mods::Mods::default(),
        }
    }
}

impl ShellView {
    /// Whether the keys are in a text field: the composer, a model field
    /// being typed in (a canvas's name, a dock field), or a mod's.
    pub(crate) fn typing(&self, window: &Window, cx: &mut App) -> bool {
        let in_field = window
            .focused(cx)
            .is_some_and(|focused| field::holds_focus(&focused, window, cx));
        in_field || self.composer_focused(window, cx) || in_text_field(window)
    }
}

impl ShellView {
    /// The canvas slot, from the window's corner to the canvas block's far
    /// corner, with what floats over it (the dock, a floating toolbar, the
    /// notices) and the mods that run out of sight.
    fn slot(
        &mut self,
        models: Option<&Models>,
        canvas: Option<specular_interact::Frame>,
        (ribbon, dock, toast): (Option<AnyElement>, Option<AnyElement>, Option<AnyElement>),
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> AnyElement {
        let frame = div().absolute().left_0().top_0();
        let frame = match canvas {
            Some(canvas) => frame.w(px(canvas.right())).h(px(canvas.bottom())),
            None => frame.size_full(),
        };
        frame
            .children(self.mods.background(cx))
            .child(self.canvas_slot(window, cx))
            .children(models.map_or_default(|models| over_slot(models, (ribbon, dock), window, cx)))
            .children(toast)
            .into_any_element()
    }
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        // What effects asked of the window is applied by this render.
        self.asks.changed.set(false);
        if let Some(wanted) = self.asks.field.take() {
            field::want(wanted, cx);
        }
        if let Some(command) = self.asks.shell.take() {
            window.defer(cx, move |_, cx| crate::menus::run_shell(command, cx));
        }
        named::begin_frame();
        let models = canvas::models();
        if let Some(models) = models.as_ref() {
            crate::window_buttons::apply(models.layout.look.traffic_lights, window);
        }
        let size = window.viewport_size();
        let size = (f32::from(size.width), f32::from(size.height));
        let boxes = models.as_ref().map(|models| frame::boxes(models, size));
        let canvas = boxes.as_deref().map(frame::canvas_frame);
        theme::set_chrome_top(canvas.map_or(theme::TAB_ROW, |canvas| canvas.y));
        if let Some(model) = models
            .as_ref()
            .and_then(|models| models.onboarding.as_ref())
        {
            let hero = (self
                .hero
                .get_or_insert_with(|| cx.new(|_| onboarding::HeroPrism::default())))
            .clone();
            if let Some(welcome) = self.welcome_mod(models.as_ref(), &hero, window, cx) {
                return welcome;
            }
            return onboarding::onboarding(model, hero).into_any_element();
        }
        if let Some(hero) = self.hero.take() {
            crate::mods::set_hero(None);
            let images = hero.update(cx, |hero, _| hero.take_images());
            crate::avatar::drop_images(images, window);
            grow_from_welcome(window, cx);
        }
        let panels = self.mod_panels(models.as_ref(), window, cx);
        let menu = self.canvas_menu(models.as_ref(), window, cx);
        let mod_bar::ModPanels {
            zoom_in_tabs,
            floating: floating_ribbon,
            dock,
            toast,
        } = panels;
        let blocks = match (models.as_ref(), boxes.as_deref()) {
            (Some(models), Some(boxes)) => self.blocks(models, boxes, zoom_in_tabs, window, cx),
            _ => Vec::new(),
        };
        let overlays = models.as_ref().map_or_default(|models| {
            overlay_layer::layer(&models.mod_overlays, |id| self.mods.copy(id, cx), size)
        });
        let backdrop = match (models.as_ref(), canvas) {
            (Some(models), Some(canvas)) => frame::backdrop(
                models,
                specular_interact::Frame::sized(size.0, size.1),
                canvas,
            ),
            _ => Vec::new(),
        };
        let mut root = div()
            .id("shell")
            .size_full()
            .relative()
            .text_color(cx.theme().foreground)
            .text_size(px(crate::look::font_size()));
        if let Some(font) = crate::look::font() {
            root = root.font_family(font);
        }
        root
            // A drop the canvas slot did not take, on the toolbar, the
            // sidebar or the right panel, lands where a paste would.
            .on_drop(|paths: &ExternalPaths, _, _| {
                canvas::with(|canvas| {
                    (canvas.runtime).drop_files(paths.paths().iter().cloned(), None);
                });
            })
            .children(backdrop)
            .child(self.slot(
                models.as_ref(),
                canvas,
                (floating_ribbon, dock, toast),
                window,
                cx,
            ))
            .children(blocks)
            .children(models.as_ref().and_then(|models| {
                frame::window_border(models, crate::window_buttons::corner_radius(window))
            }))
            .when_some(models.as_ref(), |root, models: &Models| {
                root.when(models.regions.chat, |root| {
                    root.children(self.chat_resize_handle(&models.chat))
                })
            })
            // A drag that began on the canvas is the canvas's until it ends:
            // nothing it passes over hovers, opens a tooltip or scrolls.
            .when(self.pointer.held(), |root| {
                root.child(div().absolute().inset_0().occlude())
            })
            // A press outside an open list closes it and reaches nothing
            // under it, as a macOS menu's does.
            .when(dropdown::list_open(window, cx), |root| {
                root.child(div().absolute().inset_0().occlude())
            })
            .children(menu)
            .children(overlays)
            .into_any_element()
    }
}
