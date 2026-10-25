//! One step of a script, done headless.

use std::path::PathBuf;

use anyhow::Context as _;
use glam::Vec2;
use specular_doc::Document;
use specular_interact::{Action, ControlId, Driver, Event, PageNotice};

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
                script::check_sent(self.app.app(), &self.sent, &fact)
            }
            Step::Expect(_) | Step::State(_) | Step::Mark(_) | Step::Save(_) => self.do_step(step),
            _ => {
                self.sent.clear();
                self.do_step(step)
            }
        }
    }

    fn do_step(&mut self, step: Step) -> anyhow::Result<()> {
        match step {
            Step::Move(at) => self.drive(|app| app.pointer_move(at)),
            Step::Press(at) => self.drive(|app| app.press(at)),
            Step::DragTo(at) => self.drive(|app| app.drag_to(at)),
            Step::Release => self.drive(|app| app.release()),
            Step::ReleaseAt(at) => self.drive(|app| app.drag_to(at).release()),
            Step::Click(at) => self.drive(|app| app.pointer_move(at).click(at)),
            Step::DoubleClick(at) => self.drive(|app| app.pointer_move(at).double_click(at)),
            Step::TripleClick(at) => self.drive(|app| app.pointer_move(at).triple_click(at)),
            Step::RightClick(at) if self.laid_out() => self.drive(|app| app.right_click(at)),
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
                self.stand_ins.clipboard = Some(text);
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
                self.run_pages_for(std::time::Duration::from_millis(ms))?;
                self.now_ms += ms;
                let now = self.now_ms;
                self.drive(|app| app.tick(now))?;
                self.settle()
            }
            // The settings dialog is never a drawn panel: its switches are
            // pressed from the models.
            Step::Control(id) if self.laid_out() && !id.starts_with("settings.") => {
                self.control(&id, |app, at| app.pointer_move(at).click(at))
            }
            Step::HoverControl(id) if self.laid_out() => self.control(&id, Driver::pointer_move),
            Step::PressControl(id) if self.laid_out() => {
                self.control(&id, |app, at| app.pointer_move(at).press(at))
            }
            Step::Control(id) => {
                self.app.control(&id)?;
                self.drive(|app| app)
            }
            // With nothing laid out there is nothing to point at or hold
            // down. The name must still be a control's.
            Step::HoverControl(id) | Step::PressControl(id) => {
                specular_interact::control_named(self.app.app(), &id)?;
                Ok(())
            }
            Step::Panels(on) => self.drive(|app| app.send(Event::BuiltinPanels(on))),
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
            Step::Snapshot(path) => self.snapshot(&path),
            Step::Save(path) => self.save(&path),
            Step::State(path) => {
                let path = self.out.join(path);
                let text = script::dump(self.app.app());
                std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
            }
            Step::Expect(expect) => self.checks.check(self.app.app(), &expect),
            Step::Mark(name) => self.checks.mark(self.app.app(), &name),
            Step::Reopen => self.reopen(),
            Step::UndoToStart(except) => self.undo_to_start(&except),
            Step::Window(WindowStep::Drop(paths, at)) => self.drop_files(&paths, at),
            // The run skips these before they get here.
            Step::Window(_) => Ok(()),
            Step::Canvas(step) => step
                .event(self.app.app())
                .and_then(|event| self.drive(|app| app.send(event))),
        }?;
        self.frame_for_documents()
    }

    /// Drops the files at `paths` at `at`, as a window gets them: whole
    /// paths with no `..` in them, a relative one read from the canvas's
    /// folder.
    fn drop_files(&mut self, paths: &[PathBuf], at: Option<Vec2>) -> anyhow::Result<()> {
        let real = |path: PathBuf| std::fs::canonicalize(&path).unwrap_or(path);
        let space = self.space.clone().map(real);
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
        let text = script::saved_text(self.app.app())?;
        let document = Document::from_canvas_str(&text).context("reading the saved canvas")?;
        self.drive(|app| app.send(Event::DocumentOpened(Box::new(document))))
    }

    /// Undoes until nothing is left and checks the canvas is the start.
    fn undo_to_start(&mut self, except: &[String]) -> anyhow::Result<()> {
        for _ in 0..UNDO_LIMIT {
            if !self.app.app().can_undo() {
                break;
            }
            self.drive(|app| app.act(Action::Undo))?;
        }
        self.checks.back_at_start(self.app.app(), except)
    }

    /// Whether the built-in panels are on: a control is then clicked where
    /// they lay it out. Off, a control is activated from the models.
    fn laid_out(&self) -> bool {
        self.app.session().panel.built_in
    }

    /// Points at the middle of the control named `id` and does `input`
    /// there. A name that is not shown fails the run with the ones that are.
    fn control(
        &mut self,
        id: &str,
        input: impl for<'app> FnOnce(&'app mut Driver, Vec2) -> &'app mut Driver,
    ) -> anyhow::Result<()> {
        let layout = specular_interact::panel::builtin::layout(self.app.app());
        let Some(node) = layout.node(&ControlId::from(id.to_owned())) else {
            let shown: Vec<&str> = layout.controls().map(ControlId::as_str).collect();
            anyhow::bail!("no control `{id}` is shown; these are: {}", shown.join(" "));
        };
        let at = node.rect.centre();
        self.drive(|app| input(app, at))
    }
}
