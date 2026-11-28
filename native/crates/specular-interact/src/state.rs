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
//! | `panels/sidebar` | the sidebar: shown or not, and its sections of rows by control id (ADR 0059) |
//! | `panels/chat` | the right panel: its threads, the transcript showing and the composer; `chat.*` commands act on it (ADR 0059) |
//! | `chat/context` | what the next message goes with: the pill, the folder, the auto-fix mode and the model |
//! | `chat/threads` | the active canvas's threads with no transcript, the one the panel has open, and the model |
//! | `chat/thread/<id>` | one thread of the active canvas on its own: transcript, queue, run; one key a thread |
//! | `theme` | the appearance drawn and the choice behind it |
//! | `settings` | what is kept between launches |
//! | `keys` | every key binding: its keys, where it fires and the command it runs, the person's `keys.json` first |
//! | `layout` | the layout in force by name, its file, what was toggled over it, and the regions hidden |
//! | `layouts` | every layout, `{name, title, active}` (ADR 0059) |
//! | `look` | the look in force: appearance, theme, colours by name, font, text size (ADR 0059) |
//! | `overlays` | the mod overlays open, bottom first: `{id, mod, props, kind, ...}` (ADR 0059) |
//! | `notices` | [`notices`]: a page that stopped or failed to load, a share that failed, mods to install |
//! | `selection` | `{ids, edges}`: what is selected (ADR 0059) |
//! | `canvas/items` | every item of the active canvas, back to front, without place or size (ADR 0059) |
//! | `canvas/info` | the canvas showing, its counts, and every canvas of the space (ADR 0059) |
//! | `canvas/mod_items` | the active canvas's mod items, without their positions (ADR 0057) |
//! | `tool` | the armed mod tool's stroke, or `null` (ADR 0057) |
//! | `call` | the last mod command run, `{command, number, args}`, or `null` |
//! | `agents` | the agents with a cursor on the canvas shown, `{agents: [{id, name, color, label}], following}`, without positions |
//!
//! A control's `action` is left out: a reader presses a control with
//! `control.press {id}`, by the id written beside it. The camera is in no
//! key, so a pan or a pinch moves no revision; the zoom readout is written
//! as its id alone.

mod chat;
mod keys;
mod panels;
mod revisions;

pub use self::chat::THREAD_KEY;
pub use self::revisions::Revisions;

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::commands::{Run, naming};
use crate::{
    App, Appearance, PaletteRun, context_menu, dock, palette, tool_options, toolbar, view_strip,
};

/// Every fixed key, in the order [`state`] lists them. Beside them, each
/// thread of the active canvas has a key of its own, [`THREAD_KEY`] and
/// its id: see [`is_state_key`].
pub const STATE_KEYS: [&str; 27] = [
    "panels/toolbar",
    "panels/tool_options",
    "panels/dock",
    "panels/strip",
    "panels/menu",
    "panels/palette",
    "panels/share",
    "panels/welcome",
    "panels/sidebar",
    "panels/chat",
    "chat/threads",
    "chat/context",
    "theme",
    "settings",
    "keys",
    "layout",
    "layouts",
    "look",
    "overlays",
    "notices",
    "selection",
    "canvas/items",
    "canvas/info",
    "canvas/mod_items",
    "tool",
    "call",
    "agents",
];

