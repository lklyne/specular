//! The toolbar and the item popup as data: [`toolbar`] and [`popup_for`] say
//! which controls exist now, their state and the [`Action`](crate::Action)
//! each one dispatches. A renderer draws them and sends the action of a
//! pressed control back as an [`Event::Action`](crate::Event).
//!
//! The models hold no sizes, positions or colors in pixels, and no hover,
//! press or open state; a renderer keeps those against [`ControlId`].
//! [`builtin`] is the renderer that needs no UI library: it lays the models
//! out, keeps that state in the session and routes the pointer, and
//! `specular-scene` draws what it laid out.

mod build;
pub mod builtin;
mod icon;
mod id;
mod model;
mod models;
mod popup;
mod toolbar;

pub use icon::Icon;
pub use id::ControlId;
pub use model::{
    Button, Control, Dropdown, DropdownOption, DropdownSection, Entries, Face, Label, OptionLayout,
    PaintRole, Palette, Stepper, Swatch, Swatches, Toggle,
};
pub use models::{
    Align, Placement, PopupAnchor, PopupModel, ToolButton, ToolbarModel, ToolbarSection,
};
pub use popup::popup_for;
pub use toolbar::toolbar;
