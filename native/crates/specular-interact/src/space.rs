//! [`Space`]: the folder of canvases the app has open.
//!
//! Every canvas in the space is held whole: its [`Document`], its
//! [`History`] and its own view (camera and selection). Exactly one is
//! active. The active canvas's state is [`App`]'s own `document`, `history`
//! and `session`, so everything that reads the app reads the active canvas
//! with no indirection; the others are parked in their [`Canvas`] entry.
//! Switching moves the three out and the other three in. Nothing is
//! serialised and a parked canvas keeps its undo stack.

mod background;
mod names;
mod ops;
mod tab_ref;

use specular_core::Camera;
use specular_doc::{Document, History};

pub(crate) use self::background::in_background;
pub use self::names::{DEFAULT_CANVAS_NAME, canvas_file_name, legacy_canvas_file_name};
pub(crate) use self::ops::{act, create, file_changed, open};
pub use self::tab_ref::{TabRefError, resolve_tab_ref};
use crate::saved::{self, LoadedFits};
use crate::{App, Selection};

/// Names one canvas of the space: the tab id the space's index keeps for it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanvasId(String);

impl CanvasId {
    /// The id `text` spells.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CanvasId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The folder of canvases the app has open, and which one is active. There
/// is always at least one canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    /// The folder, or `None` for a canvas that is in no folder: a demo, a
    /// test, a benchmark.
    folder: Option<String>,
    canvases: Vec<Canvas>,
    /// Index of the active canvas.
    active: usize,
    /// How many times another canvas became the active one.
    switches: u64,
}

/// One canvas of the space.
#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    /// Its id.
    pub id: CanvasId,
    /// The name the user gave it. Unique in the space once trimmed.
    pub name: String,
    /// Its file's name inside the space folder.
    pub file: String,
    state: CanvasState,
}

/// Where a canvas's document is.
#[derive(Debug, Clone, PartialEq)]
enum CanvasState {
    /// In [`App`]'s own fields.
    Active,
    /// Here, untouched until the canvas is switched to.
    Parked(Box<Parked>),
}

/// A background canvas, whole.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Parked {
    pub(crate) document: Document,
    pub(crate) history: History<Selection>,
    pub(crate) view: CanvasView,
}

/// The part of the session that belongs to one canvas.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CanvasView {
    pub(crate) camera: Camera,
    pub(crate) selection: Selection,
    pub(crate) loaded_fits: LoadedFits,
    /// Whether the document's texts have been measured since it was read.
    /// A canvas that has never been the active one has not.
    pub(crate) fitted: bool,
}

/// A space as the shell read it from disk.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedSpace {
    /// The space folder.
    pub folder: Option<String>,
    /// Every canvas in it, in the order the space lists them. With none,
    /// an empty first canvas is made.
    pub canvases: Vec<OpenedCanvas>,
    /// The canvas to show. `None`, or an id that is not listed, shows the
    /// first.
    pub active: Option<CanvasId>,
}

/// One canvas as the shell read it from disk.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedCanvas {
    /// Its id.
    pub id: CanvasId,
    /// Its name.
    pub name: String,
    /// Its file's name inside the space folder.
    pub file: String,
    /// What the file holds.
    pub document: Document,
    /// The camera it was saved with, or the one to start from.
    pub camera: Camera,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            folder: None,
            canvases: vec![Canvas {
                id: CanvasId::new("tab_1"),
                name: DEFAULT_CANVAS_NAME.to_owned(),
                file: format!("{DEFAULT_CANVAS_NAME}.canvas"),
                state: CanvasState::Active,
            }],
            active: 0,
            switches: 0,
        }
    }
}

impl Space {
    /// The space folder, if the canvases are in one.
    pub fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Every canvas, in the space's order.
    pub fn canvases(&self) -> &[Canvas] {
        &self.canvases
    }

    /// The canvas the window shows.
    pub fn active(&self) -> &Canvas {
        // `active` is kept in range by everything that changes the list.
        &self.canvases[self.active]
    }

    /// The canvas `id` names.
    pub fn canvas(&self, id: &CanvasId) -> Option<&Canvas> {
        self.canvases.iter().find(|canvas| canvas.id == *id)
    }

    /// The canvas a caller outside the app named with `tab_ref`: an id, or
    /// an exact name.
    pub fn resolve(&self, tab_ref: &str) -> Result<&Canvas, TabRefError> {
        resolve_tab_ref(&self.canvases, tab_ref)
    }

    /// Counts the times another canvas became the active one, so a caller
    /// can tell the document it looked at before from the one there now.
    pub(crate) fn switches(&self) -> u64 {
        self.switches
    }

    pub(crate) fn index_of(&self, id: &CanvasId) -> Option<usize> {
        self.canvases.iter().position(|canvas| canvas.id == *id)
    }
}

impl Canvas {
    /// Whether this is the canvas the window shows.
    pub fn is_active(&self) -> bool {
        matches!(self.state, CanvasState::Active)
    }

    fn parked(&self) -> Option<&Parked> {
        match &self.state {
            CanvasState::Active => None,
            CanvasState::Parked(parked) => Some(parked),
        }
    }
}

impl App {
    /// The space and its canvases.
    pub fn space(&self) -> &Space {
        &self.space
    }

    /// The document of the canvas `id` names, active or not.
    pub fn canvas_document(&self, id: &CanvasId) -> Option<&Document> {
        let canvas = self.space.canvas(id)?;
        Some(
            canvas
                .parked()
                .map_or(&self.document, |parked| &parked.document),
        )
    }

    /// What a save of the canvas `id` writes: its document, with every text
    /// the session left alone at the size it was read with, and its camera.
    pub fn canvas_to_save(&self, id: &CanvasId) -> Option<(Document, Camera)> {
        let canvas = self.space.canvas(id)?;
        Some(match canvas.parked() {
            None => (self.document_to_save(), self.session.camera),
            Some(parked) => (
                saved::to_save(&parked.document, &parked.view.loaded_fits),
                parked.view.camera,
            ),
        })
    }

    /// An app whose active canvas is the background canvas `id`, for
    /// reading it the way the active one is read. It holds a copy of that
    /// one document and nothing of the others. `None` when `id` is the
    /// active canvas or names none.
    pub fn background(&self, id: &CanvasId) -> Option<Self> {
        let canvas = self.space.canvas(id)?;
        let parked = canvas.parked()?;
        let mut app = Self {
            document: parked.document.clone(),
            tool_defaults: self.tool_defaults.clone(),
            theme: self.theme,
            measure: self.measure.clone(),
            ..Self::default()
        };
        app.session.camera = parked.view.camera;
        app.session.selection = parked.view.selection.clone();
        app.session.loaded_fits = parked.view.loaded_fits.clone();
        app.session.viewport = self.session.viewport;
        app.session.now_ms = self.session.now_ms;
        app.space = Space {
            folder: self.space.folder.clone(),
            canvases: vec![Canvas {
                id: canvas.id.clone(),
                name: canvas.name.clone(),
                file: canvas.file.clone(),
                state: CanvasState::Active,
            }],
            active: 0,
            switches: 0,
        };
        Some(app)
    }
}
