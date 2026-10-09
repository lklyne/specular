//! The editing commands `AppKit`'s key bindings give a key.
//!
//! A text view does not learn "word left" from the key. `AppKit` turns the
//! key into commands (`moveWordLeft:`, `deleteToBeginningOfLine:`) from the
//! system's bindings and the user's own `DefaultKeyBinding.dict`, and a
//! browser sends them into the page with the key. A page here is drawn
//! offscreen and is no text view, so the same resolution is asked for by
//! hand, of the object `NSTextInputContext` itself asks:
//! `NSKeyBindingManager`.
//!
//! That class is not in `AppKit`'s headers. It is looked up by name and
//! asked whether it answers the one message used, so a system without it
//! leaves a page with the bare key, which is what it had before.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::cell::{OnceCell, RefCell};

use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};

use crate::native::Id;

thread_local! {
    /// The selectors the manager sent the client during one resolution.
    static RESOLVED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The manager and the client it answers, or nothing on a system that
    /// has no such manager.
    static RESOLVER: OnceCell<Option<(Id, Id)>> = const { OnceCell::new() };
}

extern "C-unwind" fn do_command(_this: &AnyObject, _cmd: Sel, selector: Sel) {
    let name = selector.name().to_string_lossy().into_owned();
    RESOLVED.with(|resolved| resolved.borrow_mut().push(name));
}

/// Text is the key's own to type; only commands are asked for.
extern "C-unwind" fn insert_text(_this: &AnyObject, _cmd: Sel, _text: Id) {}

fn resolver() -> Option<(Id, Id)> {
    let class = AnyClass::get(c"NSKeyBindingManager")?;
    let mut builder = ClassBuilder::new(c"SpecularKeyBindingClient", class!(NSObject))?;
    // SAFETY: both functions match their selector's signature,
    // `void (id, SEL, SEL)` and `void (id, SEL, id)`.
    unsafe {
        builder.add_method(
            sel!(doCommandBySelector:),
            do_command as extern "C-unwind" fn(_, _, _),
        );
        builder.add_method(
            sel!(insertText:),
            insert_text as extern "C-unwind" fn(_, _, _),
        );
    }
    let client_class = builder.register();
    // SAFETY: `respondsToSelector:` is asked before each message that no
    // header declares, and the two objects live as long as the process.
    unsafe {
        let shared: bool = msg_send![class, respondsToSelector: sel!(sharedKeyBindingManager)];
        if !shared {
            return None;
        }
        let manager: Id = msg_send![class, sharedKeyBindingManager];
        if manager.is_null() {
            return None;
        }
        let interprets: bool = msg_send![
            manager,
            respondsToSelector: sel!(interpretEventAsCommand:forClient:)
        ];
        if !interprets {
            return None;
        }
        let client: Id = msg_send![client_class, new];
        (!client.is_null()).then_some((manager, client))
    }
}

/// The command a selector names for a page, or nothing for one that is not
/// an editing command there: `noop:`, which is what a Command chord with no
/// binding resolves to, and the `insert` family (`insertNewline:`,
/// `insertTab:`), which a page does from the key itself. Sent as commands,
/// Tab would type a tab and not move to the next field.
pub(crate) fn command_name(selector: &str) -> Option<String> {
    let name = selector.strip_suffix(':').unwrap_or(selector);
    let inserts = (name.get(..6)).is_some_and(|start| start.eq_ignore_ascii_case("insert"));
    (!name.is_empty() && name != "noop" && !inserts).then(|| name.to_owned())
}

/// The editing commands the key-down `event` is bound to, in order.
pub(crate) fn commands(event: Id) -> Vec<String> {
    let resolver = RESOLVER.with(|cell| {
        *cell.get_or_init(|| {
            let found = resolver();
            if found.is_none() {
                tracing::warn!(
                    "AppKit has no key-binding manager to ask; a page gets keys without \
                     their editing commands"
                );
            }
            found
        })
    });
    let Some((manager, client)) = resolver else {
        return Vec::new();
    };
    RESOLVED.with(|resolved| resolved.borrow_mut().clear());
    // SAFETY: `event` is the live key-down `NSEvent` the monitor was called
    // with, and the manager was asked that it answers this message. It
    // calls `doCommandBySelector:` on the client before it returns.
    let _: bool = unsafe { msg_send![manager, interpretEventAsCommand: event, forClient: client] };
    let selectors = RESOLVED.with(|resolved| std::mem::take(&mut *resolved.borrow_mut()));
    (selectors.iter())
        .filter_map(|selector| command_name(selector))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selector_is_a_command_unless_it_inserts_or_does_nothing() {
        let rows = [
            ("moveWordLeft:", Some("moveWordLeft")),
            ("deleteToBeginningOfLine:", Some("deleteToBeginningOfLine")),
            (
                "moveToBeginningOfParagraph:",
                Some("moveToBeginningOfParagraph"),
            ),
            ("cancelOperation:", Some("cancelOperation")),
            // What Command+A and Command+Z resolve to: the menu has those.
            ("noop:", None),
            ("insertNewline:", None),
            ("insertTab:", None),
            ("insertBacktab:", None),
        ];
        for (selector, want) in rows {
            assert_eq!(command_name(selector).as_deref(), want, "{selector}");
        }
    }
}
