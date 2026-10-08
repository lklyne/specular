//! Changes asked for from outside the window: the HTTP API's way into
//! [`update`](crate::update).
//!
//! An [`ApiCall`] runs what the user could have done by hand, so an agent's
//! change is one undo step like any other, and it is answered with an
//! [`Effect::ApiReply`](crate::Effect::ApiReply). The ticket stands in for a
//! reply channel: [`Event`](crate::Event) stays plain data, and the shell
//! keeps the channel beside the ticket.

use specular_core::PageNav;
use specular_doc::{Command, Document, EntityId, ItemId, Kind, Rect, TextStyle};

use crate::{Action, App, CanvasId, Effect, edit, live, page_state, space, update};

/// One change the HTTP API asks for.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiCall {
    /// Names the reply.
    pub ticket: u64,
    /// The canvas to run on, which `--tab` named. `None` is the active
    /// one. A background canvas is changed without being shown.
    pub canvas: Option<CanvasId>,
    /// What to do.
    pub run: ApiRun,
}

/// What an [`ApiCall`] does.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiRun {
    /// Run an action. With `on`, those items are selected first, as the
    /// user would select them before pressing the action's key.
    Act {
        /// The items to select first.
        on: Option<Vec<ItemId>>,
        /// The action.
        action: Action,
    },
    /// Move a hosted page through its history, or reload it. Refused where
    /// the page has said there is nowhere to go, and for an id that is not
    /// a page.
    Navigate {
        /// The page entity.
        page: EntityId,
        /// Where to.
        nav: PageNav,
    },
    /// Run a command as one undo step, with the refit of the groups it
    /// touches and the page hosts brought in step.
    Apply {
        /// The command.
        command: Command,
        /// The items to select once it has run.
        select: Option<Vec<ItemId>>,
    },
    /// [`Apply`](Self::Apply) after the shell has put files in the space
    /// folder: the [`Effect::WriteNote`] and [`Effect::CopyAsset`] of the
    /// entities `command` adds, which run before the effects that load
    /// them. Nothing runs when the command is refused.
    ApplyWithFiles {
        /// The file effects, in order.
        files: Vec<Effect>,
        /// The command.
        command: Command,
        /// The items to select once it has run.
        select: Option<Vec<ItemId>>,
    },
    /// Add an empty canvas with this name and leave the user where they
    /// are. Refused when the name is empty or taken.
    NewCanvas {
        /// The name.
        name: String,
    },
}

/// How an [`ApiCall`] went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiOutcome {
    /// It ran.
    Done,
    /// It did not run, and nothing changed. The text says why.
    Refused(String),
}

pub(crate) fn run(app: &mut App, call: ApiCall, effects: &mut Vec<Effect>) {
    let ApiCall {
        ticket,
        canvas,
        run,
    } = call;
    let background = canvas.filter(|id| *id != app.space.active().id);
    let Some(id) = background else {
        let outcome = run_call(app, run, effects);
        effects.push(Effect::ApiReply { ticket, outcome });
        return;
    };
    let ran = space::in_background(app, &id, effects, |app, effects| {
        let outcome = run_call(app, run, effects);
        effects.push(Effect::ApiReply { ticket, outcome });
    });
    if !ran {
        let outcome = ApiOutcome::Refused(format!("the tab '{id}' is gone"));
        effects.push(Effect::ApiReply { ticket, outcome });
    }
}

fn run_call(app: &mut App, run: ApiRun, effects: &mut Vec<Effect>) -> ApiOutcome {
    // The user's drag owns the document until the button comes up.
    if app.session.gesture.is_some() {
        return ApiOutcome::Refused("a drag is in flight; try again when it ends".to_owned());
    }
    match run {
        ApiRun::Act { on, action } => {
            if let Some(items) = on {
                edit::end(app, effects);
                app.session.selection.set(items);
                update::drop_dangling(app, effects);
            }
            update::run_action(app, action, effects);
        }
        ApiRun::Navigate { page, nav } => {
            if let Err(reason) = page_state::allows(app, &page, &nav) {
                return ApiOutcome::Refused(reason);
            }
            effects.push(Effect::Navigate { page, nav });
        }
        ApiRun::Apply { command, select } => {
            return apply(app, command, select, Vec::new(), effects);
        }
        ApiRun::ApplyWithFiles {
            files,
            command,
            select,
        } => return apply(app, command, select, files, effects),
        ApiRun::NewCanvas { name } => {
            if let Err(reason) = space::create(app, Some(&name), false, effects) {
                return ApiOutcome::Refused(reason);
            }
        }
    }
    ApiOutcome::Done
}

/// Runs `command` as one undo step, with the file effects it needs first.
fn apply(
    app: &mut App,
    command: Command,
    select: Option<Vec<ItemId>>,
    files: Vec<Effect>,
    effects: &mut Vec<Effect>,
) -> ApiOutcome {
    edit::end(app, effects);
    // A refused command has to be reported, and the step itself only
    // logs it, so it is tried on a copy first.
    let mut trial = app.document.clone();
    if let Err(error) = trial.apply(command.clone()) {
        return ApiOutcome::Refused(error.to_string());
    }
    effects.extend(files);
    let command = with_text_fits(app, &trial, command);
    update::document_step(app, command, effects);
    if let Some(items) = select {
        app.session.selection.set(items);
        update::drop_dangling(app, effects);
    }
    ApiOutcome::Done
}

/// `command` followed by the resize of every text it adds or changes to
/// the size its text takes, as ending an edit of it in the window would
/// leave it. `after` is the document as `command` leaves it. A sticky only
/// grows, so a size the caller asked for is kept when the text fits in it.
fn with_text_fits(app: &App, after: &Document, command: Command) -> Command {
    let fits = after.entities().filter_map(|entity| {
        let text = match &entity.kind {
            Kind::Text(text) => text,
            Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {
                return None;
            }
        };
        let before = app.document.entity(&entity.id);
        if before.is_some_and(|before| before.kind == entity.kind && before.rect == entity.rect) {
            return None;
        }
        let fitted = edit::fitted(app, entity.rect, text);
        let rect = match text.resolved_style() {
            TextStyle::Sticky => Rect {
                height: fitted.height.max(entity.rect.height),
                ..fitted
            },
            TextStyle::Plain => fitted,
        };
        (rect != entity.rect).then(|| Command::SetRect {
            id: entity.id.clone(),
            rect,
        })
    });
    let mut commands = vec![command];
    commands.extend(fits);
    live::batch(commands)
}
