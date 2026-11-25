//! A chrome panel as a Luau script (step S7, ADR 0055 lane 1).
//!
//! Three things cross the VM boundary and nothing else does:
//!
//! - **Down, the snapshot.** [`snapshot`] turns `toolbar(&App)` into plain
//!   [`Data`]: ids, labels, kinds, on/selected, enabled, options. The host
//!   fires it at the script as the `toolbar` signal of `@specular/app` when
//!   the model differs from the one last sent, and at no other time. Firing
//!   is what moves the script's version, so it is also what invalidates the
//!   memoized render: no fire, no entry into the VM.
//! - **Up, `run(name, args)`.** The one host function. It queues the call;
//!   the host carries it out once the script's handler has returned, by
//!   looking `name` up among the controls of the model as it is then and
//!   sending that control's own `Action` through `update`.
//! - **Elements.** The script's `render` builds Rune elements with
//!   `@rune/ui`, plus `ui.control`, the host node below, for the roles and
//!   states `@rune/ui` has no prop for.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use rune::{Cx, El, HasPopup, Tx, Waker, col, row};
use rune_luau::{Data, HostNode, NodeArgs, Palette, Prop, Script, ScriptOptions, Sources};
use specular_interact::{
    Action, App, Control, Dropdown, DropdownSection, ToolbarModel, ToolbarSection, toolbar,
};

use crate::chrome;
use crate::shell::Shell;

/// The module a panel requires for the app: `toolbar` (a signal) and `run`.
const MODULE: &str = "@specular/app";
/// The signal the toolbar snapshot arrives on.
const TOOLBAR: &str = "toolbar";

/// What a script asked for with `run(name, args)`, not yet carried out.
type Runs = Rc<RefCell<Vec<(String, Data)>>>;

/// A scripted panel and the host's side of its boundary.
pub struct Panel {
    /// The VM.
    pub script: Script,
    /// The model the script last received.
    sent: RefCell<Option<ToolbarModel>>,
    /// Set by each new VM (a start or a reload): it has no snapshot yet.
    fresh: Rc<Cell<bool>>,
    runs: Runs,
    /// How many snapshots have been sent.
    pub sends: Cell<u64>,
}

impl Panel {
    /// The toolbar panel from the script folder `dir`.
    pub fn from_dir(dir: &Path) -> std::io::Result<Self> {
        Ok(Self::new(Sources::from_dir(dir)?))
    }

    /// The toolbar panel from `sources`.
    pub fn new(sources: Sources) -> Self {
        let runs = Runs::default();
        let fresh = Rc::new(Cell::new(true));
        let script = Script::with_options("toolbar", sources, options(&runs, &fresh));
        Self {
            script,
            sent: RefCell::new(None),
            fresh,
            runs,
            sends: Cell::new(0),
        }
    }

    /// Reloads the panel when a file under `dir` is saved, and lets the
    /// watcher ask for a frame.
    pub fn watch(&self, dir: &Path) {
        if let Err(error) = self.script.watch(dir) {
            eprintln!("rune-shell: not watching {}: {error}", dir.display());
        }
    }

    /// Hands the script the window's waker.
    pub fn set_waker(&self, waker: Waker) {
        self.script.set_waker(waker);
    }

    /// Sends the toolbar snapshot if the model differs from the one the
    /// running VM holds. The comparison is the model's own `PartialEq`, on
    /// the whole toolbar: one key, as coarse as the panel.
    pub fn sync(&self, app: &App) {
        let model = toolbar(app);
        let fresh = self.fresh.take();
        if !fresh && self.sent.borrow().as_ref() == Some(&model) {
            return;
        }
        self.script.fire(TOOLBAR, [snapshot(&model)]);
        self.sends.set(self.sends.get() + 1);
        *self.sent.borrow_mut() = Some(model);
    }

    /// The panel's elements. Memoized on the script's version: with no
    /// snapshot sent and no handler run since the last call, the VM is not
    /// entered.
    pub fn view(&self, shell: &Shell, cx: &Cx<Shell>) -> El {
        self.sync(&shell.app);
        self.script.view(cx)
    }

    /// What the script has asked for since the last call.
    fn take_runs(&self) -> Vec<(String, Data)> {
        std::mem::take(&mut self.runs.borrow_mut())
    }
}

/// `AppSpec::poll` for the panel: due tasks and a watched reload, then
/// whatever a task asked to run. True when script code ran.
pub fn poll(shell: &mut Shell, now: f64, tx: &mut Tx) -> bool {
    let Some(panel) = &shell.panel else {
        return false;
    };
    let ran = panel.script.poll(now, tx);
    let runs = panel.take_runs();
    carry_out(shell, runs);
    ran
}

