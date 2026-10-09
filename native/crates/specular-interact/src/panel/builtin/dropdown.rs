//! The list under an open dropdown: its sections stacked in a floating
//! frame.

use glam::Vec2;
use specular_doc::{TextAlign, TextFont};

use super::super::{Control, Dropdown, DropdownOption, DropdownSection, OptionLayout};
use super::controls::{self, MEDIUM, RowKind, text};
use super::metrics::{
    CELL, CELL_ICON, CHEVRON, CONTROL_RADIUS, CONTROLS_GAP, FONT_LIST_MIN, GAP, GRID_CELL,
    GRID_GAP, GRID_ICON, INSET, KEY_PAD, LIST_MIN, LIST_OFFSET, MENU_OFFSET, PRESET_OFFSET,
    PRESETS, PRESETS_WIDE, ROW, ROW_GAP, ROW_PAD, ROW_RADIUS, ROW_TALL, SECTION_GAP,
    SECTION_RULE_INSET, SEGMENTED_ROW, STEPPER_INSET, TEXT_LINE,
};
use super::node::{Chrome, Node, Panel, PanelRect, Part, Run, Surface, Tone};
use super::{Ctx, place};

/// How a dropdown's lists mark the current choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListStyle {
    /// A check at the end of the row: text sizes, typefaces.
    Menu,
    /// The row filled in, with a size or a key hint at its end: zoom levels,
    /// page sizes.
    Presets,
}

fn options_of(content: &[DropdownSection]) -> impl Iterator<Item = &DropdownOption> {
    content.iter().flat_map(|section| match section {
        DropdownSection::Options { options, .. } => options.as_slice(),
        DropdownSection::Controls(_) => &[],
    })
}

/// Whether any option previews a typeface, which a menu of sizes does not.
fn has_fonts(content: &[DropdownSection]) -> bool {
    options_of(content).any(|option| option.face.font.is_some())
}

/// Whether `controls` hold labelled toggles, the segments of a row that is
/// as wide as its list.
fn has_segments(controls: &[Control]) -> bool {
    controls.iter().any(|control| match control {
        Control::Toggle(toggle) => toggle.face.icon.is_some() && toggle.face.text.is_some(),
        Control::Button(_)
        | Control::Stepper(_)
        | Control::Field(_)
        | Control::Swatches(_)
        | Control::Dropdown(_)
        | Control::Choices(_)
        | Control::Separator => false,
    })
}

fn has_stepper(controls: &[Control]) -> bool {
    controls.iter().any(|control| match control {
        Control::Stepper(_) => true,
        Control::Button(_)
        | Control::Toggle(_)
        | Control::Field(_)
        | Control::Swatches(_)
        | Control::Dropdown(_)
        | Control::Choices(_)
        | Control::Separator => false,
    })
}

fn label_of(option: &DropdownOption) -> super::super::Label {
    (option.face.text.clone()).unwrap_or_else(|| option.label.clone())
}

fn row_height(option: &DropdownOption) -> f32 {
    if option.face.font.is_some() {
        ROW_TALL
    } else {
        ROW
    }
}

struct Lists<'a, 'b> {
    ctx: &'a Ctx<'b>,
    style: ListStyle,
    /// The width of the content, which every section fills.
    width: f32,
}

impl Lists<'_, '_> {
    fn choice(&self, option: &DropdownOption, rect: PanelRect, chrome: Chrome) -> Node {
        Node {
            id: Some(option.id.clone()),
            rect,
            radius: CONTROL_RADIUS,
            chrome,
            state: self.ctx.state(&option.id, option.enabled, option.selected),
            parts: Vec::new(),
            run: Some(Run::Act {
                action: option.action.clone(),
                closes: true,
            }),
        }
    }

