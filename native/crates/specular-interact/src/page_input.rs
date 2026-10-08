//! Shaping input for a page: the key sequence Chromium expects, and which
//! page is owed each held button's release.

use specular_core::{InputEvent, KeyEvent, KeyEventKind, PointerButton};
use specular_doc::EntityId;

use crate::{Effect, KeyInput};

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

/// Appends the key sequence for `input` to `page`: a raw key-down and one
/// character event per produced character on press, a key-up on release.
/// Command-key chords produce no character events, as on macOS where they
/// are shortcuts rather than text.
///
/// The key-down and the key-up carry the key's own character. CEF on macOS
/// builds an `NSEvent` from each, and one with no character is a change of
/// modifiers, which on a key that is not a modifier it reads as a press.
pub(crate) fn forward_key(page: &EntityId, input: &KeyInput, effects: &mut Vec<Effect>) {
    let mut push = |kind, windows_key_code, character| {
        effects.push(Effect::ForwardInput {
            page: page.clone(),
            event: InputEvent::Key(KeyEvent {
                kind,
                windows_key_code,
                native_key_code: input.native_key_code,
                character,
                modifiers: input.modifiers,
            }),
        });
    };
    if !input.pressed {
        push(KeyEventKind::Up, input.windows_key_code, input.character);
        return;
    }
    push(
        KeyEventKind::RawDown,
        input.windows_key_code,
        input.character,
    );
    if input.modifiers.meta {
        return;
    }
    for character in input.text.as_deref().unwrap_or_default().chars() {
        push(KeyEventKind::Char, character as i32, Some(character));
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
