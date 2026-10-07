//! [`TestApp`]: construction, accessors, undo and the undo assertion.

use specular_doc::{Document, Entity, EntityId, Rect};
use specular_interact::{Action, App, Effect, Event, Selection, Session, update};

use crate::{doc_snapshot, fixtures};

/// More undo steps than any test records. Reaching it means undo is not
/// consuming its stack.
const UNDO_LIMIT: usize = 10_000;

/// An [`App`] with no window, GPU or page backend, driven by scripted input.
#[derive(Debug, Clone)]
pub struct TestApp {
    pub(crate) app: App,
    /// What `update` has returned since the last drain.
    pub(crate) effects: Vec<Effect>,
    pub(crate) input: crate::input::InputState,
    /// The document the test started from.
    start: Document,
}

impl TestApp {
    /// An app showing `document` at zoom 1 with no pan. The effects of
    /// opening it (hosting its pages) are dropped; use [`TestApp::open`] on
    /// an [`TestApp::empty`] app to assert on them.
    pub fn from_document(document: Document) -> Self {
        let mut app = Self::empty();
        app.open(document).take_effects();
        app
    }

    /// An app showing the `.canvas` file text `json`.
    #[track_caller]
    pub fn from_canvas(json: &str) -> Self {
        match Document::from_canvas_str(json) {
            Ok(document) => Self::from_document(document),
            Err(error) => panic!("the canvas could not be loaded: {error:?}"),
        }
    }

    /// An app showing `entities`, back-to-front.
    #[track_caller]
    pub fn with_entities(entities: impl IntoIterator<Item = Entity>) -> Self {
        Self::from_document(fixtures::document(entities))
    }

    /// An app showing `count` pages from [`pages`](crate::pages): `p1` at
    /// (100, 100), `p2` at (700, 100), each 400x300.
    pub fn with_pages(count: usize) -> Self {
        Self::with_entities(fixtures::pages(count))
    }

    /// An app with an empty document.
    pub fn empty() -> Self {
        Self {
            app: App::new(0),
            effects: Vec::new(),
            input: crate::input::InputState::default(),
            start: Document::new(),
        }
    }

    /// Replaces the document, as a load does. It becomes the start that
    /// [`TestApp::assert_undo_returns_to_start`] compares against.
    pub fn open(&mut self, document: Document) -> &mut Self {
        self.start = document.clone();
        self.send(Event::DocumentOpened(Box::new(document)))
    }

    /// Sends one event through [`update`] and keeps the effects. Every other
    /// input method ends up here.
    pub fn send(&mut self, event: Event) -> &mut Self {
        let effects = update(&mut self.app, event);
        self.effects.extend(effects);
        self
    }

    /// The app under test.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The document.
    pub fn document(&self) -> &Document {
        self.app.document()
    }

    /// The unsaved state: camera, tool, gesture, hover, focus.
    pub fn session(&self) -> &Session {
        self.app.session()
    }

    /// What is selected.
    pub fn selection(&self) -> &Selection {
        &self.app.session().selection
    }

    /// The selected entity's id, when exactly one entity is selected.
    pub fn selected(&self) -> Option<&str> {
        self.selection().single_entity().map(EntityId::as_str)
    }

    /// The entity `id`.
    #[track_caller]
    pub fn entity(&self, id: &str) -> &Entity {
        match self.document().entity(&EntityId::from(id)) {
            Some(entity) => entity,
            None => panic!("the document has no entity {id:?}"),
        }
    }

    /// The rect of the entity `id`.
    #[track_caller]
    pub fn rect(&self, id: &str) -> Rect {
        self.entity(id).rect
    }

    /// The effects returned since the last [`TestApp::take_effects`].
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    /// Drains the effects returned since the last drain.
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    /// Runs an [`Action`], as a menu item, a panel or the API would.
    pub fn act(&mut self, action: Action) -> &mut Self {
        self.send(Event::Action(action))
    }

    /// Undoes one step.
    pub fn undo(&mut self) -> &mut Self {
        self.act(Action::Undo)
    }

    /// Redoes one step.
    pub fn redo(&mut self) -> &mut Self {
        self.act(Action::Redo)
    }

    /// The document as stable text. See [`doc_snapshot`].
    pub fn doc_snapshot(&self) -> String {
        doc_snapshot(self.document())
    }

    /// Asserts that undoing every step gives back the document the test
    /// started from, and that redoing them all gives back the document as it
    /// is now. The app ends as it began, and the effects of the undos and
    /// redos are not kept, so this can go anywhere in a test.
    #[track_caller]
    pub fn assert_undo_returns_to_start(&mut self) -> &mut Self {
        assert!(
            self.session().gesture.is_none(),
            "a gesture is in flight, and undo does nothing until it is released or cancelled"
        );
        let end = self.document().clone();
        let kept = self.take_effects();

        let mut steps = 0;
        while self.app.can_undo() {
            assert!(steps < UNDO_LIMIT, "undo never ran out of steps");
            self.undo();
            steps += 1;
        }
        assert_same(
            self.document(),
            &self.start,
            &format!("undoing all {steps} steps did not return to the starting document"),
        );

        for _ in 0..steps {
            self.redo();
        }
        assert_same(
            self.document(),
            &end,
            &format!("redoing all {steps} steps did not return to the document before the undos"),
        );

        self.effects = kept;
        self
    }
}

/// Fails with the two snapshots, or with the full structs when they differ
/// by less than a snapshot shows (it rounds to a hundredth).
#[track_caller]
fn assert_same(actual: &Document, expected: &Document, message: &str) {
    assert_eq!(doc_snapshot(actual), doc_snapshot(expected), "{message}");
    assert_eq!(actual, expected, "{message}");
}
