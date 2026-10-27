//! One step of a script, done headless.

use std::path::PathBuf;

use anyhow::Context as _;
use glam::Vec2;
use specular_doc::Document;
use specular_interact::{Action, Event, PageNotice};

use super::Headless;
use crate::script::{self, CameraArg, Expect, Step, WindowStep};

/// More undo steps than any script makes: a history that never runs out is
/// a bug, not a long session.
const UNDO_LIMIT: usize = 10_000;

impl Headless {
    pub(super) fn step(&mut self, step: Step) -> anyhow::Result<()> {
        // `expect sent` reads the effects of the last step that was not a
        // check.
        match step {
            Step::Expect(Expect::OnCanvas(fact)) if fact.is_sent() => {
                let runtime = self.hands.sink();
                script::check_sent(runtime.app(), runtime.sent(), &fact)
            }
            Step::Expect(Expect::Panel(fact)) if fact.reads_effects() => {
                let runtime = self.hands.sink();
                fact.check(runtime.app(), Some(runtime.sent()))
            }
            Step::Expect(_) | Step::State(_) | Step::Mark(_) | Step::Save(_) => self.do_step(step),
            _ => {
                self.runtime().clear_sent();
                self.do_step(step)
            }
        }
    }

    fn do_step(&mut self, step: Step) -> anyhow::Result<()> {
        self.clear_of_kit_chrome(&step)?;
        match step {
            Step::Move(at) => self.drive(|app| app.pointer_move(at)),
            Step::Press(at) => self.drive(|app| app.press(at)),
            Step::DragTo(at) => self.drive(|app| app.drag_to(at)),
            Step::Release => self.drive(|app| app.release()),
            Step::ReleaseAt(at) => self.drive(|app| app.drag_to(at).release()),
            Step::Click(at) => self.drive(|app| app.pointer_move(at).click(at)),
            Step::DoubleClick(at) => self.drive(|app| app.pointer_move(at).double_click(at)),
            Step::TripleClick(at) => self.drive(|app| app.pointer_move(at).triple_click(at)),
            Step::RightClick(at) => self.drive(|app| app.context_menu(at)),
            Step::Drag(from, to) => self.drive(|app| {
                app.pointer_move(from)
                    .press(from)
                    .drag_to(from.midpoint(to))
                    .drag_to(to)
                    .release()
            }),
            Step::Hold(modifiers) => self.drive(|app| app.hold(modifiers)),
            Step::Key(modifiers, key) => self.drive(|app| app.chord(modifiers, key)),
            Step::Type(text) => self.drive(|app| app.type_text(&text)),
            Step::Compose(text) => self.drive(|app| app.compose(&text)),
            Step::Commit(text) => self.drive(|app| app.commit(&text)),
            Step::Clipboard(text) => {
                self.runtime().script_clipboard(Some(text));
                Ok(())
            }
            Step::Wheel(delta) => self.drive(|app| app.wheel(delta)),
            Step::ScrollAt(at, delta) => self.drive(|app| app.pointer_move(at).wheel(delta)),
            Step::Pinch(delta) => self.drive(|app| app.pinch(delta)),
            Step::Tool(tool) => self.drive(|app| app.tool(tool)),
            Step::Act(action) => self.drive(|app| app.act(action)),
            Step::Select(ids) => self.drive(|app| {
                let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
                app.select(&ids)
            }),
            Step::Camera(CameraArg::Fit) => {
                // The fit is of the document, which may still be growing as
                // text is measured; loads do not move it.
                self.drive(|app| app.act(Action::ZoomToFit))
            }
            Step::Camera(CameraArg::At(camera)) => {
                self.drive(|app| app.act(Action::SetCamera(camera)))
            }
            Step::Wait(ms) => {
                // The pages run before the clock moves: what they report
                // during the wait happened during it, not after it.
                if self.live {
                    self.runtime()
                        .run_pages_for(std::time::Duration::from_millis(ms));
                }
                self.now_ms += ms;
                let now = self.now_ms;
                self.drive(|app| app.tick(now))?;
                let live = self.live;
                self.runtime().settle(live);
                Ok(())
            }
            Step::Control(id) => {
                self.hands.control(&id)?;
                self.runtime().take_error().map_or(Ok(()), Err)
            }
            // Nothing is drawn to point at or hold down. The name must still
            // be a control's.
            Step::HoverControl(id) | Step::PressControl(id) => {
                specular_interact::control_named(self.hands.app(), &id)?;
                Ok(())
            }
            Step::Sidebar(shown) => self.drive(|app| app.show_sidebar(shown)),
            Step::Motion(on) => self.drive(|app| app.send(Event::Motion(on))),
            Step::ReduceMotion(on) => self.drive(|app| app.send(Event::ReduceMotion(on))),
            Step::PageOpensTab {
                page,
                url,
                foreground,
            } => self.drive(|app| {
                app.send(Event::Page {
                    page: page.as_str().into(),
                    notice: PageNotice::OpenTab { url, foreground },
                })
            }),
            Step::Panel(step) => {
                let event = step.event(self.hands.app())?;
                self.drive(|app| app.send(event))
            }
            Step::Snapshot(path) => self.snapshot(&path),
            Step::Save(path) => self.save(&path),
            Step::State(path) => {
                let path = self.out.join(path);
                let text = script::dump(self.hands.app());
                std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
            }
            Step::Expect(expect) => self.checks.check(self.hands.app(), &expect),
            Step::Mark(name) => self.checks.mark(self.hands.app(), &name),
            Step::Reopen => self.reopen(),
            Step::UndoToStart(except) => self.undo_to_start(&except),
            Step::Window(WindowStep::Drop(paths, at)) => self.drop_files(&paths, at),
            // The run skips these before they get here.
            Step::Window(_) => Ok(()),
            Step::Canvas(step) => step
                .event(self.hands.app())
                .and_then(|event| self.drive(|app| app.send(event))),
        }?;
        self.frame_for_documents()
    }