/// Carries out `run` calls: each name is a control of the toolbar as the
/// model has it now, and what runs is that control's own `Action`, through
/// the path a press on the Rust toolbar takes. A name no enabled control
/// has does nothing. `args` is unused: no toolbar action takes one.
fn carry_out(shell: &mut Shell, runs: Vec<(String, Data)>) {
    for (name, _args) in runs {
        let model = toolbar(&shell.app);
        match action_of(&model, &name) {
            Some(action) => shell.act(action),
            None => eprintln!("rune-shell: run(\"{name}\"): no enabled control has that name"),
        }
    }
}

fn options(runs: &Runs, fresh: &Rc<Cell<bool>>) -> ScriptOptions {
    let queue = runs.clone();
    let drain = runs.clone();
    let fresh = fresh.clone();
    ScriptOptions::new()
        .palette(palette())
        .node(control_node())
        .on_log(|level, line| eprintln!("rune-shell: luau {level:?}: {line}"))
        .setup(move |host| {
            let lua = host.lua();
            let queue = queue.clone();
            let run = lua.create_function(move |_, (name, args): (String, Option<Data>)| {
                queue.borrow_mut().push((name, args.unwrap_or_default()));
                Ok(())
            })?;
            let module = lua.create_table()?;
            module.raw_set("run", run)?;
            module.raw_set(TOOLBAR, host.signal(TOOLBAR)?)?;
            host.register_module(MODULE, module)
        })
        .start(move |_, _, _| {
            fresh.set(true);
            Ok(())
        })
        // A press handler runs inside `run`; what it queued is carried out
        // straight after, on the same state and in the same transaction.
        .around(move |state: &mut dyn Any, tx, run| {
            run(tx);
            let runs = std::mem::take(&mut *drain.borrow_mut());
            if let Some(shell) = state.downcast_mut::<Shell>() {
                carry_out(shell, runs);
            }
        })
}

/// The Rust toolbar's colours under names, so the script says `fill =
/// "surface"` and draws what `chrome.rs` draws. Other names are Rune's.
fn palette() -> Palette {
    let runes = rune_luau::default_palette();
    Rc::new(move |name, dark| {
        Some(match name {
            "surface" => chrome::surface(),
            "edge" => chrome::edge(),
            "ink" => chrome::ink(),
            "ink-strong" => chrome::ink_strong(),
            "raised" => chrome::raised(),
            "hover" => chrome::hover(),
            _ => return runes(name, dark),
        })
    })
}

/// `ui.control(props, children)`: a row (a column with `column = true`)
/// that carries what `@rune/ui` cannot say: any ARIA role Rune has, a
/// `checked` state and `hasPopup`. Layout, colours, `key`, `label` and the
/// rest are the standard props, applied after this.
fn control_node() -> HostNode {
    HostNode::new("control", |args: &mut NodeArgs| {
        let mut el = if args.flag("column") { col() } else { row() };
        if let Some(name) = args.string("role") {
            el = el.role(role(name).ok_or_else(|| format!("unknown role {name}"))?);
        }
        if let Some(Prop::Bool(on)) = args.get("checked") {
            el = el.checked(*on);
        }
        if args.string("hasPopup") == Some("menu") {
            el = el.has_popup(HasPopup::Menu);
        }
        el = args.on_press(el, "onPress");
        Ok(el.children(std::mem::take(&mut args.children)))
    })
    .props(|name| matches!(name, "column" | "role" | "checked" | "hasPopup" | "onPress"))
}

/// The roles the chrome uses. Rune takes a role as a `&'static str`.
fn role(name: &str) -> Option<&'static str> {
    Some(match name {
        "button" => "button",
        "toolbar" => "toolbar",
        "menu" => "menu",
        "menuitem" => "menuitem",
        "menuitemradio" => "menuitemradio",
        "menuitemcheckbox" => "menuitemcheckbox",
        "group" => "group",
        _ => return None,
    })
}

