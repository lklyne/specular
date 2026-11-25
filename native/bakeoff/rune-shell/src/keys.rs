//! Keys and the input method, from Rune's key events to the app's.
//!
//! Rune hands a key over as a logical [`KeyEvent`] (what it types) with the
//! physical key behind it on the handler's `Tx`. The app wants both in one
//! [`KeyInput`], which `specular-interact` builds from a macOS key code and
//! the characters the key carried, as the Kit's `keys.rs` does from the raw
//! `NSEvent`. So the physical key is named by its DOM code, looked up in
//! [`PhysicalKey`]'s table, and handed to [`mac_key_input`] with its code.
//!
//! Presses come through the window's `on_key`, which hears every key nothing
//! focused took, releases through the root's `on_key_up`, and modifier keys
//! on their own through `on_key` too (`App::modifier_keys`).

use rune::{Ime, Key, KeyEvent, Modifier, Mods, PhysicalKeyCode, Tx};
use specular_core::{ImeEvent, Modifiers};
use specular_interact::{Event, KeyInput, PhysicalKey, mac_key_input};

use crate::shell::Shell;

/// Rune's modifiers as the app's. On macOS `cmd` is Command; elsewhere it is
/// Control, and the app's `meta` is the logo key.
pub fn modifiers(mods: Mods) -> Modifiers {
    Modifiers {
        shift: mods.shift,
        control: mods.ctrl,
        alt: mods.alt,
        meta: if cfg!(target_os = "macos") {
            mods.cmd
        } else {
            mods.logo
        },
    }
}

/// The key behind `code`. Winit names its key codes as the DOM does, but for
/// the logo keys.
fn physical(code: PhysicalKeyCode) -> Option<PhysicalKey> {
    let name = format!("{code:?}");
    let dom = match name.as_str() {
        "SuperLeft" => "MetaLeft",
        "SuperRight" => "MetaRight",
        other => other,
    };
    PhysicalKey::from_dom_code(dom)
}

/// The key a logical key sits on in a US layout, for a key with no physical
/// origin (a script's `"key"` without a `"code"`).
fn us_key(key: &Key) -> Option<PhysicalKey> {
    use specular_interact::Key as K;
    let binding = match key {
        Key::Enter => K::Enter,
        Key::Backspace => K::Backspace,
        Key::Delete => K::Delete,
        Key::Left => K::ArrowLeft,
        Key::Right => K::ArrowRight,
        Key::Up => K::ArrowUp,
        Key::Down => K::ArrowDown,
        Key::Home => K::Home,
        Key::End => K::End,
        Key::PageUp => K::PageUp,
        Key::PageDown => K::PageDown,
        Key::Escape => K::Escape,
        Key::Tab => K::Tab,
        Key::Command(' ') => K::Space,
        Key::Command(c) => K::Char(c.to_ascii_lowercase()),
        Key::Text(text) => match text.chars().collect::<Vec<_>>()[..] {
            [' '] => K::Space,
            [c] => K::Char(c.to_ascii_lowercase()),
            _ => return None,
        },
        Key::Modifier { modifier, .. } => return modifier_key(*modifier),
        _ => return None,
    };
    PhysicalKey::for_key(binding)
}

fn modifier_key(modifier: Modifier) -> Option<PhysicalKey> {
    Some(match modifier {
        Modifier::ShiftLeft => PhysicalKey::ShiftLeft,
        Modifier::ShiftRight => PhysicalKey::ShiftRight,
        Modifier::ControlLeft => PhysicalKey::ControlLeft,
        Modifier::ControlRight => PhysicalKey::ControlRight,
        Modifier::AltLeft => PhysicalKey::AltLeft,
        Modifier::AltRight => PhysicalKey::AltRight,
        Modifier::SuperLeft => PhysicalKey::SuperLeft,
        Modifier::SuperRight => PhysicalKey::SuperRight,
        _ => return None,
    })
}

/// What `event` typed, as `NSEvent.characters` carries it. Keys that type
/// nothing carry nothing; `mac_key_input` names Return, Tab and the rest.
fn characters(event: &KeyEvent) -> String {
    match &event.key {
        Key::Text(text) => text.clone(),
        Key::Command(c) if event.mods.shift => c.to_uppercase().collect(),
        Key::Command(c) => c.to_string(),
        _ => String::new(),
    }
}

/// `event` as the app's key transition, or `None` for a key the app has no
/// name for (a media key, Fn).
fn key_input(event: &KeyEvent, pressed: bool, tx: &Tx) -> Option<KeyInput> {
    let pressed = match event.key {
        Key::Modifier { pressed, .. } => pressed,
        _ => pressed,
    };
    let key = match event.key {
        // A modifier key's own code is the side Rune names, not the key
        // that changed the flags last.
        Key::Modifier { modifier, .. } => modifier_key(modifier),
        _ => tx.physical_key().and_then(physical),
    }
    .or_else(|| us_key(&event.key))?;
    Some(mac_key_input(
        key.mac_key_code(),
        pressed,
        pressed && tx.key_repeat(),
        &characters(event),
        modifiers(event.mods),
    ))
}

/// `AppSpec::on_key`: a key went down that nothing focused took.
pub fn on_key(shell: &mut Shell, event: &KeyEvent, tx: &mut Tx) -> bool {
    let Some(input) = key_input(event, true, tx) else {
        return false;
    };
    shell.dispatch(Event::Key(input));
    true
}

/// The root's `on_key_up`: a key came up.
pub fn on_key_up(shell: &mut Shell, event: &KeyEvent, tx: &mut Tx) -> bool {
    let Some(input) = key_input(event, false, tx) else {
        return false;
    };
    shell.dispatch(Event::Key(input));
    true
}

/// The UTF-16 length of `text[..byte]`: the Kit hands the app composition
/// ranges in the units `NSTextInputClient` speaks.
fn utf16_at(text: &str, byte: usize) -> u32 {
    let byte = (0..=byte.min(text.len()))
        .rev()
        .find(|at| text.is_char_boundary(*at))
        .unwrap_or(0);
    text[..byte].encode_utf16().count() as u32
}

/// `App::on_ime`: what the input method composed, while the app has the
/// input method on (an editor's caret carries `El::ime_cursor`).
pub fn on_ime(shell: &mut Shell, ime: &Ime, _: &mut Tx) -> bool {
    let event = match ime {
        Ime::Preedit(text, _, _) if text.is_empty() => ImeEvent::Cancel,
        Ime::Preedit(text, cursor, _) => {
            let end = text.len();
            let (start, stop) = cursor.unwrap_or((end, end));
            ImeEvent::SetComposition {
                text: text.clone(),
                selection: utf16_at(text, start)..utf16_at(text, stop),
                replacement: None,
            }
        }
        Ime::Commit(text) => ImeEvent::Commit {
            text: text.clone(),
            replacement: None,
        },
        _ => return false,
    };
    shell.dispatch(Event::Ime(event));
    true
}
