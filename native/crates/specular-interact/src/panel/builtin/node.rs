//! What [`layout`](super::layout) hands a drawer: panels of nodes, each a
//! rect with the parts painted in it.

use glam::Vec2;
use specular_doc::{Color, TextAlign, TextFont};

use super::super::{ControlId, Icon, Label, PaintRole, Palette};
use crate::Action;

/// An axis-aligned rect in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PanelRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl PanelRect {
    /// A rect.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// The middle.
    pub fn centre(self) -> Vec2 {
        Vec2::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether `point` is inside: left and top edges count, right and bottom
    /// do not, so two rects side by side never both claim a point.
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.right() && point.y < self.bottom()
    }

    /// The rect of `size` centred in this one.
    #[must_use]
    pub fn centred(self, size: Vec2) -> Self {
        let corner = self.centre() - size / 2.0;
        Self::new(corner.x, corner.y, size.x, size.y)
    }

    /// The same rect moved by `by`.
    #[must_use]
    pub fn moved(self, by: Vec2) -> Self {
        Self::new(self.x + by.x, self.y + by.y, self.width, self.height)
    }
}

/// Which panel a node belongs to, which decides the surface drawn under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// The strip across the top of the viewport.
    Toolbar,
    /// The floating popup of a tool or a selection.
    Popup,
    /// The floating list under an open dropdown.
    Dropdown,
    /// The sidebar's frame: its ground, its right edge and the head of the
    /// Canvases list.
    Sidebar,
    /// The sidebar's scrolling list. Its box is the window its nodes are
    /// seen through.
    SidebarList,
}

/// A panel: its box and what is in it, back to front.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    /// Which panel it is.
    pub surface: Surface,
    /// Its box.
    pub rect: PanelRect,
    /// Whether it is a list of words painted as a menu: white, with a
    /// zinc edge and a deeper shadow, as the text size list is, rather than
    /// on the surface every other floating panel shares.
    pub menu: bool,
    /// Its nodes in paint order.
    pub nodes: Vec<Node>,
}

/// Where the pointer is against a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pointing {
    /// Elsewhere.
    #[default]
    Away,
    /// Over it.
    Hover,
    /// Over it with the button down, the press having started on it.
    Pressed,
}

/// How a node is drawn now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeState {
    /// Whether it can be pressed.
    pub enabled: bool,
    /// Whether it is on: the active tool, a toggle that is on, the selected
    /// option, the trigger of the open dropdown.
    pub on: bool,
    /// Where the pointer is.
    pub pointing: Pointing,
    /// Whether it is drawn at half strength, as a row hooked to a document
    /// its page has left is.
    pub dimmed: bool,
}

impl NodeState {
    /// A node that is not a control.
    pub(super) const STILL: Self = Self {
        enabled: true,
        on: false,
        pointing: Pointing::Away,
        dimmed: false,
    };
}

/// What is painted in a node's own box, behind its parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chrome {
    /// Nothing.
    Plain,
    /// A toolbar button: filled when hovered or on.
    ToolButton,
    /// The trigger of a toolbar list: filled when hovered, and in the
    /// popover color while its list is open.
    ToolMenu,
    /// A popup control: filled when hovered, darker when on.
    Button,
    /// A list row that marks its choice with a check: filled when hovered.
    MenuRow,
    /// A list row that marks its choice by its fill.
    PresetRow,
    /// A color choice: a ring inside its edge when on.
    Swatch,
    /// The box around a stepper: an outline.
    Field,
    /// The box of a text field: white, with an outline, and a ring while it
    /// has the keys.
    Input,
    /// A line between groups of a bar.
    Divider,
    /// A line between sections of a list.
    Rule,
    /// A row of the sidebar: filled when it is on, quieter when hovered.
    Row,
    /// A button on the sidebar or the toolbar's edge: quiet when hovered,
    /// firmer when pressed.
    Subtle,
    /// A line across a panel in its border color.
    Edge,
    /// The thumb of a scrollbar.
    Scrollbar,
    /// The box of a name edited where it is read: white with an outline,
    /// and no ring while it has the keys.
    InlineInput,
}

/// Which of a panel's two text colors a part takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The node's own: quiet at rest, full when hovered or on.
    Follow,
    /// Always full.
    Strong,
    /// Always quiet.
    Muted,
}

/// A stored color with the surface it is resolved for.
#[derive(Debug, Clone, PartialEq)]
pub struct Tint {
    /// The color as stored.
    pub color: Color,
    /// The hues it is drawn from.
    pub palette: Palette,
    /// How it is used.
    pub role: PaintRole,
}

