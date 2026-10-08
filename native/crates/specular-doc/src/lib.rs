//! The typed canvas document.
//!
//! A [`Document`] holds entities, edges and annotations as plain structs.
//! Every mutation is a [`Command`] passed to [`Document::apply`], which
//! returns the command that undoes it. [`History`] is the undo and redo
//! stacks built on those inverses.
//!
//! This crate is the bottom of the dependency graph: no window, GPU, CEF or
//! file I/O. Field names and optionality follow the `.canvas` format so the
//! reader and writer are a serde layer over these types. Types whose wire
//! shape matches their in-memory shape ([`Edge`], [`Annotation`],
//! [`PageAnchor`], [`Stroke`], the value enums) derive serde here. [`Entity`]
//! does not, because a node spreads its fields between the top level and the
//! `specular` extension object depending on its kind.
//!
//! `extra` fields hold JSON this crate does not model, so a load and save
//! never drops another tool's data.

mod anchor;
mod annotation;
mod canvas;
mod color;
mod command;
mod document;
mod edge;
mod entity;
mod geometry;
mod history;
mod id;
mod kinds;
mod presets;

pub use anchor::{AnchorElement, PageAnchor};
pub use annotation::{Annotation, AnnotationAnchor, AnnotationStatus, Author, RegionAnchor, Reply};
pub use canvas::{CanvasError, tidy_json};
pub use color::{Color, ColorPreset};
pub use command::{Command, CommandError};
pub use document::Document;
pub use edge::{Edge, EdgeEnd, EdgeKind, EdgeSide, LineStyle};
pub use entity::{Entity, Kind};
pub use geometry::{Point, Rect};
pub use history::History;
pub use id::{AnnotationId, EdgeId, EntityId, ItemId};
pub use kinds::{
    BorderStyle, BrushType, ColorScheme, Drawing, FileRef, FillStyle, Group, LayoutMode, ObjectFit,
    Page, PageSource, Shape, ShapeKind, Stroke, Text, TextAlign, TextFont, TextStyle,
    VerticalAlign, WidthMode,
};
pub use presets::{LAPTOP, VIEWPORT_PRESETS, ViewportPreset, preset};

/// A JSON object, used for metadata and for unmodeled passthrough fields.
pub type JsonMap = serde_json::Map<String, serde_json::Value>;