    fn list_row(&self, option: &DropdownOption, top: f32) -> Node {
        let rect = PanelRect::new(0.0, top, self.width, row_height(option));
        let inner = PanelRect::new(ROW_PAD, top, self.width - ROW_PAD * 2.0, rect.height);
        let font = option.face.font;
        match self.style {
            ListStyle::Menu => {
                let mut parts = vec![text(
                    label_of(option),
                    inner,
                    TextAlign::Left,
                    font,
                    Tone::Strong,
                )];
                if option.selected {
                    let mark = PanelRect::new(
                        inner.right() - CHEVRON,
                        rect.centre().y - CHEVRON / 2.0,
                        CHEVRON,
                        CHEVRON,
                    );
                    parts.push(Part::Check { rect: mark });
                }
                Node {
                    radius: ROW_RADIUS,
                    parts,
                    ..self.choice(option, rect, Chrome::MenuRow)
                }
            }
            ListStyle::Presets => {
                let mut parts = vec![Part::Text {
                    text: label_of(option),
                    rect: inner,
                    align: TextAlign::Left,
                    font: font.unwrap_or(TextFont::Sans),
                    weight: MEDIUM,
                    tone: Tone::Follow,
                }];
                if let Some(trailing) = &option.trailing {
                    parts.push(text(
                        trailing.clone(),
                        inner,
                        TextAlign::Right,
                        None,
                        Tone::Muted,
                    ));
                }
                if let Some(chord) = option.chord {
                    let keys = chord.text();
                    let width = self.ctx.text_width(&keys, TextFont::Sans) + KEY_PAD.0 * 2.0;
                    let height = TEXT_LINE;
                    let rect = PanelRect::new(
                        inner.right() - width,
                        rect.centre().y - height / 2.0,
                        width,
                        height,
                    );
                    parts.push(Part::Key {
                        text: keys.into(),
                        rect,
                    });
                }
                Node {
                    parts,
                    ..self.choice(option, rect, Chrome::PresetRow)
                }
            }
        }
    }

    /// A cell of a row or a grid of glyph choices.
    fn cell(&self, option: &DropdownOption, rect: PanelRect, glyph: f32) -> Node {
        let part = match option.face.icon {
            Some(icon) => Part::Icon {
                icon,
                rect: rect.centred(Vec2::splat(glyph)),
                tint: None,
            },
            None => text(
                label_of(option),
                rect,
                TextAlign::Center,
                option.face.font,
                Tone::Follow,
            ),
        };
        Node {
            parts: vec![part],
            ..self.choice(option, rect, Chrome::Button)
        }
    }

    /// The nodes of `section` from `top`, and its height.
    fn section(&self, section: &DropdownSection, top: f32) -> (Vec<Node>, f32) {
        match section {
            DropdownSection::Options { layout, options } => match layout {
                OptionLayout::List => {
                    let mut y = top;
                    let mut nodes = Vec::new();
                    for option in options {
                        nodes.push(self.list_row(option, y));
                        y += row_height(option);
                    }
                    (nodes, y - top)
                }
                OptionLayout::Row => self.cells(options, options.len(), CELL, GAP, CELL_ICON, top),
                OptionLayout::Grid { columns } => self.cells(
                    options,
                    usize::from(*columns),
                    GRID_CELL,
                    GRID_GAP,
                    GRID_ICON,
                    top,
                ),
            },
            DropdownSection::Controls(row) => {
                let inset = if has_stepper(row) { STEPPER_INSET } else { 0.0 };
                let fill = self.width - inset * 2.0;
                let laid = controls::row(self.ctx, row, RowKind::Dropdown, Some(fill));
                let by = Vec2::new(inset, top);
                let height = laid.height;
                let nodes = laid.nodes.into_iter().map(|node| node.moved(by)).collect();
                (nodes, height + inset)
            }
        }
    }

    fn cells(
        &self,
        options: &[DropdownOption],
        columns: usize,
        side: f32,
        gap: f32,
        glyph: f32,
        top: f32,
    ) -> (Vec<Node>, f32) {
        let columns = columns.max(1);
        let nodes: Vec<Node> = (options.iter().enumerate())
            .map(|(index, option)| {
                let (column, line) = (index % columns, index / columns);
                let rect = PanelRect::new(
                    column as f32 * (side + gap),
                    top + line as f32 * (side + gap),
                    side,
                    side,
                );
                self.cell(option, rect, glyph)
            })
            .collect();
        let lines = options.len().div_ceil(columns) as f32;
        (nodes, lines * side + (lines - 1.0).max(0.0) * gap)
    }
}

fn section_width(ctx: &Ctx<'_>, section: &DropdownSection, style: ListStyle) -> f32 {
    let cells = |count: usize, side: f32, gap: f32| {
        count as f32 * side + count.saturating_sub(1) as f32 * gap
    };
    match section {
        DropdownSection::Options { layout, options } => match (layout, style) {
            (OptionLayout::List, ListStyle::Presets) => {
                if options.iter().any(|option| option.trailing.is_some()) {
                    PRESETS_WIDE
                } else {
                    PRESETS
                }
            }
            (OptionLayout::List, ListStyle::Menu) => {
                let widest = (options.iter())
                    .map(|option| {
                        let font = option.face.font.unwrap_or(TextFont::Sans);
                        ctx.text_width(&label_of(option), font)
                    })
                    .fold(0.0, f32::max);
                let least = if options.iter().any(|option| option.face.font.is_some()) {
                    FONT_LIST_MIN
                } else {
                    LIST_MIN
                };
                (ROW_PAD * 2.0 + widest + ROW_GAP + CHEVRON).max(least - INSET * 2.0)
            }
            (OptionLayout::Row, _) => cells(options.len(), CELL, GAP),
            (OptionLayout::Grid { columns }, _) => {
                cells(usize::from(*columns).max(1), GRID_CELL, GRID_GAP)
            }
        },
        DropdownSection::Controls(row) if has_segments(row) => SEGMENTED_ROW,
        DropdownSection::Controls(row) => {
            let inset = if has_stepper(row) { STEPPER_INSET } else { 0.0 };
            controls::natural_width(ctx, row, RowKind::Dropdown) + inset * 2.0
        }
    }
}

