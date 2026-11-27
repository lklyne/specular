//! [`ModDraw`]: one entry of a mod item's display list (ADR 0057).
//!
//! A mod describes what its item looks like as a short list of these, in
//! item space: `(0, 0)` is the item's top-left corner and a unit is a
//! canvas unit, so the canvas pans, zooms and moves the item without asking
//! the mod. It is the vocabulary the scene already draws (rects, ellipses,
//! paths, text, images), sent as JSON. Colours are CSS hex strings
//! (`#rgb`, `#rrggbb`, `#rrggbbaa`). An image's `src` is a path in the space
//! folder or an `http(s)` URL.
//!
//! The same list is an item's saved snapshot, so the format is part of the
//! `.canvas` file.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ObjectFit;

/// One thing a mod item draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ModDraw {
    /// A rect, rounded by `radius`.
    Rect {
        /// Its box.
        #[serde(flatten)]
        frame: DrawBox,
        /// How it is filled and outlined.
        #[serde(flatten)]
        paint: Paint,
        /// Corner radius.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        radius: Option<f64>,
    },
    /// An ellipse filling its box.
    Ellipse {
        /// Its box.
        #[serde(flatten)]
        frame: DrawBox,
        /// How it is filled and outlined.
        #[serde(flatten)]
        paint: Paint,
    },
    /// A line through `points`, closed into a polygon when `closed`.
    Path {
        /// Its points, `[x, y]` each.
        points: Vec<[f64; 2]>,
        /// Whether the last point joins the first.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        closed: bool,
        /// How it is filled and outlined.
        #[serde(flatten)]
        paint: Paint,
    },
    /// A run of text, its top-left at `x`, `y`, wrapped at `width` when set.
    Text {
        /// Left edge.
        x: f64,
        /// Top edge.
        y: f64,
        /// What it says.
        text: String,
        /// Font size. Absent is 14.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<f64>,
        /// Text colour. Absent is the theme's text colour.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        /// Where it wraps.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<f64>,
        /// `left`, `center` or `right` within `width`. Absent is left.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        align: Option<String>,
    },
    /// A picture fitted to its box as CSS `object-fit` fits an `<img>`.
    Image {
        /// Its box.
        #[serde(flatten)]
        frame: DrawBox,
        /// A path in the space folder, or an `http(s)` URL.
        src: String,
        /// How it fits the box. Absent is `cover`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fit: Option<ObjectFit>,
        /// Corner radius.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        radius: Option<f64>,
    },
}

/// A box in item space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrawBox {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// How a shape is filled and outlined. Neither set draws nothing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Paint {
    /// Fill colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Outline colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    /// Outline width. Absent is 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<f64>,
}

impl ModDraw {
    /// Reads a display list sent as JSON, leaving out entries that are not
    /// draws, so one bad entry does not blank the item. `None` when `value`
    /// is not a list.
    pub fn list(value: &Value) -> Option<Vec<Self>> {
        let entries = value.as_array()?;
        Some(
            (entries.iter())
                .filter_map(|entry| Self::deserialize(entry).ok())
                .collect(),
        )
    }

    /// The image paths and URLs `draws` show, in order.
    pub fn images(draws: &[Self]) -> impl Iterator<Item = &str> {
        draws.iter().filter_map(|draw| match draw {
            Self::Image { src, .. } => Some(src.as_str()),
            Self::Rect { .. } | Self::Ellipse { .. } | Self::Path { .. } | Self::Text { .. } => {
                None
            }
        })
    }
}
