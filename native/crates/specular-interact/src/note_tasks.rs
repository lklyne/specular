//! Ticking a task in a Document that is being read, not edited.
//!
//! The renderer reports where each task's tick box landed (the
//! [`TickBox`]es in [`NoteLayout`](crate::NoteLayout)), numbered in the
//! order the source has its tasks. A press on one flips that task's `[ ]`
//! or `[x]` in the file, as one undo step, and does nothing else: the
//! Document is not selected and nothing is dragged.

use pulldown_cmark::{Event, Options, Parser};
use specular_doc::EntityId;

use crate::edit::{NOTE_PADDING, note};
use crate::scroll_follow::placed_rect;
use crate::{App, Effect, Hit, PointerInput, hit_test};

/// Takes a press on a tick box of a Document shown as read, and says
/// whether it did.
pub(crate) fn press(app: &mut App, input: &PointerInput, effects: &mut Vec<Effect>) -> bool {
    let Some((file, task)) = task_under(app, input) else {
        return false;
    };
    let Some(source) = note::text_of(app.session.notes.get(&file)) else {
        return false;
    };
    let Some(ticked) = toggled(source, task) else {
        return false;
    };
    note::replace(app, &file, ticked, effects);
    true
}

/// The file and the task whose tick box is under the press.
fn task_under(app: &App, input: &PointerInput) -> Option<(String, u32)> {
    let Hit::EntityBody { entity } = hit_test(app, input.screen) else {
        return None;
    };
    if app.text_edit().is_some_and(|edit| edit.entity() == &entity) {
        return None;
    }
    let found = app.document.entity(&entity)?;
    let file = note::file_of(found)?.to_owned();
    let world = app.session.camera.screen_to_world(input.screen);
    let rect = placed_rect(app, found);
    let padding = f64::from(NOTE_PADDING);
    let x = (f64::from(world.x) - rect.x - padding) as f32;
    let y = (f64::from(world.y) - rect.y - padding) as f32 + app.session.notes.scroll(&entity);
    let task = tick_at(app, &entity, x, y)?;
    Some((file, task))
}

/// The task whose tick box holds the point `x`, `y` of `entity`'s text.
fn tick_at(app: &App, entity: &EntityId, x: f32, y: f32) -> Option<u32> {
    (app.session.notes.ticks(entity).iter())
        .find(|tick| {
            (tick.x..tick.x + tick.width).contains(&x)
                && (tick.y..tick.y + tick.height).contains(&y)
        })
        .map(|tick| tick.task)
}

/// `source` with its `task`th task (counting from 0, top down) ticked if it
/// was not and unticked if it was. `None` when there is no such task.
pub(crate) fn toggled(source: &str, task: u32) -> Option<String> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    let (done, range) = (Parser::new_ext(source, options).into_offset_iter())
        .filter_map(|(event, range)| match event {
            Event::TaskListMarker(done) => Some((done, range)),
            _ => None,
        })
        .nth(usize::try_from(task).ok()?)?;
    // The marker's range is its `[ ]` or `[x]`; the mark is the byte after
    // the opening bracket.
    let open = range.start + source.get(range.clone())?.find('[')?;
    let mark = open + 1;
    let mut out = String::with_capacity(source.len());
    out.push_str(source.get(..mark)?);
    out.push(if done { ' ' } else { 'x' });
    out.push_str(source.get(mark + 1..)?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::toggled;

    #[test]
    fn ticks_and_unticks_the_numbered_task() {
        let source = "# List\n\n- [ ] one\n- [x] two\n  - [ ] nested\n\n> - [X] quoted\n";
        assert_eq!(
            toggled(source, 0).as_deref(),
            Some("# List\n\n- [x] one\n- [x] two\n  - [ ] nested\n\n> - [X] quoted\n")
        );
        assert_eq!(
            toggled(source, 1).as_deref(),
            Some("# List\n\n- [ ] one\n- [ ] two\n  - [ ] nested\n\n> - [X] quoted\n")
        );
        assert_eq!(
            toggled(source, 2).as_deref(),
            Some("# List\n\n- [ ] one\n- [x] two\n  - [x] nested\n\n> - [X] quoted\n")
        );
        assert_eq!(
            toggled(source, 3).as_deref(),
            Some("# List\n\n- [ ] one\n- [x] two\n  - [ ] nested\n\n> - [ ] quoted\n")
        );
        assert_eq!(toggled(source, 4), None);
    }

    #[test]
    fn brackets_in_code_are_not_tasks() {
        let source = "```\n- [ ] not a task\n```\n\n- [ ] task\n";
        assert_eq!(
            toggled(source, 0).as_deref(),
            Some("```\n- [ ] not a task\n```\n\n- [x] task\n")
        );
    }
}
