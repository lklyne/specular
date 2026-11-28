//! Host components: Rust-built elements a mod places but cannot draw
//! itself (ADR 0055). A mod imports them from `specular` beside the
//! functions and gives each an id and props:
//!
//! ```js
//! Icon.new("tool.draw.glyph", { name: "draw-pen-tool", tint: "1", size: 20 })
//! Zoom.new("zoom", { compact: false })
//! ToolOptions.new("tool-options", { place: "docked" })
//! Favicon.new("view.item.p1.icon", { tab: "view.item.p1", icon: "page", size: 14 })
//! WindowDrag.new("tabs-bare", {})
//! Share.new("share", {})
//! Prism.new("prism", {})
//! Popover.new("dock.choices", { top: 36 }).child(list)
//! ```
//!
//! `Icon` is drawn here from the panel glyphs, and `Favicon` is a tab's
//! glyph until the window has the page's own icon, which no state key
//! carries. `Zoom` shows the camera, which is in no state key so that a pan
//! never enters a script; `ToolOptions` is the dock's own controls;
//! `WindowDrag` is bare title bar the window is dragged and zoomed by; and
//! `Share` is whatever the layout puts in the Share slot; `Prism` is the
//! first-run view's turning Prism, drawn on the GPU; `Popover` is a box
//! over the rest of the window that keeps a press from what is under it,
//! which a mod's own elements cannot. Only the window
//! has those, so here they are placeholders the window replaces
//! ([`Components::with`]).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Image, ImageFormat, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Styled as _, Window, div, img, px,
};
use gpui_shell::{ComponentArgs, HostValue};
use specular_interact::{Appearance, Icon, PaintRole, Palette, Revisions};
use specular_scene::{Color, Colors, icon_svg, panel_color};

use crate::chrome::appearance_of;
use crate::value::from_host;

/// Builds one host component's element from its id and props.
pub type Build = Rc<dyn for<'a> Fn(ComponentArgs<'a>, &mut Window, &mut App) -> AnyElement>;

/// The Prism's side, in points, as the window draws it.
const PRISM: f32 = 160.0;

/// The names of the host components every mod can import.
pub const HOST_COMPONENTS: [&str; 12] = [
    "Composer",
    "Favicon",
    "Field",
    "Icon",
    "Pattern",
    "Popover",
    "Prism",
    "Share",
    "Swatch",
    "ToolOptions",
    "WindowDrag",
    "Zoom",
];

/// The host components a mod's `specular` module exports, by name.
#[derive(Clone)]
pub struct Components {
    by_name: BTreeMap<&'static str, Build>,
}

impl std::fmt::Debug for Components {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.by_name.keys()).finish()
    }
}

impl Components {
    /// [`HOST_COMPONENTS`] as a window-less host has them: a real `Icon`,
    /// whose colours follow the `theme` key in `revisions`, and
    /// placeholders for the rest.
    pub fn standard(revisions: Rc<RefCell<Revisions>>) -> Self {
        let glyph = {
            let revisions = revisions.clone();
            Rc::new(
                move |args: ComponentArgs<'_>, _: &mut Window, _: &mut App| {
                    let appearance = appearance_of(revisions.borrow().get("theme"));
                    icon(args.props(), appearance)
                },
            )
        };
        let swatch = crate::swatch::build(revisions.clone());
        let favicon: Build = Rc::new(move |args, _, _| {
            let appearance = appearance_of(revisions.borrow().get("theme"));
            icon(&favicon_as_icon(args.props()), appearance)
        });
        let mut by_name: BTreeMap<&'static str, Build> = BTreeMap::new();
        by_name.insert("Icon", glyph);
        by_name.insert("Favicon", favicon);
        by_name.insert("Zoom", placeholder("100%"));
        by_name.insert("ToolOptions", placeholder(""));
        by_name.insert("Field", placeholder(""));
        by_name.insert("Composer", placeholder(""));
        by_name.insert("Pattern", placeholder(""));
        by_name.insert("Swatch", swatch);
        by_name.insert("Popover", Rc::new(|args, _, _| popover(args)));
        by_name.insert(
            "WindowDrag",
            Rc::new(|_, _, _| div().size_full().into_any_element()),
        );
        by_name.insert("Share", placeholder("Share"));
        by_name.insert(
            "Prism",
            Rc::new(|_, _, _| div().size(px(PRISM)).into_any_element()),
        );
        Self { by_name }
    }

    /// Puts `build` in as the component `name`, in place of one there was.
    #[must_use]
    pub fn with(
        mut self,
        name: &'static str,
        build: impl for<'a> Fn(ComponentArgs<'a>, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.by_name.insert(name, Rc::new(build));
        self
    }

    /// The component `name`, to wrap in one of the host's own.
    pub fn get(&self, name: &str) -> Option<Build> {
        self.by_name.get(name).cloned()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'static str, &Build)> {
        self.by_name.iter().map(|(name, build)| (*name, build))
    }
}

