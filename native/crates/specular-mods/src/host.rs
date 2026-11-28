//! The `specular` host module: what a mod imports to reach the app.
//!
//! ```js
//! import { state, run, commands, shared, setShared, props, notify, colors } from "specular";
//! import { Icon, Zoom, ToolOptions } from "specular";
//! ```
//!
//! Every mod gets its own instance of the module, built by
//! [`specular_module`] with the mod's id, so a read is recorded against the
//! mod that made it. A host function must not call back into the script
//! engine; none of these do.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use gpui_shell::{HostError, HostModule, HostValue};
use serde_json::Value;

use crate::chrome::{appearance_of, chrome_colors};
use crate::components::Components;
use crate::value::{from_host, to_host};
use crate::{Reads, Revisions};

/// Why a command a mod ran did not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunError(pub String);

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RunError {}

/// What the app offers a mod beyond its keyed state.
pub trait ModHost {
    /// Runs the command `name` with `args`, as the palette, a key or the
    /// CLI would. A command that changes the document is an undo step like
    /// any other.
    fn run(&self, name: &str, args: &Value) -> Result<Value, RunError>;

    /// Every command a mod may run: name, args and about.
    fn commands(&self) -> Value;

    /// A line for the person: `level` is `info`, `success`, `warning` or
    /// `error`.
    fn notify(&self, level: &str, text: &str);

    /// A mod wrote a value other mods may read. The host renders again
    /// every mod that read it.
    fn shared_changed(&self) {}
}

/// The module name a mod imports.
pub const MODULE: &str = "specular";

const DECLARATIONS: &str = include_str!("../specular.d.ts");

/// What every mod of one window shares: the app it reaches, the state it
/// reads, who read what, and the host components it places.
#[derive(Clone)]
pub struct Hosting {
    /// Runs commands and shows notices.
    pub host: Rc<dyn ModHost>,
    /// The keyed state, with a revision per key.
    pub revisions: Rc<RefCell<Revisions>>,
    /// Which keys each mod read.
    pub reads: Rc<RefCell<Reads>>,
    /// The Rust-built elements a mod places.
    pub components: Components,
}

impl Hosting {
    /// `host` over fresh state, with the window-less host components.
    pub fn new(host: Rc<dyn ModHost>) -> Self {
        let revisions = Rc::new(RefCell::new(Revisions::default()));
        Self {
            host,
            components: Components::standard(revisions.clone()),
            revisions,
            reads: Rc::new(RefCell::new(Reads::default())),
        }
    }
}

impl std::fmt::Debug for Hosting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hosting")
            .field("components", &self.components)
            .finish_non_exhaustive()
    }
}

/// The `specular` module for the mod `id`.
pub fn specular_module(id: &str, hosting: &Hosting) -> HostModule {
    let Hosting {
        host,
        revisions,
        reads,
        components,
    } = hosting.clone();
    let read = {
        let (id, revisions) = (id.to_owned(), revisions.clone());
        move |key: &str| -> HostValue {
            let revisions = revisions.borrow();
            reads.borrow_mut().record(&id, key, revisions.revision(key));
            revisions.get(key).map_or(HostValue::Null, to_host)
        }
    };
    let read = Rc::new(read);
    let props_key = format!("mods/{id}/props");
    let state = read.clone();
    let shared = read.clone();
    let theme = read.clone();
    let props = read;
    let runner = host.clone();
    let lister = host.clone();
    let notifier = host.clone();
    let writer = host;
    let mut module = HostModule::new(MODULE)
        .declarations(DECLARATIONS)
        .function("state", move |arguments| Ok(state(arguments.string(0)?)))
        .function("props", move |_| Ok(props(&props_key)))
        .function("colors", move |_| {
            let appearance = appearance_of(Some(&from_host(&theme("theme"))));
            Ok(to_host(&chrome_colors(appearance)))
        })
        .function("shared", move |arguments| {
            Ok(shared(&format!("shared/{}", arguments.string(0)?)))
        })
        .function("setShared", move |arguments| {
            let key = format!("shared/{}", arguments.string(0)?);
            let value = arguments.get(1).map_or(Value::Null, from_host);
            let changed = revisions.borrow_mut().publish(&key, value);
            if changed {
                writer.shared_changed();
            }
            Ok(HostValue::Bool(changed))
        })
        .function("run", move |arguments| {
            let name = arguments.string(0)?;
            let args = arguments.get(1).map_or(Value::Null, from_host);
            (runner.run(name, &args))
                .map(|value| to_host(&value))
                .map_err(|error| HostError::new(format!("{name}: {error}")))
        })
        .function("commands", move |_| Ok(to_host(&lister.commands())))
        .function("notify", move |arguments| {
            let text = arguments.string(0)?;
            let level = arguments
                .get(1)
                .and_then(HostValue::as_str)
                .unwrap_or("info");
            notifier.notify(level, text);
            Ok(HostValue::Null)
        });
    for (name, build) in components.iter() {
        let build = build.clone();
        module = module.component(name, move |args, window, cx| build(args, window, cx));
    }
    module
}
