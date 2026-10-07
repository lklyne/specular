//! Reading one `.canvas` node into an [`Entity`].

use serde_json::Value;

use super::fields::take;
use crate::{
    Color, Drawing, Entity, EntityId, FileRef, Group, JsonMap, Kind, Page, Rect, Shape, Text,
};

/// Reads a node. A node this crate cannot type (an unknown `type`, or a
/// required field missing or malformed) comes back untouched as the error,
/// so the caller can keep it as raw JSON.
///
/// The fields no kind claims become the entity's `extra`.
pub(super) fn read_node(mut node: JsonMap) -> Result<Entity, JsonMap> {
    let Some(kind) = read_required(&mut node) else {
        return Err(node);
    };
    let (Some(id), Some(x), Some(y), Some(width), Some(height)) = (
        take::<String>(&mut node, "id"),
        take(&mut node, "x"),
        take(&mut node, "y"),
        take(&mut node, "width"),
        take(&mut node, "height"),
    ) else {
        return Err(node);
    };
    node.shift_remove("type");

    let mut entity = Entity::new(id, Rect::new(x, y, width, height), kind);
    // `text` and `file` are JSON Canvas node types, so their Specular fields
    // are namespaced under `specular`. The kinds Specular defines carry
    // theirs at the top level.
    let mut ext: JsonMap = take(&mut node, "specular").unwrap_or_default();
    match &mut entity.kind {
        Kind::Page(page) => {
            page.preset_index = take(&mut node, "presetIndex");
            page.sync_id = take(&mut node, "syncId");
            entity.label = take(&mut node, "label");
            page.source = take(&mut node, "source");
            let legacy: Option<EntityId> = take(&mut node, "groupId");
            entity.parent = take(&mut node, "parentGroupId").or(legacy);
            page.metadata = take(&mut node, "metadata");
            page.color_scheme = take(&mut node, "colorScheme");
        }
        Kind::Text(text) => {
            text.color = take_node_color(&mut node, &mut ext);
            text.style = take(&mut ext, "textStyle");
            text.width_mode = take(&mut ext, "widthMode");
            text.size = take(&mut ext, "textSize");
            text.font = take(&mut ext, "textFont");
            entity.anchor = take(&mut ext, "pageAnchor");
            entity.parent = take(&mut ext, "parentGroupId");
            entity.label = take(&mut ext, "label");
        }
        Kind::File(file) => {
            file.subpath = take(&mut node, "subpath");
            file.object_fit = take(&mut node, "objectFit");
            file.preset_index = take(&mut node, "presetIndex");
            file.metadata = take(&mut node, "metadata");
            entity.parent = take(&mut ext, "parentGroupId");
            entity.label = take(&mut ext, "label");
        }
        Kind::Group(group) => {
            entity.label = take(&mut node, "label");
            let color: Option<Color> = take(&mut node, "color");
            group.color = take(&mut node, "groupColor").or(color);
            group.layout_mode = take(&mut node, "layoutMode");
            group.layout_gap = take(&mut node, "layoutGap");
            entity.parent = take(&mut node, "parentGroupId");
            group.managed_layout = take(&mut node, "managedLayout");
            group.source_task_id = take(&mut node, "sourceTaskId");
            group.metadata = take(&mut node, "groupMetadata");
            // Legacy copies of the membership each member's `parent` holds.
            node.shift_remove("pageIds");
            node.shift_remove("entityIds");
        }
        Kind::Drawing(_) => {
            entity.label = take(&mut node, "label");
            entity.parent = take(&mut node, "parentGroupId");
            entity.anchor = take(&mut node, "pageAnchor");
        }
        Kind::Shape(shape) => {
            shape.text = take(&mut node, "text").unwrap_or_default();
            shape.color = take_node_color(&mut node, &mut ext);
            shape.stroke_width = take(&mut node, "strokeWidth");
            shape.border_style = take(&mut node, "borderStyle");
            shape.border_color = take(&mut node, "borderColor");
            shape.theme = take(&mut node, "theme");
            entity.label = take(&mut node, "label");
            entity.parent = take(&mut node, "parentGroupId");
            entity.anchor = take(&mut node, "pageAnchor");
            shape.text_size = take(&mut ext, "textSize");
            shape.fill_style = take(&mut ext, "fillStyle");
            shape.text_align = take(&mut ext, "textAlign");
            shape.text_vertical_align = take(&mut ext, "textVerticalAlign");
        }
    }
    if !ext.is_empty() {
        node.insert("specular".to_owned(), Value::Object(ext));
    }
    entity.extra = node;
    Ok(entity)
}

/// Checks the fields every node needs, then takes the one its kind needs.
/// That take is the last step that can refuse the node, so a refused node
/// has lost nothing.
fn read_required(node: &mut JsonMap) -> Option<Kind> {
    let has_base = node.get("id").is_some_and(Value::is_string)
        && ["x", "y", "width", "height"]
            .iter()
            .all(|key| node.get(*key).is_some_and(Value::is_number));
    if !has_base {
        return None;
    }
    Some(match node.get("type")?.as_str()? {
        "link" => Kind::Page(Page {
            url: take(node, "url")?,
            ..Page::default()
        }),
        "text" => Kind::Text(Text {
            text: take(node, "text")?,
            ..Text::default()
        }),
        "file" => Kind::File(FileRef {
            file: take(node, "file")?,
            ..FileRef::default()
        }),
        "group" => Kind::Group(Group::default()),
        "drawing" => Kind::Drawing(Drawing {
            strokes: take(node, "strokes")?,
        }),
        "shape" => Kind::Shape(Shape::new(take(node, "shapeKind")?)),
        _ => return None,
    })
}

/// A text or shape node's color. The neutral is stored as the red preset
/// plus `specular.colorRole`, so tools that ignore `specular` still read a
/// valid color.
fn take_node_color(node: &mut JsonMap, ext: &mut JsonMap) -> Option<Color> {
    if ext.get("colorRole").and_then(Value::as_str) == Some("neutral") {
        ext.shift_remove("colorRole");
        node.shift_remove("color");
        return Some(Color::Neutral);
    }
    take(node, "color")
}
