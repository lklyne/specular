//! Where each node of a layout's tree sits (ADR 0059): a pure function
//! both runners read, so the window draws its blocks where a headless run
//! says the canvas's insets are.
//!
//! A row lays its children out left to right and a column top to bottom.
//! A child with a `px` size takes that many points; the rest share what is
//! left by `weight` (1 when none is given), and `min` and `max` clamp each.
//! A stack gives its whole box to the child it shows. Padding and gaps come
//! off before the children are placed.

use super::node::{Node, NodeKind, Padding, Size};

/// A box in points from the window's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    /// Its left edge.
    pub x: f32,
    /// Its top edge.
    pub y: f32,
    /// Its width.
    pub width: f32,
    /// Its height.
    pub height: f32,
}

impl Frame {
    /// The box at the origin, `width` by `height`.
    pub const fn sized(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    /// Its right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Its bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }
}

/// What the app says of a node before the tree is laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gate {
    /// As the layout says.
    AsIs,
    /// Left out: the app has nothing for it to show now.
    Hidden,
    /// Exactly this many points along its parent's axis.
    Px(f32),
}

/// One node shown, and where.
#[derive(Debug, Clone, Copy)]
pub struct Placed<'a> {
    /// The node.
    pub node: &'a Node,
    /// Its box.
    pub frame: Frame,
}

/// Every node of `root` shown in `window`, parents before children, with
/// `gate` saying what the app makes of each.
pub fn solve<'a>(root: &'a Node, window: Frame, gate: &dyn Fn(&Node) -> Gate) -> Vec<Placed<'a>> {
    let mut out = Vec::new();
    if shown(root, gate) {
        place(root, window, gate, &mut out);
    }
    out
}

fn shown(node: &Node, gate: &dyn Fn(&Node) -> Gate) -> bool {
    gate(node) != Gate::Hidden
}

fn place<'a>(
    node: &'a Node,
    frame: Frame,
    gate: &dyn Fn(&Node) -> Gate,
    out: &mut Vec<Placed<'a>>,
) {
    out.push(Placed { node, frame });
    let (vertical, horizontal) = node.padding.map_or((0.0, 0.0), Padding::axes);
    let inner = Frame {
        x: frame.x + horizontal,
        y: frame.y + vertical,
        width: (frame.width - 2.0 * horizontal).max(0.0),
        height: (frame.height - 2.0 * vertical).max(0.0),
    };
    match node.kind {
        NodeKind::Block => {}
        NodeKind::Stack => {
            if let Some(child) = node.shown_child().filter(|child| shown(child, gate)) {
                place(child, inner, gate, out);
            }
        }
        NodeKind::Row | NodeKind::Column => {
            let across = node.kind == NodeKind::Row;
            let children: Vec<&Node> = (node.children.iter())
                .filter(|child| shown(child, gate))
                .collect();
            let room = if across { inner.width } else { inner.height };
            let gaps = node.gap.unwrap_or(0.0) * children.len().saturating_sub(1) as f32;
            let sizes = share(&children, (room - gaps).max(0.0), gate);
            let mut at = if across { inner.x } else { inner.y };
            for (child, size) in children.into_iter().zip(sizes) {
                let frame = if across {
                    Frame {
                        x: at,
                        width: size,
                        ..inner
                    }
                } else {
                    Frame {
                        y: at,
                        height: size,
                        ..inner
                    }
                };
                place(child, frame, gate, out);
                at += size + node.gap.unwrap_or(0.0);
            }
        }
    }
}

/// Each child's length along the axis, out of `room`.
fn share(children: &[&Node], room: f32, gate: &dyn Fn(&Node) -> Gate) -> Vec<f32> {
    let clamp = |node: &Node, length: f32| {
        let length = node.max.map_or(length, |max| length.min(max));
        node.min.map_or(length, |min| length.max(min)).max(0.0)
    };
    let fixed = |node: &Node| match (gate(node), node.size) {
        (Gate::Px(px), _) | (_, Some(Size::Px(px))) => Some(px),
        _ => None,
    };
    let weight = |node: &Node| match node.size {
        Some(Size::Weight(weight)) => weight.max(0.0),
        _ => 1.0,
    };
    let taken: f32 = (children.iter())
        .filter_map(|node| fixed(node).map(|px| clamp(node, px)))
        .sum();
    let weights: f32 = (children.iter())
        .filter(|node| fixed(node).is_none())
        .map(|node| weight(node))
        .sum();
    let left = (room - taken).max(0.0);
    (children.iter())
        .map(|node| match fixed(node) {
            Some(px) => clamp(node, px),
            None if weights > 0.0 => clamp(node, left * weight(node) / weights),
            None => 0.0,
        })
        .collect()
}
