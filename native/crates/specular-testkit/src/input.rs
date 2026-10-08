//! Scripted input: pointer, keys, text, wheel and pinch.
//!
//! Every method here is [`Driver`]'s of the same name, which has its
//! documentation, and returns the [`TestApp`] so a chain can end in one of
//! the testkit's own.

use std::sync::Arc;

use glam::Vec2;
use specular_core::{Modifiers, PointerButton};
use specular_interact::{Action, Driver, Event, Key, TextMeasure, Tool};

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
    fn pointer_move(at: impl Into<Vec2>);
    fn press(at: impl Into<Vec2>);
    fn drag_to(at: impl Into<Vec2>);
    fn release();
    fn drag(from: impl Into<Vec2>, to: impl Into<Vec2>);
    fn click(at: impl Into<Vec2>);
    fn double_click(at: impl Into<Vec2>);
    fn triple_click(at: impl Into<Vec2>);
    fn pointer_leave();
    fn key_down(key: Key);
    fn key_up(key: Key);
    fn key(key: Key);
    fn chord(modifiers: Modifiers, key: Key);
    fn type_text(text: &str);
    fn compose(text: &str);
    fn commit(text: &str);
    fn paste(text: &str);
    fn wheel(delta: impl Into<Vec2>);
    fn pinch(delta: f32);
    fn note_text(file: &str, text: &str);
    fn tick(unix_ms: u64);
    fn viewport(size: impl Into<Vec2>);
    fn zoom(zoom: f32);
    fn tool(tool: Tool);
    fn select(ids: &[&str]);
    fn right_click(at: impl Into<Vec2>);
    fn press_button(button: PointerButton, at: impl Into<Vec2>);
    fn show_sidebar(shown: bool);
    fn send(event: Event);
    fn act(action: Action);
    fn measure_with(measure: Arc<dyn TextMeasure>);
}
