//! A block's or a mod's [`Material`]: a system view under the window's
//! GPUI content, filling the element, that shows the canvas and whatever
//! else lies behind it as glass or a frosted blur.
//!
//! GPUI composites a native view beneath everything it paints, so the
//! material shows wherever the elements over it leave the window clear. A
//! panel on a material leaves its own fill off ([`panel_fill`]), and a
//! translucent `background` over it tints it.
//!
//! macOS 26 has `NSGlassEffectView`; an earlier macOS gets the blur of an
//! `NSVisualEffectView`, which blends with the window's content behind it.
//! Each element keeps its view in its element state, so a material that
//! stops being drawn is taken out of the window with the state.

use std::cell::Cell;
use std::ffi::c_void;
use std::ptr::NonNull;

use gpui_kit::{
    AnyElement, App, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, NativeView, ParentElement as _, Pixels, Styled as _, Window, div,
};
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use specular_interact::Material;

type Id = *mut AnyObject;

thread_local! {
    /// Whether the panel being built lies on a material.
    static ON_MATERIAL: Cell<bool> = const { Cell::new(false) };
}

/// What `build` returns, with every [`panel_fill`] in it left clear when
/// `on` says the panel lies on a material.
pub(super) fn built_on<T>(on: bool, build: impl FnOnce() -> T) -> T {
    let was = ON_MATERIAL.replace(on);
    let built = build();
    ON_MATERIAL.set(was);
    built
}

/// A panel's own fill, `0xRRGGBBAA`: clear while it is built on a
/// material, so the material shows.
pub(super) fn panel_fill(hex: u32) -> gpui_kit::Hsla {
    if ON_MATERIAL.get() {
        gpui_kit::Hsla::transparent_black()
    } else {
        crate::theme::solid(hex)
    }
}

/// `NSVisualEffectMaterialSidebar`: the frosting macOS gives a sidebar,
/// light or dark with the window's appearance.
const SIDEBAR: isize = 7;
/// `NSVisualEffectBlendingModeWithinWindow`: blur what lies behind in the
/// window, which is the canvas.
const WITHIN_WINDOW: isize = 1;
/// `NSVisualEffectStateActive`: frosted while the window is not key too.
const ACTIVE: isize = 1;

/// `material` filling its parent, with corners of `radius`. `id` keeps the
/// one system view across frames, so it must be unique in the window.
pub(crate) fn material(id: impl Into<ElementId>, material: Material, radius: f32) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .top_0()
        .size_full()
        .child(Surface {
            id: id.into(),
            material,
            radius,
        })
        .into_any_element()
}

/// The element that holds the system view.
struct Surface {
    id: ElementId,
    material: Material,
    radius: f32,
}

/// The view a surface keeps between frames, and what it was made as.
struct Held {
    material: Material,
    radius: f32,
    view: NativeView,
}

impl Element for Surface {
    type RequestLayoutState = Option<AnyElement>;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let view = id.and_then(|id| {
            window.with_element_state(id, |held: Option<Option<Held>>, window| {
                let held = held.flatten().filter(|held| {
                    held.material == self.material && held.radius.total_cmp(&self.radius).is_eq()
                });
                let held = held.or_else(|| {
                    let view = attach(self.material, self.radius, window, cx)?;
                    Some(Held {
                        material: self.material,
                        radius: self.radius,
                        view,
                    })
                });
                (held.as_ref().map(|held| held.view.clone()), held)
            })
        });
        let mut shown =
            view.map_or_else(|| div().into_any_element(), IntoElement::into_any_element);
        (shown.request_layout(window, cx), Some(shown))
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui_kit::Bounds<Pixels>,
        shown: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(shown) = shown {
            shown.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui_kit::Bounds<Pixels>,
        shown: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(shown) = shown {
            shown.paint(window, cx);
        }
    }
}

/// A new system view for `material`, attached to `window`, or `None` when
/// the window cannot show one.
fn attach(
    material: Material,
    radius: f32,
    window: &mut Window,
    cx: &mut App,
) -> Option<NativeView> {
    let view = make(material, f64::from(radius));
    let pointer = NonNull::new(view.cast::<c_void>())?;
    // SAFETY: `view` is a live NSView made just now, in no view hierarchy.
    let attached = unsafe { NativeView::new(pointer, window, cx) };
    // SAFETY: `view` came from `new`, which this owns; GPUI retains its own.
    unsafe {
        let _: () = msg_send![view, release];
    }
    match attached {
        Ok(view) => Some(view),
        Err(error) => {
            tracing::warn!("a material cannot be shown: {error:#}");
            None
        }
    }
}

#[expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]
/// An `NSView` for `material` with corners of `radius`, owned by the caller.
fn make(material: Material, radius: f64) -> Id {
    let glass = match material {
        Material::Glass => objc2::runtime::AnyClass::get(c"NSGlassEffectView"),
        Material::Blur => None,
    };
    // SAFETY: Both classes are NSView subclasses, made and set up on the
    // main thread, where GPUI draws.
    unsafe {
        if let Some(glass) = glass {
            let view: Id = msg_send![glass, new];
            let _: () = msg_send![view, setCornerRadius: radius];
            return view;
        }
        let view: Id = msg_send![class!(NSVisualEffectView), new];
        let _: () = msg_send![view, setMaterial: SIDEBAR];
        let _: () = msg_send![view, setBlendingMode: WITHIN_WINDOW];
        let _: () = msg_send![view, setState: ACTIVE];
        let _: () = msg_send![view, setWantsLayer: true];
        let layer: Id = msg_send![view, layer];
        if !layer.is_null() {
            let _: () = msg_send![layer, setCornerRadius: radius];
            let _: () = msg_send![layer, setMasksToBounds: true];
        }
        view
    }
}
