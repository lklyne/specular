//! A key with its modifiers, as a binding names it.

use crate::{Key, KeyInput};

/// A key with the modifiers that must be held, and no others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    /// The physical key.
    pub key: Key,
    /// Command or Control.
    pub cmd: bool,
    /// Shift.
    pub shift: bool,
    /// Option.
    pub alt: bool,
}

impl Chord {
    /// The chord as macOS writes it: `⇧⌘Z`.
    pub fn text(self) -> String {
        let mut text = String::new();
        for (held, sign) in [(self.alt, '⌥'), (self.shift, '⇧'), (self.cmd, '⌘')] {
            if held {
                text.push(sign);
            }
        }
        match self.key {
            Key::Char(character) => text.extend(character.to_uppercase()),
            Key::Escape => text.push('⎋'),
            Key::Enter => text.push('↩'),
            Key::Tab => text.push('⇥'),
            Key::Backspace => text.push('⌫'),
            Key::Delete => text.push('⌦'),
            Key::Space => text.push_str("Space"),
            Key::ArrowLeft => text.push('←'),
            Key::ArrowRight => text.push('→'),
            Key::ArrowUp => text.push('↑'),
            Key::ArrowDown => text.push('↓'),
            Key::Home => text.push('↖'),
            Key::End => text.push('↘'),
            Key::PageUp => text.push('⇞'),
            Key::PageDown => text.push('⇟'),
            Key::Other => {}
        }
        text
    }

    /// `key` with no modifiers.
    pub const fn key(key: Key) -> Self {
        Self {
            key,
            cmd: false,
            shift: false,
            alt: false,
        }
    }

    /// The letter, digit or punctuation key `character` with no modifiers.
    pub const fn char(character: char) -> Self {
        Self::key(Key::Char(character))
    }

    /// The same key with Command or Control held as well.
    #[must_use]
    pub const fn cmd(self) -> Self {
        Self { cmd: true, ..self }
    }

    /// The same key with Shift held as well.
    #[must_use]
    pub const fn shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }

    /// The same key with Option held as well.
    #[must_use]
    pub const fn alt(self) -> Self {
        Self { alt: true, ..self }
    }

    /// The chord `input` is, whether it is a press or a release.
    ///
    /// Escape is Escape whatever is held with it, so it cancels a drag that
    /// has Shift or Option down.
    pub fn of(input: &KeyInput) -> Self {
        if input.key == Key::Escape {
            return Self::key(Key::Escape);
        }
        let modifiers = input.modifiers;
        Self {
            key: input.key,
            cmd: modifiers.meta || modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.alt,
        }
    }
}