    /// Fails a step that would put the pointer down, or move it with no
    /// button held, where the Kit window draws its chrome: there the window
    /// would hand it to the chrome, not the canvas. A drag already under
    /// way keeps the pointer wherever it goes, as the window does.
    fn clear_of_kit_chrome(&self, step: &Step) -> anyhow::Result<()> {
        let app = self.hands.app();
        let at = match step {
            Step::Move(at) if app.session().gesture.is_none() => *at,
            Step::Press(at)
            | Step::Click(at)
            | Step::DoubleClick(at)
            | Step::TripleClick(at)
            | Step::RightClick(at)
            | Step::Drag(at, _)
            | Step::ScrollAt(at, _) => *at,
            _ => return Ok(()),
        };
        match script::kit_chrome_at(app, at) {
            Some(chrome) => anyhow::bail!(
                "({}, {}) is {chrome} in the Kit window; keep pointer steps on the canvas",
                at.x,
                at.y
            ),
            None => Ok(()),
        }
    }

    /// Drops the files at `paths` at `at`, as a window gets them: whole
    /// paths with no `..` in them, a relative one read from the canvas's
    /// folder.
    fn drop_files(&mut self, paths: &[PathBuf], at: Option<Vec2>) -> anyhow::Result<()> {
        let real = |path: PathBuf| std::fs::canonicalize(&path).unwrap_or(path);
        let space = self
            .hands
            .sink()
            .space_folder()
            .map(|folder| real(folder.to_owned()));
        let files = (paths.iter())
            .map(|path| match &space {
                Some(space) if path.is_relative() => real(space.join(path)),
                _ => path.clone(),
            })
            .map(|path| crate::app::drop_run::dropped_file(&path, space.as_deref()))
            .collect();
        self.drive(|app| app.send(Event::FilesDropped { files, screen: at }))
    }

    /// Writes the canvas as a save would and opens what was written.
    fn reopen(&mut self) -> anyhow::Result<()> {
        let text = script::saved_text(self.hands.app())?;
        let document = Document::from_canvas_str(&text).context("reading the saved canvas")?;
        self.drive(|app| app.send(Event::DocumentOpened(Box::new(document))))
    }

    /// Undoes until nothing is left and checks the canvas is the start.
    fn undo_to_start(&mut self, except: &[String]) -> anyhow::Result<()> {
        for _ in 0..UNDO_LIMIT {
            if !self.hands.app().can_undo() {
                break;
            }
            self.drive(|app| app.act(Action::Undo))?;
        }
        self.checks.back_at_start(self.hands.app(), except)
    }
}
