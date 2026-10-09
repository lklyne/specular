//! [`Driver`]: an [`App`] with scripted hands. Pointer, keys, text, wheel
//! and pinch go in as the events a window would send, through [`update`],
//! and the effects that come back pile up for the caller to run or read.
//!
//! The headless runner's script steps and the testkit's `TestApp` are both
//! this with their own things around it.

use std::sync::Arc;

use glam::Vec2;
use specular_core::{Camera, ImeEvent, Modifiers, PointerButton, PointerEventKind};
use specular_doc::{Document, EdgeId, EntityId, ItemId};

use crate::{
    Action, App, ClipboardContent, Effect, Event, Key, KeyInput, NoteNotice, PhysicalKey,
    PointerInput, Session, SidebarAction, TextMeasure, Tool, UnknownControl, WheelInput,
    control_named, mac_key_input, update,
};

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
/// Control.
pub const CTRL: Modifiers = Modifiers {
    control: true,
    ..NONE
};
/// Option.
pub const ALT: Modifiers = Modifiers { alt: true, ..NONE };
/// Command.
pub const CMD: Modifiers = Modifiers { meta: true, ..NONE };
/// Command and Shift.
pub const CMD_SHIFT: Modifiers = Modifiers {
    shift: true,
    meta: true,
    ..NONE
};

/// An [`App`] driven by scripted input, with no window behind it.
#[derive(Debug, Clone)]
pub struct Driver {
    app: App,
    /// What `update` has returned since the last drain.
    effects: Vec<Effect>,
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
        Self {
            app,
            effects: Vec::new(),
            pointer: Vec2::ZERO,
            held: NONE,
        }
    }

    /// The app being driven.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The document.
    pub fn document(&self) -> &Document {
        self.app.document()
    }

    /// The unsaved state: camera, tool, gesture, hover, focus.
    pub fn session(&self) -> &Session {
        self.app.session()
    }

    /// Lays text out with `measure` from here on.
    pub fn measure_with(&mut self, measure: Arc<dyn TextMeasure>) -> &mut Self {
        self.app.set_text_measure(measure);
        self
    }

    /// Sends one event through [`update`] and keeps the effects. Every other
    /// input method ends up here.
    pub fn send(&mut self, event: Event) -> &mut Self {
        let effects = update(&mut self.app, event);
        self.effects.extend(effects);
        self
    }

    /// Runs an [`Action`], as a menu item, a panel or the API would.
    pub fn act(&mut self, action: Action) -> &mut Self {
        self.send(Event::Action(action))
    }

    /// The effects returned since the last [`Driver::take_effects`].
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// Drains the effects returned since the last drain.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    /// Puts `effects` in place of the ones kept: what a caller took, ran
    /// something it did not want the effects of, and gives back.
    pub fn set_effects(&mut self, effects: Vec<Effect>) -> &mut Self {
        self.effects = effects;
        self
    }
}

/// Pointer positions are logical screen pixels and take a tuple:
/// `app.press((200.0, 150.0))`.
impl Driver {
    /// Holds these modifier keys down for every event until
    /// [`Driver::let_go`]. Replaces the ones held before.
    pub fn hold(&mut self, modifiers: Modifiers) -> &mut Self {
        self.held = modifiers;
        self
    }

    /// Lets go of every modifier key.
    pub fn let_go(&mut self) -> &mut Self {
        self.hold(NONE)
    }

    fn pointer(&mut self, kind: PointerEventKind) -> &mut Self {
        self.send(Event::Pointer(PointerInput {
            kind,
            screen: self.pointer,
            modifiers: self.held,
        }))
    }

    fn button(&mut self, pressed: bool, click_count: u8) -> &mut Self {
        let button = PointerButton::Left;
        self.pointer(if pressed {
            PointerEventKind::Down {
                button,
                click_count,
            }
        } else {
            PointerEventKind::Up {
                button,
                click_count,
            }
        })
    }

    /// Moves the pointer to `at`.
    pub fn pointer_move(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.pointer = at.into();
        self.pointer(PointerEventKind::Move)
    }

