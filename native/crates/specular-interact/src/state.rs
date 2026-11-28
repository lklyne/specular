//! The app as named keys of JSON (ADR 0055): what a mod, a page or an
//! agent reads, one key at a time.
//!
//! | Key | What it holds |
//! |---|---|
//! | `panels/toolbar` | [`toolbar`]: the tools, the theme button, the view controls |
//! | `panels/tool_options` | [`tool_options`]: the armed tool's options, or `null` |
//! | `panels/dock` | [`dock`]: the selection's controls, or `null` while a tool has options |
//! | `panels/strip` | [`view_strip`]: the tab row, with the right panel's toggle and, while no mod draws the tools, the zoom readout |
//! | `panels/menu` | the open context menu and where it opened, or `null` |
//! | `panels/palette` | [`palette`]: every command a person can pick, with its command name |
//! | `panels/share` | the Share button and its popover |
//! | `panels/welcome` | [`onboarding`](crate::onboarding): the first-run view while no space is open, or `null` |
//! | `theme` | the appearance drawn and the choice behind it |
//! | `settings` | what is kept between launches |
//! | `keys` | every key binding: its keys, where it fires and the command it runs |
//! | `layout` | the layout file in force, what was toggled over it, and the regions hidden |
//! | `notices` | [`notices`]: a page that stopped or failed to load, a share that failed, mods to install |
//! | `canvas/mod_items` | the active canvas's mod items, without their positions (ADR 0057) |
//! | `tool` | the armed mod tool's stroke, or `null` (ADR 0057) |
//! | `call` | the last mod command run, `{command, number, args}`, or `null` |
//!
//! A control's `action` is left out: a reader presses a control with
//! `control.press {id}`, by the id written beside it. The camera is in no
//! key, so a pan or a pinch moves no revision; the zoom readout is written
//! as its id alone.

mod keys;
mod revisions;

pub use self::revisions::Revisions;

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::commands::{Run, naming};
use crate::{
    App, Appearance, PaletteRun, context_menu, dock, palette, tool_options, toolbar, view_strip,
};

/// Every key, in the order [`state`] lists them.
pub const STATE_KEYS: [&str; 16] = [
    "panels/toolbar",
    "panels/tool_options",
    "panels/dock",
    "panels/strip",
    "panels/menu",
    "panels/palette",
    "panels/share",
    "panels/welcome",
    "theme",
    "settings",
    "keys",
    "layout",
    "notices",
    "canvas/mod_items",
    "tool",
    "call",
];

fn to_json(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Every key of `app` and its value now.
pub fn state(app: &App) -> BTreeMap<&'static str, Value> {
    let bar = toolbar(app);
    let options = tool_options(app);
    let docked = if options.is_some() { None } else { dock(app) };
    let menu = (app.session.panel.menu.as_ref()).map(|open| {
        json!({
            "at": [open.at.x, open.at.y],
            "menu": to_json(context_menu(app, &open.target)),
        })
    });
    let entries = [
        ("panels/toolbar", to_json(&bar)),
        ("panels/tool_options", to_json(options)),
        ("panels/dock", to_json(docked)),
        ("panels/strip", strip_json(app, &bar)),
        ("panels/menu", menu.unwrap_or(Value::Null)),
        ("panels/palette", palette_json(app)),
        ("panels/share", to_json(&bar.share)),
        ("panels/welcome", to_json(crate::onboarding(app))),
        ("theme", theme_json(app)),
        ("settings", settings_json(app)),
        ("keys", keys::bindings()),
        ("layout", layout_json(app)),
        ("notices", to_json(crate::notices(app))),
        ("canvas/mod_items", crate::mods::mod_items_json(app)),
        ("tool", crate::mods::tool_json(app)),
        ("call", crate::mods::call_json(app)),
    ];
    entries.into_iter().collect()
}

/// The value of `key` now, or `None` for a key there is not.
pub fn state_key(app: &App, key: &str) -> Option<Value> {
    state(app).remove(key)
}

/// Publishes every key of `app` into `revisions`. Returns the keys whose
/// revision moved.
pub fn publish(revisions: &mut Revisions, app: &App) -> Vec<&'static str> {
    (state(app).into_iter())
        .filter_map(|(key, value)| revisions.publish(key, value).then_some(key))
        .collect()
}

/// Publishes the keys a mod acts on as they happen, which cannot wait for
/// the next full [`publish`]: the `call` key, and the `tool` key while its
/// stroke is live (see [`crate::mods::tool_is_live`]). Building them walks
/// no document. Returns whether one moved.
pub fn publish_live(revisions: &mut Revisions, app: &App) -> bool {
    let call = revisions.publish("call", crate::mods::call_json(app));
    let tool =
        crate::mods::tool_is_live(app) && revisions.publish("tool", crate::mods::tool_json(app));
    call || tool
}

/// The tab row as it draws: its tabs and its add button, then what ends
/// the row. The zoom readout is there only while the tools are the app's
/// own: a toolbar mod carries it in its ribbon.
fn strip_json(app: &App, bar: &crate::ToolbarModel) -> Value {
    let mut strip = to_json(view_strip(app));
    if let Value::Object(fields) = &mut strip {
        fields.insert("chat".to_owned(), to_json(&bar.chat));
        let zoom = app
            .layout()
            .toolbar
            .fill
            .is_none()
            .then(|| json!({ "id": "zoom" }));
        fields.insert("zoom".to_owned(), zoom.unwrap_or(Value::Null));
    }
    strip
}

fn palette_json(app: &App) -> Value {
    let items: Vec<Value> = (palette(app).into_iter())
        .map(|item| {
            let command = match item.run {
                PaletteRun::Action(action) => {
                    naming(&Run::Action(action)).map(|spec| spec.name.to_owned())
                }
                PaletteRun::Shell(command) => {
                    naming(&Run::Shell(command)).map(|spec| spec.name.to_owned())
                }
                PaletteRun::Named { name, .. } => crate::command(&name)
                    .map(|spec| spec.name.to_owned())
                    .or(Some(name)),
            };
            json!({
                "label": item.label,
                "group": item.group,
                "command": command,
                "chord": item.chord,
                "enabled": item.enabled,
                "checked": item.checked,
                "keywords": item.keywords,
            })
        })
        .collect();
    Value::Array(items)
}

const fn appearance_name(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    }
}

fn theme_json(app: &App) -> Value {
    json!({
        "appearance": appearance_name(app.appearance()),
        "choice": app.theme().key(),
    })
}

fn settings_json(app: &App) -> Value {
    let settings = app.settings;
    json!({
        "showSidebar": settings.show_sidebar,
        "showChat": settings.show_chat,
        "tools": settings.tools,
        "hideAgentCursors": settings.hidden_cursors.agent,
        "hideSyncedCursors": settings.hidden_cursors.synced,
    })
}

fn layout_json(app: &App) -> Value {
    let regions = app.regions();
    let hidden: Vec<&str> = (crate::Region::ALL.into_iter())
        .filter(|region| !regions.shows(*region))
        .map(crate::Region::name)
        .collect();
    json!({
        "file": to_json(app.layout()),
        "toggles": to_json(app.layout_toggles()),
        "hidden": hidden,
    })
}
