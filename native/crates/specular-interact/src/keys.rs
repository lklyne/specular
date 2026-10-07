//! Key routing: a binding first, then the focused page.
//!
//! Escape always cancels and is never forwarded. The other bindings belong
//! to the canvas, so while a page has keyboard focus their keys go to the
//! page instead.

use specular_core::PointerEventKind;

use crate::update::run_action;
use crate::{
    Action, App, Effect, Focus, Key, KeyInput, PointerInput, Tool, gesture, grid, page_input,
};

/// How far an arrow key moves the selection, in canvas units. Shift moves it
/// a grid step instead.
const NUDGE_STEP: f64 = 5.0;

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
    // A drag reads Shift, Option and Command, so one that changes while the
    // pointer is still takes effect at once.
    if let (Some(_), Some(screen)) = (&app.session.gesture, app.session.pointer) {
        let held = PointerInput {
            kind: PointerEventKind::Move,
            screen,
            modifiers: input.modifiers,
        };
        gesture::drag(app, &held);
    }
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
        Key::Backspace
        | Key::Delete
        | Key::ArrowLeft
        | Key::ArrowRight
        | Key::ArrowUp
        | Key::ArrowDown
            if dragging =>
        {
            Route::Swallow
        }
        Key::Backspace | Key::Delete if plain && !input.repeat => Route::Run(Action::Delete),
        Key::Char('d') if command && !modifiers.shift => {
            if dragging || input.repeat {
                Route::Swallow
            } else {
                Route::Run(Action::Duplicate)
            }
        }
        Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown
            if plain || shift_only(input) =>
        {
            let step = if modifiers.shift {
                grid::GRID_SIZE
            } else {
                NUDGE_STEP
            };
            let (dx, dy) = match input.key {
                Key::ArrowLeft => (-step, 0.0),
                Key::ArrowRight => (step, 0.0),
                Key::ArrowUp => (0.0, -step),
                _ => (0.0, step),
            };
            Route::Run(Action::Nudge { dx, dy })
        }
        Key::Char('z') if command && dragging => Route::Swallow,
        Key::Char('z') if command && modifiers.shift => Route::Run(Action::Redo),
        Key::Char('z') if command => Route::Run(Action::Undo),
        _ => Route::Pass,
    }
}

const fn shift_only(input: &KeyInput) -> bool {
    let modifiers = input.modifiers;
    modifiers.shift && !(modifiers.control || modifiers.alt || modifiers.meta)
}
