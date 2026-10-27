//! [`Driver`]: scripted hands on an app. Pointer, keys, text, wheel and
//! pinch go in as the events a window would send, to an [`EventSink`].
//!
//! The sink is [`Owned`] by default: an [`App`] of the driver's own, whose
//! effects pile up for the caller to read, as the testkit's `TestApp` does.
//! The headless runner's sink is the shell's runtime, which runs each
//! effect as a window's does.

mod input;

use glam::Vec2;
use specular_core::Modifiers;
use specular_doc::Document;

use crate::{Action, App, Effect, Event, Key, Session, update};

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};
/// Shift.
pub const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
/// Command.
pub const CMD: Modifiers = Modifiers { meta: true, ..NONE };

/// Where a [`Driver`]'s events go, and the app they change.
pub trait EventSink {
    /// The app the events change.
    fn app(&self) -> &App;

    /// Sends one event through [`update`].
    fn send(&mut self, event: Event);
}

/// An app of the driver's own, and what `update` has returned since the
/// last drain.
#[derive(Debug, Clone)]
pub struct Owned {
    app: App,
    effects: Vec<Effect>,
}

impl EventSink for Owned {
    fn app(&self) -> &App {
        &self.app
    }

    fn send(&mut self, event: Event) {
        let effects = update(&mut self.app, event);
        self.effects.extend(effects);
    }
}

/// An app driven by scripted input, with no window behind it.
#[derive(Debug, Clone)]
pub struct Driver<S = Owned> {
    sink: S,
    /// Where the pointer was last put, in logical screen pixels.
    pointer: Vec2,
    /// The modifier keys held down.
    held: Modifiers,
}

fn both(a: Modifiers, b: Modifiers) -> Modifiers {
    Modifiers {
        shift: a.shift || b.shift,
        control: a.control || b.control,
        alt: a.alt || b.alt,
        meta: a.meta || b.meta,
    }
}

/// The key that types `character` on a US layout, and whether it needs Shift.
fn key_for(character: char) -> (Key, bool) {
    match character {
        ' ' => (Key::Space, false),
        '\n' => (Key::Enter, false),
        '\t' => (Key::Tab, false),
        _ if character.is_uppercase() => (Key::Char(character.to_ascii_lowercase()), true),
        _ => (Key::Char(character), false),
    }
}

impl Driver {
    /// Drives `app` from where it stands, with the pointer at the origin
    /// and no key held.
    pub fn new(app: App) -> Self {
        Self::over(Owned {
            app,
            effects: Vec::new(),
        })
    }

    /// Drains the effects returned since the last drain.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.sink.effects)
    }

    /// Puts `effects` in place of the ones kept: what a caller took, ran
    /// something it did not want the effects of, and gives back.
    pub fn set_effects(&mut self, effects: Vec<Effect>) -> &mut Self {
        self.sink.effects = effects;
        self
    }
}

impl<S: EventSink> Driver<S> {
    /// Drives the app behind `sink`, with the pointer at the origin and no
    /// key held.
    pub fn over(sink: S) -> Self {
        Self {
            sink,
            pointer: Vec2::ZERO,
            held: NONE,
        }
    }

    /// Where the events go.
    pub fn sink(&self) -> &S {
        &self.sink
    }

    /// Where the events go, to change.
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }

    /// The app being driven.
    pub fn app(&self) -> &App {
        self.sink.app()
    }

    /// The document.
    pub fn document(&self) -> &Document {
        self.app().document()
    }

    /// The unsaved state: camera, tool, gesture, hover, focus.
    pub(crate) fn session(&self) -> &Session {
        self.app().session()
    }

    /// Sends one event to the sink. Every other input method ends up here.
    pub fn send(&mut self, event: Event) -> &mut Self {
        self.sink.send(event);
        self
    }

    /// Runs an [`Action`], as a menu item, a panel or the API would.
    pub fn act(&mut self, action: Action) -> &mut Self {
        self.send(Event::Action(action))
    }
}
