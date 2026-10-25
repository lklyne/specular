//! The pointer gestures and keys a driver sends.

use glam::Vec2;
use specular_core::{Camera, ImeEvent, Modifiers, PointerButton, PointerEventKind};
use specular_doc::{EdgeId, EntityId, ItemId};

use super::{Driver, EventSink, NONE, SHIFT, both, key_for};
use crate::{
    Action, Event, Key, KeyInput, PhysicalKey, PointerInput, SidebarAction, Tool, UnknownControl,
    WheelInput, control_named, mac_key_input,
};

/// Pointer positions are logical screen pixels and take a tuple:
/// `app.press((200.0, 150.0))`.
impl<S: EventSink> Driver<S> {
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
    pub(crate) fn right_click(&mut self, at: impl Into<Vec2>) -> &mut Self {
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
        let control = control_named(self.app(), name)?;
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
    pub(crate) fn press_button(&mut self, button: PointerButton, at: impl Into<Vec2>) -> &mut Self {
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