    /// Presses the left button at `at`. No move event comes first.
    pub fn press(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.pointer = at.into();
        self.button(true, 1)
    }

    /// Moves the pointer to `at` with the button still down. The same event
    /// as [`Driver::pointer_move`], named for how a drag reads.
    pub fn drag_to(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.pointer_move(at)
    }

    /// Releases the left button where the pointer is.
    pub fn release(&mut self) -> &mut Self {
        self.button(false, 1)
    }

    /// A whole drag: press at `from`, move to `to`, release.
    pub fn drag(&mut self, from: impl Into<Vec2>, to: impl Into<Vec2>) -> &mut Self {
        self.press(from).drag_to(to).release()
    }

    /// Presses and releases the left button at `at`.
    pub fn click(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.press(at).release()
    }

    /// Two clicks at `at`, the second with a click count of 2.
    pub fn double_click(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.click(at).button(true, 2).button(false, 2)
    }

    /// Three clicks at `at`, with click counts of 1, 2 and 3.
    pub fn triple_click(&mut self, at: impl Into<Vec2>) -> &mut Self {
        self.double_click(at).button(true, 3).button(false, 3)
    }

    /// The pointer leaves the window.
    pub fn pointer_leave(&mut self) -> &mut Self {
        self.pointer(PointerEventKind::Leave)
    }

    fn key_event(&mut self, key: Key, pressed: bool, extra: Modifiers) -> &mut Self {
        let modifiers = both(self.held, extra);
        let typed = match key {
            Key::Char(character) if modifiers.shift => Some(character.to_ascii_uppercase()),
            Key::Char(character) => Some(character),
            Key::Space => Some(' '),
            Key::Escape
            | Key::Enter
            | Key::Tab
            | Key::Backspace
            | Key::Delete
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Other => None,
        };
        // Command and Control turn a key into a shortcut, which types nothing.
        let types = pressed && !modifiers.meta && !modifiers.control;
        let text = typed.filter(|_| types).map(String::from);
        // The codes and the character a US keyboard sends for the key, so a
        // page gets the event a shell would give it.
        let input = match PhysicalKey::for_key(key) {
            Some(physical) => {
                let characters = typed.map(String::from).unwrap_or_default();
                let code = physical.mac_key_code();
                KeyInput {
                    text,
                    ..mac_key_input(code, pressed, false, &characters, modifiers)
                }
            }
            None => KeyInput {
                key,
                pressed,
                repeat: false,
                text,
                character: None,
                modifiers,
                windows_key_code: 0,
                native_key_code: 0,
                commands: Vec::new(),
            },
        };
        self.send(Event::Key(input))
    }

    /// Presses `key` and leaves it down.
    pub fn key_down(&mut self, key: Key) -> &mut Self {
        self.key_event(key, true, NONE)
    }

    /// Releases `key`.
    pub fn key_up(&mut self, key: Key) -> &mut Self {
        self.key_event(key, false, NONE)
    }

    /// Presses and releases `key`.
    pub fn key(&mut self, key: Key) -> &mut Self {
        self.chord(NONE, key)
    }

    /// Presses and releases `key` with `modifiers` held for just this key,
    /// on top of any from [`Driver::hold`]: `app.chord(CMD, Key::Char('z'))`.
    pub fn chord(&mut self, modifiers: Modifiers, key: Key) -> &mut Self {
        self.key_event(key, true, modifiers)
            .key_event(key, false, modifiers)
    }