/// A line of text in a field, and what editing it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Input {
    /// The line: the field's value, or the text typed so far.
    pub text: Label,
    /// What to show instead while `text` is empty.
    pub hint: Option<Label>,
    /// The box the line is clipped to. The text starts at its left edge,
    /// moved by `scroll`.
    pub area: PanelRect,
    /// How far the line is scrolled left, in pixels.
    pub scroll: f32,
    /// The caret and selection, while the field has the keys.
    pub focus: Option<InputFocus>,
}

impl Input {
    /// Moves the line and everything over it by `by`.
    fn shift(&mut self, by: Vec2) {
        self.area = self.area.moved(by);
        if let Some(focus) = &mut self.focus {
            let moved = |rects: &mut Vec<PanelRect>| {
                for rect in rects {
                    *rect = rect.moved(by);
                }
            };
            moved(&mut focus.selection);
            moved(&mut focus.composition);
            focus.caret = focus.caret.map(|caret| caret.moved(by));
        }
    }
}

/// What a field being edited shows over its text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputFocus {
    /// The selected text, one box a line, to go behind the glyphs.
    pub selection: Vec<PanelRect>,
    /// The text the input method is composing, to underline.
    pub composition: Vec<PanelRect>,
    /// The caret, when it is in the shown half of its blink and nothing is
    /// selected.
    pub caret: Option<PanelRect>,
}

/// One thing painted inside a node.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    /// A glyph fitted into `rect`.
    Icon {
        /// Which glyph.
        icon: Icon,
        /// The box it is fitted into, keeping its proportions.
        rect: PanelRect,
        /// What a glyph that shows a color is painted in.
        tint: Option<Tint>,
    },
    /// One line of text, vertically centred in `rect`.
    Text {
        /// The text.
        text: Label,
        /// The box: the text starts at its left, straddles its middle or
        /// ends at its right, as `align` says.
        rect: PanelRect,
        /// Where in the box the line sits.
        align: TextAlign,
        /// The typeface.
        font: TextFont,
        /// The weight on the CSS scale.
        weight: u16,
        /// Its color.
        tone: Tone,
    },
    /// A disc of color with a hairline around it; a ring when `tint` is
    /// `None`.
    Dot {
        /// The disc's box.
        rect: PanelRect,
        /// The color, or `None` for the choice that paints nothing.
        tint: Option<Tint>,
    },
    /// The down chevron of a dropdown.
    Chevron {
        /// Its box.
        rect: PanelRect,
    },
    /// The mark beside the selected row of a list.
    Check {
        /// Its box.
        rect: PanelRect,
    },
    /// A key hint: text on a small filled box.
    Key {
        /// The keys.
        text: Label,
        /// The box.
        rect: PanelRect,
    },
    /// The line of a text field.
    Input(Input),
    /// A glyph in a color of the panel's text, whatever the node's state.
    Glyph {
        /// Which glyph.
        icon: Icon,
        /// The box it is fitted into.
        rect: PanelRect,
        /// Its color.
        tone: Tone,
    },
}

/// What pressing a control does.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Run {
    /// Opens the dropdown the control is the trigger of, or closes it.
    Toggle,
    /// Runs `action`.
    Act {
        /// What to run.
        action: Action,
        /// Whether the open dropdown closes afterwards: a choice from a
        /// list closes it, a control inside it leaves it open to be used
        /// again.
        closes: bool,
    },
    /// Starts editing the text field named so.
    Edit(ControlId),
}

/// One rect of a panel: a control, or a line between controls.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The control it is, or `None` for something that takes no press.
    pub id: Option<ControlId>,
    /// Its box.
    pub rect: PanelRect,
    /// The corner radius of what is painted in its box.
    pub radius: f32,
    /// What is painted in its box.
    pub chrome: Chrome,
    /// How it is drawn now.
    pub state: NodeState,
    /// What is painted over that, back to front.
    pub parts: Vec<Part>,
    pub(crate) run: Option<Run>,
}

impl Node {
    /// A node that takes no press: a line or a box.
    pub(super) const fn fixed(rect: PanelRect, chrome: Chrome) -> Self {
        Self {
            id: None,
            rect,
            radius: 0.0,
            chrome,
            state: NodeState::STILL,
            parts: Vec::new(),
            run: None,
        }
    }

    /// The same node moved by `by`, parts and all.
    #[must_use]
    pub(super) fn moved(mut self, by: Vec2) -> Self {
        self.rect = self.rect.moved(by);
        for part in &mut self.parts {
            match part {
                Part::Icon { rect, .. }
                | Part::Text { rect, .. }
                | Part::Dot { rect, .. }
                | Part::Chevron { rect }
                | Part::Check { rect }
                | Part::Key { rect, .. }
                | Part::Glyph { rect, .. } => *rect = rect.moved(by),
                Part::Input(input) => input.shift(by),
            }
        }
        self
    }
}