/// `Favicon {tab, icon, size?, color?, control?}` as the `Icon` it is
/// until the page's own icon is there: its `icon` as the glyph's name.
fn favicon_as_icon(props: &HostValue) -> HostValue {
    let mut fields = Vec::new();
    for key in ["size", "color", "control"] {
        if let Some(value) = props.get(key) {
            fields.push((key.to_owned(), value.clone()));
        }
    }
    if let Some(name) = props.get("icon") {
        fields.push(("name".to_owned(), name.clone()));
    }
    HostValue::Object(fields)
}

/// A component only a window can draw, standing in with `text`.
/// `Popover {top?, left?, right?, bottom?}` around its children: placed
/// out of the flow at those offsets from the `relative()` box it is in, and
/// a press anywhere on it is its children's alone. The canvas under a
/// dropdown that hangs below its bar never sees the press that picks from
/// it.
fn popover(mut args: ComponentArgs<'_>) -> AnyElement {
    let offset =
        |key: &str| (args.props().get(key).and_then(HostValue::as_number)).map(|at| px(at as f32));
    let (top, left, right, bottom) = (
        offset("top"),
        offset("left"),
        offset("right"),
        offset("bottom"),
    );
    let mut layer = div()
        .id(gpui::SharedString::from(args.id().to_owned()))
        .occlude()
        .absolute();
    if let Some(top) = top {
        layer = layer.top(top);
    }
    if let Some(left) = left {
        layer = layer.left(left);
    }
    if let Some(right) = right {
        layer = layer.right(right);
    }
    if let Some(bottom) = bottom {
        layer = layer.bottom(bottom);
    }
    layer.children(args.take_children()).into_any_element()
}

fn placeholder(text: &'static str) -> Build {
    Rc::new(move |_, _, _| div().child(text).into_any_element())
}

type Key = (Icon, Color, Option<Color>, bool, Appearance);

thread_local! {
    /// One image per glyph and colouring: GPUI caches a decoded image by
    /// its identity, so the same `Arc` must come back each frame.
    static IMAGES: RefCell<HashMap<Key, Arc<Image>>> = RefCell::new(HashMap::new());
}

/// A `#rrggbb` or `#rrggbbaa` literal.
fn color(text: &str) -> Option<Color> {
    let hex = text.strip_prefix('#')?;
    let value = u32::from_str_radix(hex, 16).ok()?;
    let rgba = match hex.len() {
        6 => value << 8 | 0xff,
        8 => value,
        _ => return None,
    };
    let [r, g, b, a] = rgba.to_be_bytes();
    Some(Color::rgba(r, g, b, a))
}

/// The surface a tool glyph's tint is resolved for: the pens show their
/// ink, the sticky and the shape their fill, as the toolbar draws them.
const fn tint_style(icon: Icon) -> (Palette, PaintRole) {
    match icon {
        Icon::DrawPenTool | Icon::DrawHighlightTool => (Palette::Vivid, PaintRole::Ink),
        _ => (Palette::Soft, PaintRole::Fill),
    }
}

/// `Icon {name, size?, color?, tint?, active?, control?}`: a panel glyph by
/// its name in the state (`select-tool`, `{"shape": "ellipse"}`), `size`
/// pixels square, drawn in `color` and showing the stored colour `tint`.
/// `control` is for a window, which notes where the control it names is.
fn icon(props: &HostValue, appearance: Appearance) -> AnyElement {
    let size = props
        .get("size")
        .and_then(HostValue::as_number)
        .unwrap_or(20.0);
    let Some(name) = props.get("name").map(from_host) else {
        return div().size(px(size as f32)).into_any_element();
    };
    let Ok(icon) = serde_json::from_value::<Icon>(name) else {
        return div().size(px(size as f32)).into_any_element();
    };
    let current = (props.get("color").and_then(HostValue::as_str))
        .and_then(color)
        .unwrap_or(Colors::of(appearance).panel.toolbar_text);
    let tint = (props.get("tint").map(from_host))
        .and_then(|tint| serde_json::from_value::<specular_doc::Color>(tint).ok())
        .map(|stored| {
            let (palette, role) = tint_style(icon);
            panel_color(&stored, palette, role, appearance)
        });
    let on = props
        .get("active")
        .and_then(HostValue::as_bool)
        .unwrap_or(false);
    let image = IMAGES.with(|images| {
        let mut images = images.borrow_mut();
        let image = images
            .entry((icon, current, tint, on, appearance))
            .or_insert_with(|| {
                let svg = icon_svg(icon, current, tint, on, appearance);
                Arc::new(Image::from_bytes(ImageFormat::Svg, svg.into_bytes()))
            });
        Arc::clone(image)
    });
    img(image)
        .size(px(size as f32))
        .flex_shrink_0()
        .into_any_element()
}
