//! Shaping input for a page: the key sequence Chromium expects, and which
//! page is owed each held button's release.

use specular_core::{EditingKey, InputEvent, KeyEvent, KeyEventKind, PageEdit, PointerButton};
use specular_doc::EntityId;

use crate::{App, Effect, Focus, KeyInput, PhysicalKey, bindings};

/// Which page received each held button's press, so its release goes to the
/// same page (pointer capture per button) wherever the pointer ends up.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ButtonCapture {
    pages: [Option<EntityId>; 3],
}

impl ButtonCapture {
    fn slot(button: PointerButton) -> usize {
        match button {
            PointerButton::Left => 0,
            PointerButton::Middle => 1,
            PointerButton::Right => 2,
        }
    }

    /// Records that `page` received `button`'s press.
    pub(crate) fn press(&mut self, button: PointerButton, page: EntityId) {
        self.pages[Self::slot(button)] = Some(page);
    }

    /// The page that got the press of a button still held, if any.
    pub(crate) fn holder(&self) -> Option<&EntityId> {
        self.pages.iter().flatten().next()
    }

    /// The page owed `button`'s release, if a page received its press.
    pub(crate) fn release(&mut self, button: PointerButton) -> Option<EntityId> {
        self.pages[Self::slot(button)].take()
    }
}

/// Does an Edit menu command inside the entered page. Nothing without one.
pub(crate) fn edit(app: &App, edit: PageEdit, effects: &mut Vec<Effect>) {
    if let Focus::Page(page) = &app.session.focus {
        effects.push(Effect::ForwardInput {
            page: page.clone(),
            event: InputEvent::Edit(edit),
        });
    }
}

/// The editing commands a press of `input` carries into a page: the ones
/// the platform's key bindings resolved it to, or the Edit menu command it
/// is the key equivalent of. A browser gets the first from `AppKit` as the
/// key comes in and the second from its menu bar once the page has let the
/// key go; here both ride the key-down, which the page's own handler still
/// sees first.
fn editing(input: &KeyInput) -> Option<EditingKey> {
    let commands = if input.commands.is_empty() {
        vec![bindings::page_edit_for(input)?.command().to_owned()]
    } else {
        input.commands.clone()
    };
    let physical =
        (u16::try_from(input.native_key_code).ok()).and_then(PhysicalKey::from_mac_key_code);
    Some(EditingKey {
        commands,
        code: physical.map_or("", PhysicalKey::dom_code),
        key: physical.map_or_else(
            || input.character.map(String::from).unwrap_or_default(),
            |physical| physical.dom_key(input.character),
        ),
    })
}

/// Appends the key sequence for `input` to `page`: a raw key-down and one
/// character event per produced character on press, a key-up on release.
/// Command-key chords produce no character events, as on macOS where they
/// are shortcuts rather than text, and neither does a press that carries
/// editing commands: the command is what the key does.
///
/// The key-down and the key-up carry the key's own character. CEF on macOS
/// builds an `NSEvent` from each, and one with no character is a change of
/// modifiers, which on a key that is not a modifier it reads as a press.
pub(crate) fn forward_key(page: &EntityId, input: &KeyInput, effects: &mut Vec<Effect>) {
    let mut push = |kind, windows_key_code, character, editing| {
        effects.push(Effect::ForwardInput {
            page: page.clone(),
            event: InputEvent::Key(KeyEvent {
                kind,
                windows_key_code,
                native_key_code: input.native_key_code,
                character,
                modifiers: input.modifiers,
                editing,
            }),
        });
    };
    if !input.pressed {
        push(
            KeyEventKind::Up,
            input.windows_key_code,
            input.character,
            None,
        );
        return;
    }
    let editing = editing(input);
    let typed = editing.is_none() && !input.modifiers.meta;
    push(
        KeyEventKind::RawDown,
        input.windows_key_code,
        input.character,
        editing,
    );
    if !typed {
        return;
    }
    for character in input.text.as_deref().unwrap_or_default().chars() {
        push(KeyEventKind::Char, character as i32, Some(character), None);
    }
}

#[cfg(test)]
mod tests {
    use specular_core::Modifiers;

    use super::*;
    use crate::Key;

    fn key(pressed: bool, text: Option<&str>, modifiers: Modifiers) -> KeyInput {
        KeyInput {
            key: Key::Char('a'),
            pressed,
            repeat: false,
            text: text.map(str::to_owned),
            character: None,
            modifiers,
            windows_key_code: 0x41,
            native_key_code: 0,
            commands: Vec::new(),
        }
    }