    /// Types `text` one key at a time, as a US keyboard would.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for character in text.chars() {
            let (key, shift) = key_for(character);
            self.chord(if shift { SHIFT } else { NONE }, key);
        }
        self
    }

    /// The input method shows `text` as its composition so far, with its
    /// caret at the end.
    pub fn compose(&mut self, text: &str) -> &mut Self {
        let end = text.encode_utf16().count() as u32;
        self.send(Event::Ime(ImeEvent::SetComposition {
            text: text.to_owned(),
            selection: end..end,
            replacement: None,
        }))
    }

    /// The input method commits `text`, ending any composition.
    pub fn commit(&mut self, text: &str) -> &mut Self {
        self.send(Event::Ime(ImeEvent::Commit {
            text: text.to_owned(),
            replacement: None,
        }))
    }

    /// The shell answers a paste with the clipboard's text.
    pub fn paste(&mut self, text: &str) -> &mut Self {
        self.send(Event::Clipboard(ClipboardContent {
            text: Some(text.to_owned()),
            image: None,
        }))
    }

    /// Scrolls by `delta` logical pixels where the pointer is. Positive `y`
    /// moves content down.
    pub fn wheel(&mut self, delta: impl Into<Vec2>) -> &mut Self {
        self.send(Event::Wheel(WheelInput {
            delta: delta.into(),
            modifiers: self.held,
        }))
    }

    /// A trackpad pinch. Positive `delta` zooms in.
    pub fn pinch(&mut self, delta: f32) -> &mut Self {
        self.send(Event::Pinch { delta })
    }

    /// The shell read the markdown file `file`, or saw it change: it holds
    /// `text`.
    pub fn note_text(&mut self, file: &str, text: &str) -> &mut Self {
        self.send(Event::Note {
            file: file.to_owned(),
            notice: NoteNotice::Text(text.to_owned()),
        })
    }

    /// Advances the wall clock to `unix_ms`.
    pub fn tick(&mut self, unix_ms: u64) -> &mut Self {
        self.send(Event::Tick { unix_ms })
    }

    /// Sets the canvas viewport size in logical pixels.
    pub fn viewport(&mut self, size: impl Into<Vec2>) -> &mut Self {
        self.send(Event::ViewportResized(size.into()))
    }

    /// Puts the camera at `zoom` with no pan.
    pub fn zoom(&mut self, zoom: f32) -> &mut Self {
        self.act(Action::SetCamera(Camera::new(Vec2::ZERO, zoom)))
    }

    /// Switches tool, as the toolbar does.
    pub fn tool(&mut self, tool: Tool) -> &mut Self {
        self.act(Action::SetTool(tool))
    }

    /// Selects these entities and edges, replacing the selection.
    pub fn select(&mut self, ids: &[&str]) -> &mut Self {
        let items = ids.iter().map(|id| {
            if self.document().edge(&EdgeId::from(*id)).is_some() {
                ItemId::Edge(EdgeId::from(*id))
            } else {
                ItemId::Entity(EntityId::from(*id))
            }
        });
        self.act(Action::Select(items.collect()))
    }

    /// Moves the pointer to `at` and presses and releases the right button
    /// there.
    pub fn right_click(&mut self, at: impl Into<Vec2>) -> &mut Self {
        let at = at.into();
        self.pointer_move(at).press_button(PointerButton::Right, at);
        self.pointer(PointerEventKind::Up {
            button: PointerButton::Right,
            click_count: 1,
        })
    }

    /// Does what a click on the control named `name` does, with the held
    /// modifiers and no layout: the path a run with the panels off takes.
    /// A name no control has is an error that lists the ones there are.
    pub fn control(&mut self, name: &str) -> Result<&mut Self, UnknownControl> {
        let control = control_named(&self.app, name)?;
        Ok(self.send(Event::Control(control, self.held)))
    }

    /// A right click at `at` with no layout: the context menu opens for
    /// what is there, and a press that opens none goes where a right press
    /// goes.
    pub fn context_menu(&mut self, at: impl Into<Vec2>) -> &mut Self {
        let at = at.into();
        self.pointer_move(at).send(Event::ContextMenu(at));
        if self.session().panel.menu.is_none() {
            self.right_click(at);
        }
        self
    }

    /// Presses a button other than the left one at `at`.
    pub fn press_button(&mut self, button: PointerButton, at: impl Into<Vec2>) -> &mut Self {
        self.pointer = at.into();
        self.pointer(PointerEventKind::Down {
            button,
            click_count: 1,
        })
    }

    /// Shows or hides the sidebar, as its toolbar button does. It starts
    /// hidden.
    pub fn show_sidebar(&mut self, shown: bool) -> &mut Self {
        if self.session().sidebar.shown() != shown {
            self.act(Action::Sidebar(SidebarAction::Toggle));
        }
        self
    }
}
