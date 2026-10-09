//! Writing one [`Entity`] as a `.canvas` node.

use serde::Serialize;
use serde_json::Value;

use super::CanvasError;
use super::fields::{fill, set};
use crate::{Color, ColorPreset, Entity, JsonMap, Kind};

/// One JSON object being written, with the leftovers that belong in it.
struct Object<'a> {
    map: JsonMap,
    extra: Option<&'a JsonMap>,
}

impl<'a> Object<'a> {
    fn new(extra: Option<&'a JsonMap>) -> Self {
        Self {
            map: JsonMap::new(),
            extra,
        }
    }

    /// Writes a required field.
    fn set<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), CanvasError> {
        set(&mut self.map, key, value)
    }

    /// Writes an optional field. When it is absent, a leftover of the same
    /// name (a `null`, a value the typed field could not hold) is written
    /// here instead, so it keeps the place the Electron writer gave it.
    fn put<T: Serialize>(&mut self, key: &str, value: Option<&T>) -> Result<(), CanvasError> {
        match value {
            Some(value) => self.set(key, value)?,
            None => {
                if let Some(leftover) = self.extra.and_then(|extra| extra.get(key)) {
                    self.map.insert(key.to_owned(), leftover.clone());
                }
            }
        }
        Ok(())
    }

    /// The object, with the leftovers no field claimed after the fields.
    fn finish(mut self) -> JsonMap {
        if let Some(extra) = self.extra {
            fill(&mut self.map, extra);
        }
        self.map
    }
}

/// Writes an entity as a node. Fields are written in the order the Electron
/// writer uses, which is the order they land in the file: `serde_json` keeps
/// insertion order (its `preserve_order` feature).
pub(super) fn write_node(entity: &Entity) -> Result<JsonMap, CanvasError> {
    let mut node = Object::new(Some(&entity.extra));
    let mut ext = Object::new(entity.extra.get("specular").and_then(Value::as_object));
    node.set("id", &entity.id)?;
    node.set("type", &node_type(&entity.kind))?;
    // A framed page is placed by its shell's corner, as the reader undoes.
    let shell = match &entity.kind {
        Kind::Page(page) => page.shell(),
        _ => None,
    };
    let (left, top) = shell.map_or((0.0, 0.0), |shell| (shell.insets.left, shell.insets.top));
    node.set("x", &(entity.rect.x - left))?;
    node.set("y", &(entity.rect.y - top))?;
    node.set("width", &entity.rect.width)?;
    node.set("height", &entity.rect.height)?;

    let label = entity.label.as_ref();
    let parent = entity.parent.as_ref();
    let anchor = entity.anchor.as_ref();
    match &entity.kind {
        Kind::Page(page) => {
            node.set("url", &page.url)?;
            node.put("presetIndex", page.preset_index.as_ref())?;
            node.put("syncId", page.sync_id.as_ref())?;
            node.put("label", label)?;
            node.put("source", page.source.as_ref())?;
            // `groupId` is the older name for the same reference; both are
            // written so either reader finds the group.
            node.put("groupId", parent)?;
            node.put("parentGroupId", parent)?;
            node.put("metadata", page.metadata.as_ref())?;
            node.put("colorScheme", page.color_scheme.as_ref())?;
        }
        Kind::Text(text) => {
            node.set("text", &text.text)?;
            put_node_color(&mut node, text.color.as_ref())?;
            ext.put("textStyle", text.style.as_ref())?;
            ext.put("widthMode", text.width_mode.as_ref())?;
            put_color_role(&mut ext, text.color.as_ref())?;
            ext.put("textSize", text.size.as_ref())?;
            ext.put("textFont", text.font.as_ref())?;
            ext.put("pageAnchor", anchor)?;
            ext.put("parentGroupId", parent)?;
            ext.put("label", label)?;
        }
        Kind::File(file) => {
            node.set("file", &file.file)?;
            node.put("subpath", file.subpath.as_ref())?;
            node.put("objectFit", file.object_fit.as_ref())?;
            node.put("presetIndex", file.preset_index.as_ref())?;
            node.put("metadata", file.metadata.as_ref())?;
            ext.put("parentGroupId", parent)?;
            ext.put("label", label)?;
        }
        Kind::Group(group) => {
            node.put("label", label)?;
            node.put("color", group.color.as_ref())?;
            node.put("layoutMode", group.layout_mode.as_ref())?;
            node.put("layoutGap", group.layout_gap.as_ref())?;
            node.put("parentGroupId", parent)?;
            node.put("managedLayout", group.managed_layout.as_ref())?;
            node.put("groupColor", group.color.as_ref())?;
            node.put("sourceTaskId", group.source_task_id.as_ref())?;
            node.put("groupMetadata", group.metadata.as_ref())?;
        }
        Kind::Drawing(drawing) => {
            node.set("strokes", &drawing.strokes)?;
            node.put("label", label)?;
            node.put("parentGroupId", parent)?;
            node.put("pageAnchor", anchor)?;
        }
        Kind::Shape(shape) => {
            node.set("shapeKind", &shape.shape)?;
            node.set("text", &shape.text)?;
            put_node_color(&mut node, shape.color.as_ref())?;
            node.put("strokeWidth", shape.stroke_width.as_ref())?;
            node.put("borderStyle", shape.border_style.as_ref())?;
            node.put("borderColor", shape.border_color.as_ref())?;
            node.put("theme", shape.theme.as_ref())?;
            node.put("label", label)?;
            node.put("parentGroupId", parent)?;
            node.put("pageAnchor", anchor)?;
            put_color_role(&mut ext, shape.color.as_ref())?;
            ext.put("textSize", shape.text_size.as_ref())?;
            ext.put("fillStyle", shape.fill_style.as_ref())?;
            ext.put("textAlign", shape.text_align.as_ref())?;
            ext.put("textVerticalAlign", shape.text_vertical_align.as_ref())?;
        }
    }

    let ext = ext.finish();
    if !ext.is_empty() {
        node.map.insert("specular".to_owned(), Value::Object(ext));
    }
    Ok(node.finish())
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
fn put_node_color(node: &mut Object<'_>, color: Option<&Color>) -> Result<(), CanvasError> {
    match color {
        Some(Color::Neutral) => node.set("color", &Color::Preset(ColorPreset::Red)),
        Some(Color::Preset(_) | Color::Custom(_)) | None => node.put("color", color),
    }
}

fn put_color_role(ext: &mut Object<'_>, color: Option<&Color>) -> Result<(), CanvasError> {
    ext.put(
        "colorRole",
        (color == Some(&Color::Neutral)).then_some(&"neutral"),
    )
}
