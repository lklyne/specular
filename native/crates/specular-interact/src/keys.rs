//! Key routing: a binding first, then the focused page.
//!
//! Escape always cancels and is never forwarded. The other bindings belong
//! to the canvas, so while a page has keyboard focus their keys go to the
//! page instead.

use crate::update::run_action;
use crate::{Action, App, Effect, Focus, Key, KeyInput, Tool, page_input};

/// What a key transition is for.
#[derive(Debug, Clone, PartialEq)]
enum Route {
    /// A binding fired.
    Run(Action),
    /// The key belongs to a binding but does nothing this time.
    Swallow,
    /// Not a binding: the focused page gets it.
    Pass,
}

pub(crate) fn on_key(app: &mut App, input: &KeyInput, effects: &mut Vec<Effect>) {
    match route(app, input) {
        Route::Run(action) => run_action(app, action, effects),
        Route::Swallow => {}
        Route::Pass => match &app.session.focus {
            Focus::Page(page) => page_input::forward_key(page, input, effects),
            Focus::Canvas => {}
        },
    }
}

fn route(app: &App, input: &KeyInput) -> Route {
    let session = &app.session;
    let modifiers = input.modifiers;
    if input.key == Key::Escape {
        return if input.pressed {
            Route::Run(Action::Cancel)
        } else {
            Route::Swallow
        };
    }
    if !input.pressed || session.focus != Focus::Canvas {
        return Route::Pass;
    }
    let dragging = session.gesture.is_some();
    let plain = !(modifiers.shift || modifiers.control || modifiers.alt || modifiers.meta);
    let command = modifiers.meta && !modifiers.control && !modifiers.alt;
    match input.key {
        Key::Char('c') if plain && !input.repeat && !dragging => {
            Route::Run(Action::SetTool(match session.tool {
                Tool::Comment => Tool::Select,
                Tool::Select
                | Tool::AddPage
                | Tool::AddText
                | Tool::AddSticky
                | Tool::AddDocument
                | Tool::AddShape
                | Tool::Draw => Tool::Comment,
            }))
        }
        Key::Char('z') if command && dragging => Route::Swallow,
        Key::Char('z') if command && modifiers.shift => Route::Run(Action::Redo),
        Key::Char('z') if command => Route::Run(Action::Undo),
        _ => Route::Pass,
    }
}
