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

/// A chord is written as macOS writes it, `⇧⌘Z`.
impl serde::Serialize for Chord {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.text())
    }
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
    pub(crate) const fn key(key: Key) -> Self {
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
    pub(crate) const fn shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }

    /// The same key with Option held as well.
    #[must_use]
    pub(crate) const fn alt(self) -> Self {
        Self { alt: true, ..self }
    }

    /// The chord a mod writes, such as `cmd+shift+u` or `alt+1`: modifiers
    /// (`cmd`, `ctrl`, `shift`, `alt`, `option`) and one character, joined
    /// by `+`. `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let (modifiers, key) = text.rsplit_once('+').unwrap_or(("", text));
        let mut characters = key.chars();
        let character = characters.next()?.to_lowercase().next()?;
        if characters.next().is_some() {
            return None;
        }
        let mut chord = Self::char(character);
        for modifier in modifiers.split('+').filter(|word| !word.is_empty()) {
            chord = match modifier.to_ascii_lowercase().as_str() {
                "cmd" | "ctrl" | "meta" => chord.cmd(),
                "shift" => chord.shift(),
                "alt" | "option" => chord.alt(),
                _ => return None,
            };
        }
        Some(chord)
    }

    /// The chord `input` is, whether it is a press or a release.
    ///
    /// Escape is Escape whatever is held with it, so it cancels a drag that
    /// has Shift or Option down.
    pub(crate) fn of(input: &KeyInput) -> Self {
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

#[cfg(test)]
mod tests {
    use super::Chord;

    #[test]
    fn a_mod_writes_a_chord_as_words() {
        assert_eq!(
            Chord::parse("cmd+shift+U"),
            Some(Chord::char('u').cmd().shift())
        );
        assert_eq!(Chord::parse("k"), Some(Chord::char('k')));
        assert_eq!(Chord::parse("hyper+k"), None);
        assert_eq!(Chord::parse("cmd+up"), None);
    }
}