/// The line between two sections, and the room it takes with its margins.
fn divider(style: ListStyle, both_controls: bool, top: f32, width: f32) -> (Node, f32) {
    let room = if both_controls {
        CONTROLS_GAP
    } else {
        SECTION_GAP
    };
    let (chrome, inset) = match style {
        ListStyle::Presets => (Chrome::Divider, 0.0),
        ListStyle::Menu if both_controls => (Chrome::Rule, 0.0),
        ListStyle::Menu => (Chrome::Rule, SECTION_RULE_INSET),
    };
    let line = PanelRect::new(inset, top + (room - 1.0) / 2.0, width - inset * 2.0, 1.0);
    (Node::fixed(line, chrome), room)
}

const fn is_controls(section: &DropdownSection) -> bool {
    match section {
        DropdownSection::Controls(_) => true,
        DropdownSection::Options { .. } => false,
    }
}

/// Sections of choices laid out from their own top-left corner.
pub(super) struct Body {
    pub(super) nodes: Vec<Node>,
    /// The size of the sections, without a frame around them.
    pub(super) size: Vec2,
    style: ListStyle,
}

impl Body {
    /// Whether the choices are a list of words marked with checks, which is
    /// drawn as a menu rather than on the surface the popups share.
    pub(super) fn is_menu(&self) -> bool {
        self.style == ListStyle::Menu
    }
}

/// `content` stacked in a column, set apart by lines.
pub(super) fn body(ctx: &Ctx<'_>, content: &[DropdownSection]) -> Body {
    let style = if options_of(content).any(|it| it.trailing.is_some() || it.chord.is_some()) {
        ListStyle::Presets
    } else {
        ListStyle::Menu
    };
    let width = (content.iter())
        .map(|section| section_width(ctx, section, style))
        .fold(0.0, f32::max);
    let lists = Lists { ctx, style, width };
    let mut nodes = Vec::new();
    let mut top = 0.0;
    let mut before: Option<&DropdownSection> = None;
    for section in content {
        if let Some(before) = before {
            let both = is_controls(before) && is_controls(section);
            let (line, room) = divider(style, both, top, width);
            nodes.push(line);
            top += room;
        }
        let (laid, height) = lists.section(section, top);
        nodes.extend(laid);
        top += height;
        before = Some(section);
    }
    Body {
        nodes,
        size: Vec2::new(width, top),
        style,
    }
}

/// The list of `dropdown`, hung from `hang`, the bottom of the row `trigger`
/// is in.
pub(super) fn layout(
    ctx: &Ctx<'_>,
    dropdown: &Dropdown,
    trigger: PanelRect,
    hang: f32,
    viewport: Vec2,
) -> Panel {
    let content = &dropdown.content;
    let Body { nodes, size, style } = body(ctx, content);
    let size = size + Vec2::splat(INSET * 2.0);
    // A list of words lines up with the start of its trigger; anything else
    // is centred under it.
    let words = style == ListStyle::Menu
        && content.iter().any(|section| match section {
            DropdownSection::Options { layout, .. } => *layout == OptionLayout::List,
            DropdownSection::Controls(_) => false,
        });
    let sized = options_of(content).any(|option| option.trailing.is_some());
    let offset = if words {
        MENU_OFFSET
    } else if sized {
        PRESET_OFFSET
    } else {
        LIST_OFFSET
    };
    let corner = place::hanging(trigger, hang, offset, size.x, words, viewport.x).round();
    let inset = corner + Vec2::splat(INSET);
    Panel {
        surface: Surface::Dropdown,
        rect: PanelRect::new(corner.x, corner.y, size.x, size.y),
        menu: words && !has_fonts(content),
        nodes: (nodes.into_iter()).map(|node| node.moved(inset)).collect(),
    }
}
