//! The agent effects: the threads on disk and the `claude` runs.

use std::path::PathBuf;

use specular_agent::{Index, Thread};
use specular_interact::{Event, ModelsNews, ProjectId, RunRequest, Space, ThreadId, iso8601};

use super::runtime::{Runtime, ShellWindow, unix_ms};
use crate::agent::{self, AgentBackend, AgentRuns, Disabled};
use crate::prefs;

/// The runs of a session. A test and a benchmark get none, so neither can
/// start `claude`.
pub(super) fn start_runs(real: bool) -> AgentRuns {
    let backend: Box<dyn AgentBackend> = if real && !cfg!(test) {
        agent::backend_from_env()
    } else {
        Box::new(Disabled)
    };
    AgentRuns::new(backend)
}

impl<W: ShellWindow> Runtime<W> {
    /// Reads `.specular/threads/` of every project's canvas folder and
    /// answers with `Event::ThreadsLoaded`, the threads of all of them.
    pub(super) fn load_threads(&mut self) {
        let space = self.app.space();
        if space.projects().is_empty() {
            let Some(folder) = self.space.as_deref() else {
                return;
            };
            let (threads, index) = agent::load(folder, &iso8601(unix_ms()));
            self.dispatch(Event::ThreadsLoaded { threads, index });
            return;
        }
        let ids: Vec<ProjectId> = space.projects().iter().map(|p| p.id.clone()).collect();
        let (mut threads, mut index) = (Vec::new(), Index::default());
        for id in ids {
            let (loaded, found) = self.project_threads(&id);
            if self.app.space().active().project == id {
                index.active = found.active;
            }
            index.by_canvas.extend(found.by_canvas);
            threads.extend(loaded);
        }
        self.dispatch(Event::ThreadsLoaded { threads, index });
    }

    /// Reads one project's threads, just opened, and answers with
    /// `Event::ThreadsAdded`.
    pub(super) fn load_project_threads(&mut self, id: &ProjectId) {
        let (threads, index) = self.project_threads(id);
        self.dispatch(Event::ThreadsAdded { threads, index });
    }

    /// The threads of the project `id` and its index. A thread names its
    /// canvas by the id the project's index keeps, which is turned into
    /// the canvas's id here.
    fn project_threads(&self, id: &ProjectId) -> (Vec<Thread>, Index) {
        let space = self.app.space();
        let Some(project) = space.project(id) else {
            return (Vec::new(), Index::default());
        };
        let (mut threads, mut index) =
            agent::load(&PathBuf::from(&project.canvases), &iso8601(unix_ms()));
        for thread in &mut threads {
            thread.tab_id = canvas_id(space, id, &thread.tab_id);
        }
        index.by_canvas = (index.by_canvas.into_iter())
            .map(|(tab, thread)| (canvas_id(space, id, &tab), thread))
            .collect();
        (threads, index)
    }

    /// Writes one thread's file, in its canvas's project's canvas folder,
    /// under the id that project's index keeps for the canvas.
    pub(super) fn write_thread(&self, thread: &ThreadId) {
        let Some(thread) = self.app.threads().get(thread) else {
            return;
        };
        let space = self.app.space();
        let canvas = space.canvas(&specular_interact::CanvasId::new(thread.tab_id.clone()));
        let folder = canvas
            .and_then(|canvas| space.project(&canvas.project))
            .map(|project| PathBuf::from(&project.canvases))
            .or_else(|| self.space.clone());
        let Some(folder) = folder else {
            return;
        };
        let written = match canvas.filter(|canvas| canvas.tab != canvas.id.as_str()) {
            Some(canvas) => {
                let mut renamed = thread.clone();
                renamed.tab_id.clone_from(&canvas.tab);
                agent::write_thread(&folder, &renamed)
            }
            None => agent::write_thread(&folder, thread),
        };
        if let Err(error) = written {
            tracing::warn!(thread = thread.id.as_str(), "thread not written: {error}");
        }
    }

    /// Writes `.specular/threads/index.json`.
    pub(super) fn write_thread_index(&self) {
        let Some(space) = self.space.as_deref() else {
            return;
        };
        if let Err(error) = agent::write_index(space, &self.app.thread_index_json()) {
            tracing::warn!("thread index not written: {error}");
        }
    }

    /// Starts the `claude` CLI for a thread in the request's folder, else the
    /// space folder; its output comes back as `Event::Agent`.
    pub(super) fn run_agent(&mut self, request: &RunRequest) {
        self.agents.start(request, self.space.as_deref());
    }

    /// Stops a thread's run, which answers with `Notice::Cancelled`.
    pub(super) fn cancel_agent(&mut self, thread: &ThreadId) {
        self.agents.cancel(thread);
    }

    /// Hands the app the model saved by an earlier run, and asks the agent
    /// which models there are; the list comes back as `Event::Models`.
    pub(super) fn load_models(&mut self) {
        let saved = self.prefs.as_deref().and_then(prefs::load_agent_model);
        if let Some(model) = saved {
            self.dispatch(Event::Models(ModelsNews::Saved(model)));
        }
        self.agents.ask_models();
    }

    pub(super) fn save_agent_model(&self, model: &str) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_agent_model(path, model) {
            tracing::warn!(path = %path.display(), "agent model not saved: {error}");
        }
    }

    /// Hands on what the runs reported since the last turn.
    pub(super) fn take_agent_notices(&mut self) {
        if let Some(models) = self.agents.take_models() {
            self.demand.changed();
            self.dispatch(Event::Models(ModelsNews::Listed(models)));
        }
        for (thread, notice) in self.agents.poll() {
            self.demand.changed();
            self.dispatch(Event::Agent { thread, notice });
        }
    }

    /// Whether a run is in flight, so the loop must keep turning to hear it.
    pub(super) fn agent_running(&self) -> bool {
        !self.agents.is_idle()
    }
}

/// The id of the canvas the project `project`'s index calls `tab`: the
/// same, unless another project's canvas had that id first.
fn canvas_id(space: &Space, project: &ProjectId, tab: &str) -> String {
    (space.canvases_of(project))
        .find(|canvas| canvas.tab == tab)
        .map_or_else(|| tab.to_owned(), |canvas| canvas.id.as_str().to_owned())
}
