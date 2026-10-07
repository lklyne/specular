//! Writing one [`Entity`] as a `.canvas` node.

use serde_json::Value;

use super::CanvasError;
use super::fields::{fill, put, set};
use crate::{Color, ColorPreset, Entity, JsonMap, Kind};

/// Writes an entity as a node. Fields are written in the order the Electron
/// writer uses, which is the order they land in the file: `serde_json` keeps
/// insertion order (its `preserve_order` feature).
pub(super) fn write_node(entity: &Entity) -> Result<JsonMap, CanvasError> {
    let mut node = JsonMap::new();
    let mut ext = JsonMap::new();
    set(&mut node, "id", &entity.id)?;
    set(&mut node, "type", &node_type(&entity.kind))?;
    set(&mut node, "x", &entity.rect.x)?;
    set(&mut node, "y", &entity.rect.y)?;
    set(&mut node, "width", &entity.rect.width)?;
    set(&mut node, "height", &entity.rect.height)?;

    let label = entity.label.as_ref();
    let parent = entity.parent.as_ref();
    let anchor = entity.anchor.as_ref();
    match &entity.kind {
        Kind::Page(page) => {
            set(&mut node, "url", &page.url)?;
            put(&mut node, "presetIndex", page.preset_index.as_ref())?;
            put(&mut node, "syncId", page.sync_id.as_ref())?;
            put(&mut node, "label", label)?;
            put(&mut node, "source", page.source.as_ref())?;
            // `groupId` is the older name for the same reference; both are
            // written so either reader finds the group.
            put(&mut node, "groupId", parent)?;
            put(&mut node, "parentGroupId", parent)?;
            put(&mut node, "metadata", page.metadata.as_ref())?;
            put(&mut node, "colorScheme", page.color_scheme.as_ref())?;
        }
        Kind::Text(text) => {
            set(&mut node, "text", &text.text)?;
            put_node_color(&mut node, text.color.as_ref())?;
            put(&mut ext, "textStyle", text.style.as_ref())?;
            put(&mut ext, "widthMode", text.width_mode.as_ref())?;
            put_color_role(&mut ext, text.color.as_ref());
            put(&mut ext, "textSize", text.size.as_ref())?;
            put(&mut ext, "textFont", text.font.as_ref())?;
            put(&mut ext, "pageAnchor", anchor)?;
            put(&mut ext, "parentGroupId", parent)?;
            put(&mut ext, "label", label)?;
        }
        Kind::File(file) => {
            set(&mut node, "file", &file.file)?;
            put(&mut node, "subpath", file.subpath.as_ref())?;
            put(&mut node, "objectFit", file.object_fit.as_ref())?;
            put(&mut node, "presetIndex", file.preset_index.as_ref())?;
            put(&mut node, "metadata", file.metadata.as_ref())?;
            put(&mut ext, "parentGroupId", parent)?;
            put(&mut ext, "label", label)?;
        }
        Kind::Group(group) => {
            put(&mut node, "label", label)?;
            put(&mut node, "color", group.color.as_ref())?;
            put(&mut node, "layoutMode", group.layout_mode.as_ref())?;
            put(&mut node, "layoutGap", group.layout_gap.as_ref())?;
            put(&mut node, "parentGroupId", parent)?;
            put(&mut node, "managedLayout", group.managed_layout.as_ref())?;
            put(&mut node, "groupColor", group.color.as_ref())?;
            put(&mut node, "sourceTaskId", group.source_task_id.as_ref())?;
            put(&mut node, "groupMetadata", group.metadata.as_ref())?;
        }
        Kind::Drawing(drawing) => {
            set(&mut node, "strokes", &drawing.strokes)?;
            put(&mut node, "label", label)?;
            put(&mut node, "parentGroupId", parent)?;
            put(&mut node, "pageAnchor", anchor)?;
        }
        Kind::Shape(shape) => {
            set(&mut node, "shapeKind", &shape.shape)?;
            set(&mut node, "text", &shape.text)?;
            put_node_color(&mut node, shape.color.as_ref())?;
            put(&mut node, "strokeWidth", shape.stroke_width.as_ref())?;
            put(&mut node, "borderStyle", shape.border_style.as_ref())?;
            put(&mut node, "borderColor", shape.border_color.as_ref())?;
            put(&mut node, "theme", shape.theme.as_ref())?;
            put(&mut node, "label", label)?;
            put(&mut node, "parentGroupId", parent)?;
            put(&mut node, "pageAnchor", anchor)?;
            put_color_role(&mut ext, shape.color.as_ref());
            put(&mut ext, "textSize", shape.text_size.as_ref())?;
            put(&mut ext, "fillStyle", shape.fill_style.as_ref())?;
            put(&mut ext, "textAlign", shape.text_align.as_ref())?;
            put(
                &mut ext,
                "textVerticalAlign",
                shape.text_vertical_align.as_ref(),
            )?;
        }
    }

    if let Some(Value::Object(leftover)) = entity.extra.get("specular") {
        fill(&mut ext, leftover);
    }
    if !ext.is_empty() {
        node.insert("specular".to_owned(), Value::Object(ext));
    }
    fill(&mut node, &entity.extra);
    Ok(node)
}

const fn node_type(kind: &Kind) -> &'static str {
    match kind {
        Kind::Page(_) => "link",
        Kind::Text(_) => "text",
        Kind::File(_) => "file",
        Kind::Group(_) => "group",
        Kind::Drawing(_) => "drawing",
        Kind::Shape(_) => "shape",
    }
}

/// See `take_node_color` in the reader for the neutral's two-field form.
fn put_node_color(node: &mut JsonMap, color: Option<&Color>) -> Result<(), CanvasError> {
    match color {
        Some(Color::Neutral) => set(node, "color", &Color::Preset(ColorPreset::Red)),
        Some(Color::Preset(_) | Color::Custom(_)) | None => put(node, "color", color),
    }
}

fn put_color_role(ext: &mut JsonMap, color: Option<&Color>) {
    if color == Some(&Color::Neutral) {
        ext.insert("colorRole".to_owned(), Value::from("neutral"));
    }
}
