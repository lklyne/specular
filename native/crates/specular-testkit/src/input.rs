//! Scripted input: pointer, keys, text, wheel and pinch.
//!
//! Every method here is [`Driver`]'s of the same name, which has its
//! documentation, and returns the [`TestApp`] so a chain can end in one of
//! the testkit's own.

use glam::Vec2;
use specular_core::Modifiers;
use specular_interact::{Action, Driver, Event, Key};

use crate::TestApp;

/// Declares each [`Driver`] method on [`TestApp`], passing the call on.
macro_rules! driven {
    ($(fn $name:ident($($arg:ident: $kind:ty),*);)*) => {
        /// Pointer positions are logical screen pixels and take a tuple:
        /// `app.press((200.0, 150.0))`.
        impl TestApp {
            $(
                #[doc = concat!("[`Driver::", stringify!($name), "`], on the app under test.")]
                pub fn $name(&mut self $(, $arg: $kind)*) -> &mut Self {
                    Driver::$name(&mut self.driver $(, $arg)*);
                    self
                }
            )*
        }
    };
}

driven! {
    fn hold(modifiers: Modifiers);
    fn let_go();
    fn press(at: impl Into<Vec2>);
    fn drag_to(at: impl Into<Vec2>);
    fn release();
    fn drag(from: impl Into<Vec2>, to: impl Into<Vec2>);
    fn click(at: impl Into<Vec2>);
    fn double_click(at: impl Into<Vec2>);
    fn key(key: Key);
    fn chord(modifiers: Modifiers, key: Key);
    fn type_text(text: &str);
    fn tick(unix_ms: u64);
    fn viewport(size: impl Into<Vec2>);
    fn zoom(zoom: f32);
    fn select(ids: &[&str]);
    fn send(event: Event);
    fn act(action: Action);
}
