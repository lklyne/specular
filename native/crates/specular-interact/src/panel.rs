//! The toolbar and the dock as data: [`toolbar`] and [`dock`] say
//! which controls exist now, their state and the [`Action`](crate::Action)
//! each one dispatches. A renderer draws them and sends the action of a
//! pressed control back as an [`Event::Action`](crate::Event).
//!
//! The models hold no sizes, positions or colors in pixels, and no hover
//! or press state; a renderer keeps those against [`ControlId`]. A headless
//! run draws none of it and presses controls by name ([`named_controls`]).

mod build;
mod context;
mod dock;
mod field;
mod icon;
mod id;
mod model;
mod models;
mod named;
mod strip;
mod toolbar;
mod ui;

pub use context::{ContextMenu, MenuTarget, context_menu};
pub(crate) use context::{
    open as open_menu_for_right_press, open_for_press as open_menu_for_press,
};
pub use dock::{dock, tool_options};
pub use field::{Field, FieldSubmit, FieldWidth};
pub(crate) use field::{PAGE_URL, field_named};
pub use icon::Icon;
pub use id::ControlId;
pub use model::{
    Button, Choices, Control, Dropdown, DropdownOption, DropdownSection, Entries, Face, Label,
    OptionLayout, PaintRole, Palette, Stepper, Swatch, Swatches, Toggle,
};
pub use models::{
    ControlsModel, SidebarButton, ThemeButton, ToolButton, ToolbarModel, ToolbarSection,
};
pub(crate) use named::named_controls;
pub use named::{UnknownControl, control_named};
pub(crate) use named::{activate as activate_control, open_menu as open_menu_at};
pub use strip::{ViewStrip, ViewTab, view_strip};
pub use toolbar::toolbar;
pub use ui::{CHROME_HEIGHT, DOCK_ROW, PanelUi, TAB_ROW};
pub(crate) use ui::{cancel, tidy};
