//! Turning what the thread store says changed into effects.

use specular_agent::{Changed, RunRequest, Started};

use crate::{App, Effect};

/// Pushes the writes `changed` names: each thread's file, then the index.
pub(crate) fn emit(changed: &Changed, effects: &mut Vec<Effect>) {
    for thread in &changed.threads {
        effects.push(Effect::WriteThread(thread.clone()));
    }
    if changed.index {
        effects.push(Effect::WriteThreadIndex);
    }
}

/// Both sets of writes as one, each file once.
pub(super) fn merge(mut first: Changed, second: Changed) -> Changed {
    for thread in second.threads {
        if !first.threads.contains(&thread) {
            first.threads.push(thread);
        }
    }
    first.index |= second.index;
    first
}

/// The writes of `changed` and of a run just begun, then the run itself.
pub(super) fn run_started(
    app: &App,
    started: Option<Started>,
    changed: Changed,
    effects: &mut Vec<Effect>,
) {
    match started {
        Some(started) => {
            emit(&merge(changed, started.changed), effects);
            run(app, started.request, effects);
        }
        None => emit(&changed, effects),
    }
}

/// Starts `request` on the chosen model.
pub(super) fn run(app: &App, mut request: RunRequest, effects: &mut Vec<Effect>) {
    request.model = Some(app.models.chosen().to_owned());
    effects.push(Effect::RunAgent(Box::new(request)));
}