    fn kinds(input: &KeyInput) -> Vec<KeyEventKind> {
        let mut effects = Vec::new();
        forward_key(&EntityId::from("p1"), input, &mut effects);
        effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::ForwardInput {
                    event: InputEvent::Key(key),
                    ..
                } => Some(key.kind),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_letter_press_sends_raw_down_then_char_and_a_command_chord_only_raw_down() {
        let meta = Modifiers {
            meta: true,
            ..Modifiers::default()
        };
        let rows = [
            (
                key(true, Some("a"), Modifiers::default()),
                vec![KeyEventKind::RawDown, KeyEventKind::Char],
            ),
            (key(true, Some("a"), meta), vec![KeyEventKind::RawDown]),
            (
                key(false, None, Modifiers::default()),
                vec![KeyEventKind::Up],
            ),
        ];
        for (input, want) in rows {
            assert_eq!(kinds(&input), want, "pressed {}", input.pressed);
        }
    }

    /// The raw key-down's commands and DOM names, and how many character
    /// events follow it.
    fn sent(input: &KeyInput) -> (Vec<String>, &'static str, String, usize) {
        let mut effects = Vec::new();
        forward_key(&EntityId::from("p1"), input, &mut effects);
        let keys: Vec<&KeyEvent> = effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::ForwardInput {
                    event: InputEvent::Key(key),
                    ..
                } => Some(key),
                _ => None,
            })
            .collect();
        let editing = keys[0].editing.clone();
        let (commands, code, key) = editing.map_or_else(Default::default, |editing| {
            (editing.commands, editing.code, editing.key)
        });
        (commands, code, key, keys.len() - 1)
    }

    #[test]
    fn a_press_carries_the_commands_the_platform_or_the_edit_menu_gives_its_key() {
        let held = |meta, control, shift, alt| Modifiers {
            shift,
            control,
            alt,
            meta,
        };
        let press = |code, characters, modifiers, commands: &[&str]| KeyInput {
            commands: commands.iter().map(|&name| name.to_owned()).collect(),
            ..crate::mac_key_input(code, true, false, characters, modifiers)
        };
        let none = Modifiers::default();
        let own = |name: &str| vec![name.to_owned()];
        let rows = [
            // AppKit's bindings, as the shell resolved them.
            (
                press(
                    51,
                    "\u{7f}",
                    held(false, false, false, true),
                    &["deleteWordBackward"],
                ),
                (
                    own("deleteWordBackward"),
                    "Backspace",
                    "Backspace".to_owned(),
                    0,
                ),
            ),
            (
                press(
                    0,
                    "\u{1}",
                    held(false, true, false, false),
                    &["moveToBeginningOfParagraph"],
                ),
                (own("moveToBeginningOfParagraph"), "KeyA", "a".to_owned(), 0),
            ),
            // The Edit menu's key equivalents, which AppKit's bindings do
            // not hold.
            (
                press(6, "z", held(true, false, false, false), &[]),
                (own("undo"), "KeyZ", "z".to_owned(), 0),
            ),
            (
                press(6, "z", held(true, false, true, false), &[]),
                (own("redo"), "KeyZ", "z".to_owned(), 0),
            ),
            (
                press(0, "a", held(true, false, false, false), &[]),
                (own("selectAll"), "KeyA", "a".to_owned(), 0),
            ),
            (
                press(9, "v", held(true, false, true, true), &[]),
                (own("pasteAndMatchStyle"), "KeyV", "v".to_owned(), 0),
            ),
            // Control is not Command on a Mac, and a letter is only typed.
            (
                press(6, "\u{1a}", held(false, true, false, false), &[]),
                (Vec::new(), "", String::new(), 0),
            ),
            (press(0, "a", none, &[]), (Vec::new(), "", String::new(), 1)),
        ];
        for (input, want) in rows {
            assert_eq!(sent(&input), want, "{:?} {:?}", input.key, input.modifiers);
        }
    }

    #[test]
    fn release_goes_to_the_page_that_got_the_press_once() {
        let mut capture = ButtonCapture::default();
        capture.press(PointerButton::Right, EntityId::from("p2"));
        capture.press(PointerButton::Left, EntityId::from("p1"));
        assert_eq!(
            capture.release(PointerButton::Right),
            Some(EntityId::from("p2"))
        );
        assert_eq!(capture.release(PointerButton::Right), None);
        assert_eq!(
            capture.release(PointerButton::Left),
            Some(EntityId::from("p1"))
        );
    }
}
