//! `Swatch({ color, palette, role, size })`: a dot of a canvas colour as
//! the app's own dock draws it (ADR 0059).
//!
//! The panel keys name a colour as the `.canvas` file stores it (`"neutral"`,
//! `"3"`, `"#e0e0e0"`), which is not a colour a mod's element takes: the
//! same stored colour is one hue on a sticky and another on a pen stroke,
//! and the neutral follows the theme. A swatch resolves it here, with the
//! `palette` and `role` its control names; `null` is the hollow choice.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, IntoElement as _, Rgba, Styled as _, Window, div, px};
use gpui_shell::{ComponentArgs, HostValue};
use specular_interact::{PaintRole, Palette, Revisions};
use specular_scene::panel_color;

use crate::chrome::appearance_of;
use crate::components::Build;
use crate::value::from_host;

/// The dot's side when the props name none.
const SIZE: f32 = 14.0;

/// The `Swatch` component, following the theme in `revisions`.
pub(crate) fn build(revisions: Rc<RefCell<Revisions>>) -> Build {
    Rc::new(
        move |args: ComponentArgs<'_>, _: &mut Window, _: &mut App| {
            let props = args.props();
            let appearance = appearance_of(revisions.borrow().get("theme"));
            let text = |key: &str| props.get(key).and_then(HostValue::as_str);
            let palette = match text("palette") {
                Some("vivid") => Palette::Vivid,
                _ => Palette::Soft,
            };
            let role = match text("role") {
                Some("ink") => PaintRole::Ink,
                _ => PaintRole::Fill,
            };
            let size = props
                .get("size")
                .and_then(HostValue::as_number)
                .map_or(SIZE, |size| size as f32);
            let stored = (props.get("color").map(from_host))
                .and_then(|color| serde_json::from_value::<specular_doc::Color>(color).ok());
            let dot = div().size(px(size)).rounded_full();
            match stored {
                Some(stored) => {
                    let color = panel_color(&stored, palette, role, appearance);
                    let rgba = Rgba {
                        r: f32::from(color.r) / 255.0,
                        g: f32::from(color.g) / 255.0,
                        b: f32::from(color.b) / 255.0,
                        a: f32::from(color.a) / 255.0,
                    };
                    dot.bg(rgba).into_any_element()
                }
                None => dot.border_1().into_any_element(),
            }
        },
    )
}