fn to_json(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// Whether `key` names a key: a fixed one, or a thread's. A thread's key
/// may name a thread there is not (yet): it reads as null.
pub fn is_state_key(key: &str) -> bool {
    STATE_KEYS.contains(&key)
        || key
            .strip_prefix(THREAD_KEY)
            .is_some_and(|id| !id.is_empty())
}

/// Every key of `app` and its value now: the fixed keys, then a key for
/// each thread of the active canvas.
pub fn state(app: &App) -> BTreeMap<String, Value> {
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
        ("panels/sidebar", panels::sidebar_json(app)),
        ("panels/chat", chat::panel_json(app)),
        ("chat/threads", chat::threads_json(app)),
        ("chat/context", chat::context_json(app)),
        ("theme", theme_json(app)),
        ("settings", settings_json(app)),
        ("keys", keys::bindings(&app.keymap)),
        ("layout", layout_json(app)),
        ("layouts", layouts_json(app)),
        (
            "look",
            app.layout().look.to_json(&app.look(), app.appearance()),
        ),
        ("overlays", crate::overlays::overlays_json(app)),
        ("notices", to_json(crate::notices(app))),
        ("selection", panels::selection_json(app)),
        ("canvas/items", panels::items_json(app)),
        ("canvas/info", panels::info_json(app)),
        ("canvas/mod_items", crate::mods::mod_items_json(app)),
        ("tool", crate::mods::tool_json(app)),
        ("call", crate::mods::call_json(app)),
        ("agents", crate::presence::agents_json(app)),
    ];
    (entries.into_iter())
        .map(|(key, value)| (key.to_owned(), value))
        .chain(chat::thread_keys(app))
        .collect()
}

/// The value of `key` now, or `None` for a key there is not. A thread's
/// key reads as null while its thread is not on the active canvas.
pub fn state_key(app: &App, key: &str) -> Option<Value> {
    let value = state(app).remove(key);
    value.or_else(|| is_state_key(key).then_some(Value::Null))
}

/// Publishes every key of `app` into `revisions`. Returns the keys whose
/// revision moved. A thread's key that was published and is gone now (the
/// thread was closed, or another canvas shows) moves to null.
pub fn publish(revisions: &mut Revisions, app: &App) -> Vec<String> {
    let now = state(app);
    let gone: Vec<String> = (revisions.keys())
        .filter(|key| key.starts_with(THREAD_KEY) && !now.contains_key(*key))
        .map(str::to_owned)
        .collect();
    let gone = gone.into_iter().map(|key| (key, Value::Null));
    (now.into_iter().chain(gone))
        .filter_map(|(key, value)| revisions.publish(&key, value).then_some(key))
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
            .fill(crate::Region::Toolbar)
            .is_none()
            .then(|| json!({ "id": "zoom" }));
        fields.insert("zoom".to_owned(), zoom.unwrap_or(Value::Null));
    }
    strip
}

fn palette_json(app: &App) -> Value {
    let items: Vec<Value> = (palette(app).into_iter())
        .map(|item| {
            let (command, args) = match item.run {
                PaletteRun::Action(action) => keys::command_of(&action)
                    .map(|(name, args)| (name.to_owned(), args))
                    .unzip(),
                PaletteRun::Shell(command) => (
                    naming(&Run::Shell(command)).map(|spec| spec.name.to_owned()),
                    None,
                ),
                PaletteRun::Named { name, args } => (Some(name), Some(args)),
            };
            json!({
                "label": item.label,
                "group": item.group,
                "command": command,
                "args": args.unwrap_or(Value::Null),
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
    let settings = &app.settings;
    json!({
        "showSidebar": settings.show_sidebar,
        "showChat": settings.show_chat,
        "tools": settings.tools,
        "hideAgentCursors": settings.hidden_cursors.agent,
        "hideSyncedCursors": settings.hidden_cursors.synced,
    })
}

fn layouts_json(app: &App) -> Value {
    let layouts: Vec<Value> = (app.layouts())
        .map(|(name, layout)| {
            json!({
                "name": name,
                "title": layout.title.as_deref().unwrap_or(name),
                "active": name == app.layout_name(),
            })
        })
        .collect();
    Value::Array(layouts)
}

fn layout_json(app: &App) -> Value {
    let regions = app.regions();
    let hidden: Vec<&str> = (crate::Region::ALL.into_iter())
        .filter(|region| !regions.shows(*region))
        .map(crate::Region::name)
        .collect();
    let file: Value = serde_json::from_str(&app.layout().to_text()).unwrap_or(Value::Null);
    json!({
        "name": app.layout_name(),
        "file": file,
        "toggles": to_json(app.layout_toggles()),
        "hidden": hidden,
    })
}
