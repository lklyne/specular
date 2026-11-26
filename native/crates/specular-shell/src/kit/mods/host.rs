//! What the window offers a mod: the command registry, run as a key or the
//! palette would run it, and the host components only a window can draw.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{App, IntoElement as _, ParentElement as _, Styled as _, div};
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

/// The host components, with the window's own `Zoom` and `ToolOptions`,
/// and an `Icon` that notes where the control it names is, so a scripted
/// run's `control NAME` clicks it where the mod drew it.
pub(super) fn components(revisions: Rc<RefCell<Revisions>>) -> Components {
    let standard = Components::standard(revisions);
    let icon = standard.get("Icon");
    standard
        .with("Icon", move |args, window, cx| {
            let control = (args.props().get("control").and_then(HostValue::as_str))
                .map(|id| ControlId::from(id.to_owned()));
            let glyph = match icon.as_ref() {
                Some(icon) => icon(args, window, cx),
                None => div().into_any_element(),
            };
            match control {
                Some(id) => (div().relative().flex_shrink_0())
                    .child(glyph)
                    .child(crate::view::mark(&id))
                    .into_any_element(),
                None => glyph,
            }
        })
        .with("Zoom", |args, _, _| {
            let compact =
                (args.props().get("compact").and_then(HostValue::as_bool)).unwrap_or(false);
            crate::view::zoom_readout(compact).unwrap_or_else(|| div().into_any_element())
        })
        .with("ToolOptions", |_, window, cx| {
            crate::view::tool_options(window, cx).unwrap_or_else(|| div().into_any_element())
        })
}
