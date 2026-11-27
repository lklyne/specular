//! What the window offers a mod: the command registry, run as a key or the
//! palette would run it, and the host components only a window can draw.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{AnyElement, App, IntoElement as _, ParentElement as _, Styled as _, div};
use serde_json::{Value, json};
use specular_interact::{ControlId, Run};
use specular_mods::{Components, HostValue, ModHost, Revisions, RunError, with_current_app};

use crate::canvas;

/// The window as a mod's host.
pub(super) struct KitHost;

/// Does what `run` names, after the script call that asked for it.
fn perform(run: Run, cx: &mut App) {
    match run.into_event() {
        Ok(event) => canvas::dispatch(event),
        Err(command) => crate::menus::run_shell(command, cx),
    }
    if let Some(window) = cx.active_window() {
        let _ = window.update(cx, |_, window, cx| crate::view::give_back_keys(window, cx));
    }
}

impl ModHost for KitHost {
    fn run(&self, name: &str, args: &Value) -> Result<Value, RunError> {
        let run =
            canvas::with(|canvas| specular_interact::run_named(canvas.runtime.app(), name, args))
                .ok_or_else(|| RunError("the app is busy; try again".to_owned()))?
                .map_err(|error| RunError(error.to_string()))?;
        // A host function must not re-enter the script engine, and the
        // update a command makes can draw the mods again, so it runs once
        // the script's call has returned.
        with_current_app(|cx| cx.defer(move |cx| perform(run, cx)))
            .ok_or_else(|| RunError("the window cannot take a command now".to_owned()))?;
        Ok(json!({ "ok": true, "name": name }))
    }

    fn commands(&self) -> Value {
        canvas::with(|canvas| {
            let app = canvas.runtime.app();
            (specular_interact::commands().iter())
                .map(|spec| {
                    json!({
                        "name": spec.name,
                        "args": spec.args,
                        "about": spec.about,
                        "enabled": (spec.enabled)(app),
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| json!([]))
    }

    fn notify(&self, level: &str, text: &str) {
        match level {
            "error" => tracing::error!(target: "specular_mods", "{text}"),
            "warning" => tracing::warn!(target: "specular_mods", "{text}"),
            _ => tracing::info!(target: "specular_mods", "{text}"),
        }
    }

    fn shared_changed(&self) {
        // The mods that read the value render again on the next draw.
        with_current_app(|cx| cx.defer(App::refresh_windows));
    }
}

/// The control `props.control` names, if it names one.
fn control_of(props: &HostValue) -> Option<ControlId> {
    (props.get("control").and_then(HostValue::as_str)).map(|id| ControlId::from(id.to_owned()))
}

/// `glyph` with the mark of `control`, so a scripted run's `control NAME`
/// clicks it where the mod drew it.
fn marked(control: Option<ControlId>, glyph: AnyElement) -> AnyElement {
    match control {
        Some(id) => (div().relative().flex_shrink_0())
            .child(glyph)
            .child(crate::view::mark(&id))
            .into_any_element(),
        None => glyph,
    }
}

/// The host components, with the window's own `Zoom`, `ToolOptions`, `Field`,
/// `WindowDrag`, `Share` and `Prism`, a `Favicon` that shows the page's
/// own icon, and an `Icon` that notes where the control it names is.
pub(super) fn components(revisions: Rc<RefCell<Revisions>>) -> Components {
    let standard = Components::standard(revisions.clone());
    let icon = standard.get("Icon");
    let glyph = standard.get("Favicon");
    standard
        .with("Favicon", move |args, window, cx| {
            let props = args.props();
            let control = control_of(props);
            let tab = (props.get("tab").and_then(HostValue::as_str))
                .map(|id| ControlId::from(id.to_owned()));
            let size = (props.get("size").and_then(HostValue::as_number)).unwrap_or(14.0) as f32;
            let png = tab.as_ref().and_then(canvas::favicon);
            let shown = match (tab, png, glyph.as_ref()) {
                (Some(tab), Some(png), _) => crate::view::favicon(&tab, &png, size),
                (_, _, Some(glyph)) => glyph(args, window, cx),
                (_, _, None) => div().into_any_element(),
            };
            marked(control, shown)
        })
        .with("WindowDrag", |_, _, _| crate::view::window_drag())
        .with("Pattern", |args, _, _| pattern(args.props()))
        .with("Share", |_, _, _| crate::view::share_slot())
        .with("Prism", |_, _, _| {
            crate::mods::hero().map_or_else(
                || div().into_any_element(),
                gpui_kit::IntoElement::into_any_element,
            )
        })
        .with("Icon", move |args, window, cx| {
            let control = control_of(args.props());
            let glyph = match icon.as_ref() {
                Some(icon) => icon(args, window, cx),
                None => div().into_any_element(),
            };
            marked(control, glyph)
        })
        .with("Zoom", |args, _, _| {
            let compact =
                (args.props().get("compact").and_then(HostValue::as_bool)).unwrap_or(false);
            crate::view::zoom_readout(compact).unwrap_or_else(|| div().into_any_element())
        })
        .with("Field", move |args, window, cx| {
            let style = field_style(args.props());
            let Some(id) = args.props().get("control").and_then(HostValue::as_str) else {
                return super::text_field::field(args.props(), style, &revisions, window, cx);
            };
            let model = canvas::models().and_then(|models| {
                [models.dock.as_ref(), models.tool_options.as_ref()]
                    .into_iter()
                    .flatten()
                    .find_map(|controls| controls.field(id).cloned())
            });
            match model {
                Some(model) => crate::view::model_field(&model, style, window, cx),
                None => div().into_any_element(),
            }
        })
        .with("ToolOptions", |_, window, cx| {
            crate::view::tool_options(window, cx).unwrap_or_else(|| div().into_any_element())
        })
}

/// `Pattern({ paint, width, height })`: a box filling its parent, painted
/// as a layout paints a block (`crate::paint`). `width` and `height` say
/// how far stripes must reach; the window's size when left out.
fn pattern(props: &HostValue) -> AnyElement {
    let paint = props.get("paint").map(specular_mods::from_host);
    let reach = |key: &str| {
        props
            .get(key)
            .and_then(HostValue::as_number)
            .map_or(1600.0, |value| value as f32)
    };
    let element = div().absolute().left_0().top_0().size_full();
    crate::paint::paint(element, paint.as_ref(), (reach("width"), reach("height")))
        .into_any_element()
}

/// A `Field`'s `radius`, `background` and `border` props, over the app's
/// own look of a field.
fn field_style(props: &HostValue) -> crate::view::FieldStyle {
    let color = |key: &str| {
        props
            .get(key)
            .and_then(HostValue::as_str)
            .and_then(crate::paint::hsla)
    };
    crate::view::FieldStyle {
        radius: props
            .get("radius")
            .and_then(HostValue::as_number)
            .map(|radius| radius as f32),
        background: color("background"),
        border: color("border"),
    }
}