fn map<const N: usize>(fields: [(&str, Data); N]) -> Data {
    Data::Map(
        fields
            .into_iter()
            .filter(|(_, value)| *value != Data::Nil)
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn text(value: Option<&str>) -> Data {
    value.map_or(Data::Nil, Data::from)
}

/// A pressable control as plain data. `on` is `Nil` for a plain button.
fn pressable(id: &str, label: &str, word: Option<&str>, on: Data, enabled: bool) -> Data {
    let kind = if on == Data::Nil { "button" } else { "toggle" };
    map([
        ("kind", kind.into()),
        ("id", id.into()),
        ("label", label.into()),
        ("text", text(word)),
        ("on", on),
        ("enabled", enabled.into()),
    ])
}

fn dropdown(model: &Dropdown) -> Data {
    let mut items = Vec::new();
    for section in &model.content {
        match section {
            DropdownSection::Options { options, .. } => {
                items.extend(options.iter().map(|option| {
                    map([
                        ("kind", "option".into()),
                        ("id", option.id.as_str().into()),
                        ("label", option.label.as_ref().into()),
                        ("text", text(option.face.text.as_deref())),
                        ("selected", option.selected.into()),
                        ("enabled", option.enabled.into()),
                    ])
                }));
            }
            DropdownSection::Controls(controls) => items.extend(controls.iter().map(control)),
        }
    }
    map([
        ("kind", "dropdown".into()),
        ("id", model.id.as_str().into()),
        ("label", model.label.as_ref().into()),
        ("text", text(model.summary.text.as_deref())),
        ("options", Data::List(items)),
    ])
}

fn control(model: &Control) -> Data {
    match model {
        Control::Button(button) => pressable(
            button.id.as_str(),
            &button.label,
            button.face.text.as_deref(),
            Data::Nil,
            button.enabled,
        ),
        Control::Toggle(toggle) => pressable(
            toggle.id.as_str(),
            &toggle.label,
            toggle.face.text.as_deref(),
            toggle.on.into(),
            toggle.enabled,
        ),
        Control::Dropdown(model) => dropdown(model),
        Control::Separator => map([("kind", "separator".into())]),
        // The kinds the toolbar never holds, as in `chrome.rs`.
        Control::Swatches(_) | Control::Stepper(_) | Control::Field(_) | Control::Choices(_) => {
            map([("kind", "unsupported".into())])
        }
    }
}

/// The toolbar model as plain data: what the script reads. It holds no
/// `Action`, no icon and no chord, and nothing of the camera but the zoom
/// readout's own text.
pub fn snapshot(model: &ToolbarModel) -> Data {
    let sections = model.sections.iter().map(|section| match section {
        ToolbarSection::Tools(tools) => map([
            ("kind", "tools".into()),
            (
                "controls",
                Data::List(
                    tools
                        .iter()
                        .map(|tool| {
                            pressable(
                                tool.id.as_str(),
                                &tool.label,
                                None,
                                tool.active.into(),
                                true,
                            )
                        })
                        .collect(),
                ),
            ),
        ]),
        ToolbarSection::Zoom(zoom) => map([
            ("kind", "zoom".into()),
            ("controls", Data::List(vec![dropdown(zoom)])),
        ]),
    });
    let copy = &model.share.copy;
    map([
        ("sections", Data::List(sections.collect())),
        (
            "theme",
            pressable(
                model.theme.id.as_str(),
                &model.theme.label,
                None,
                Data::Nil,
                true,
            ),
        ),
        (
            "share",
            pressable(copy.id.as_str(), &copy.label, None, Data::Nil, true),
        ),
        (
            "chat",
            model.chat.as_ref().map_or(Data::Nil, |chat| {
                pressable(chat.id.as_str(), &chat.label, None, chat.open.into(), true)
            }),
        ),
        ("view", Data::List(model.view.iter().map(control).collect())),
    ])
}

fn action_in_dropdown(model: &Dropdown, name: &str) -> Option<Action> {
    model.content.iter().find_map(|section| match section {
        DropdownSection::Options { options, .. } => options
            .iter()
            .find(|option| option.id.as_str() == name && option.enabled)
            .map(|option| option.action.clone()),
        DropdownSection::Controls(controls) => {
            controls.iter().find_map(|model| action_in(model, name))
        }
    })
}

fn action_in(model: &Control, name: &str) -> Option<Action> {
    match model {
        Control::Button(button) => {
            (button.id.as_str() == name && button.enabled).then(|| button.action.clone())
        }
        Control::Toggle(toggle) => {
            (toggle.id.as_str() == name && toggle.enabled).then(|| toggle.action.clone())
        }
        Control::Dropdown(model) => action_in_dropdown(model, name),
        Control::Separator
        | Control::Swatches(_)
        | Control::Stepper(_)
        | Control::Field(_)
        | Control::Choices(_) => None,
    }
}

/// What the enabled control named `name` does, if the toolbar has one. The
/// names are the ids [`snapshot`] hands out, so the script can only ask for
/// what the model offers now.
pub fn action_of(model: &ToolbarModel, name: &str) -> Option<Action> {
    let in_sections = model.sections.iter().find_map(|section| match section {
        ToolbarSection::Tools(tools) => tools
            .iter()
            .find(|tool| tool.id.as_str() == name)
            .map(|tool| tool.action.clone()),
        ToolbarSection::Zoom(zoom) => action_in_dropdown(zoom, name),
    });
    in_sections
        .or_else(|| (model.theme.id.as_str() == name).then(|| model.theme.action.clone()))
        .or_else(|| (model.share.copy.id.as_str() == name).then(|| model.share.copy.action.clone()))
        .or_else(|| {
            let chat = model.chat.as_ref()?;
            (chat.id.as_str() == name).then(|| chat.action.clone())
        })
        .or_else(|| model.view.iter().find_map(|model| action_in(model, name)))
}
