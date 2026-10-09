//! The toolbar and the dock as data: [`toolbar`] and [`dock`] say
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
mod context;
mod field;
mod icon;
mod id;
mod model;
mod models;
mod named;
mod popup;
mod toolbar;

pub use context::{MenuTarget, context_menu};
pub(crate) use field::field_named;
pub use field::{Field, FieldSubmit, FieldWidth};
pub use icon::Icon;
pub use id::ControlId;
pub use model::{
    Button, Choices, Control, Dropdown, DropdownOption, DropdownSection, Entries, Face, Label,
    OptionLayout, PaintRole, Palette, Stepper, Swatch, Swatches, Toggle,
};
pub use models::{
    PopupModel, SidebarButton, ThemeButton, ToolButton, ToolbarModel, ToolbarSection,
};
pub use named::{UnknownControl, control_named, named_controls};
pub(crate) use named::{activate as activate_control, open_menu as open_menu_at};
pub use popup::dock;
pub use toolbar::toolbar;
