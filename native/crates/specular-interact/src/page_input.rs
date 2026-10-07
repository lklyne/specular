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

    /// The page owed `button`'s release, if a page received its press.
    pub(crate) fn release(&mut self, button: PointerButton) -> Option<EntityId> {
        self.pages[Self::slot(button)].take()
    }
}

/// Appends the key sequence for `input` to `page`: a raw key-down and one
/// character event per produced character on press, a key-up on release.
/// Command-key chords produce no character events, as on macOS where they
/// are shortcuts rather than text.
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
        push(KeyEventKind::Up, input.windows_key_code, None);
        return;
    }
    push(KeyEventKind::RawDown, input.windows_key_code, None);
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
    fn letter_press_sends_raw_down_then_char() {
        let input = key(true, Some("a"), Modifiers::default());
        assert_eq!(kinds(&input), [KeyEventKind::RawDown, KeyEventKind::Char]);
    }

    #[test]
    fn command_chord_sends_no_char() {
        let meta = Modifiers {
            meta: true,
            ..Modifiers::default()
        };
        assert_eq!(kinds(&key(true, Some("a"), meta)), [KeyEventKind::RawDown]);
    }

    #[test]
    fn release_sends_single_up() {
        assert_eq!(
            kinds(&key(false, None, Modifiers::default())),
            [KeyEventKind::Up]
        );
    }

    #[test]
    fn release_goes_to_the_page_that_got_the_press() {
        let mut capture = ButtonCapture::default();
        capture.press(PointerButton::Right, EntityId::from("p2"));
        capture.press(PointerButton::Left, EntityId::from("p1"));
        assert_eq!(
            capture.release(PointerButton::Right),
            Some(EntityId::from("p2"))
        );
    }

    #[test]
    fn release_without_press_goes_nowhere() {
        let mut capture = ButtonCapture::default();
        capture.press(PointerButton::Left, EntityId::from("p1"));
        capture.release(PointerButton::Left);
        assert_eq!(capture.release(PointerButton::Left), None);
    }
}
