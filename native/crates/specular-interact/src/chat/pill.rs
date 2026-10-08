//! The live pill: what this turn is about, from the app as it is now.
//!
//! A DOM node comes first, then the focused comment, then the canvas
//! selection, then nothing (the canvas name). The DOM node is the one the
//! inspect tool picked, which stays the target after the tool is put down.

use specular_agent::{
    CanvasSelection, FocusedAnnotation, InspectNode, Pill, PillInput, origin_of, resolve,
};
use specular_doc::{AnnotationAnchor, AnnotationId, Entity, ItemId, Kind};

use crate::App;

/// How many characters of a text name a selected item.
const WORDS: usize = 40;

/// The pill for the app as it is.
pub(super) fn live(app: &App) -> Pill {
    resolve(&PillInput {
        inspect_node: inspected(app),
        focused_annotation: app
            .session
            .focused_comment
            .as_ref()
            .and_then(|id| focused(app, id)),
        canvas_selection: selection(app),
    })
}

fn inspected(app: &App) -> Option<InspectNode> {
    let target = app.inspected()?;
    let address = crate::inspect::address(app, &target.page);
    Some(InspectNode {
        name: target.node.name.clone(),
        tag_name: target.node.tag_name.clone(),
        origin: address.as_deref().and_then(origin_of),
        page_id: Some(target.page.as_str().to_owned()),
    })
}

/// The pill for a turn aimed at the comment `id`, or `None` when the
/// document no longer holds it.
pub(super) fn of_comment(app: &App, id: &str) -> Option<Pill> {
    let focused = focused(app, &AnnotationId::new(id))?;
    Some(resolve(&PillInput {
        focused_annotation: Some(focused),
        ..PillInput::default()
    }))
}

fn focused(app: &App, id: &AnnotationId) -> Option<FocusedAnnotation> {
    let annotation = app.document.annotation(id)?;
    let anchor_type = match &annotation.anchor {
        AnnotationAnchor::Element { .. } => "element",
        AnnotationAnchor::Region(_) => "region",
        AnnotationAnchor::Page { .. } => "page",
        AnnotationAnchor::Canvas { .. } => "canvas",
    };
    Some(FocusedAnnotation {
        id: id.as_str().to_owned(),
        text: annotation.text.clone(),
        element_name: annotation.element_name.clone(),
        anchor_type: anchor_type.to_owned(),
    })
}

fn selection(app: &App) -> Option<CanvasSelection> {
    let items = app.session.selection.items();
    let entity_ids: Vec<String> = app
        .session
        .selection
        .entities()
        .map(|id| id.as_str().to_owned())
        .collect();
    let label = match items {
        [] => return None,
        [ItemId::Entity(id)] => entity_label(app, app.document.entity(id)?),
        [ItemId::Edge(_)] => "edge".to_owned(),
        many => format!("{} items", many.len()),
    };
    Some(CanvasSelection {
        count: items.len(),
        label,
        entity_ids,
    })
}

/// What the chip calls one selected entity: a page by its live title, a file
/// by its file name, a group by its label, a text or shape by its opening
/// words, and anything else, or anything nameless, by its kind.
fn entity_label(app: &App, entity: &Entity) -> String {
    match &entity.kind {
        Kind::Page(_) => {
            let title = app.page_state(&entity.id).map(|state| state.title.trim());
            title.filter(|t| !t.is_empty()).unwrap_or("page").to_owned()
        }
        Kind::File(file) => {
            let name = file.file.rsplit('/').next().unwrap_or(&file.file);
            name.to_owned()
        }
        Kind::Group(_) => named(entity.label.as_deref().unwrap_or(""), "group"),
        Kind::Text(text) => first_words(&text.text, "text"),
        Kind::Shape(shape) => first_words(&shape.text, "shape"),
        Kind::Drawing(_) => "drawing".to_owned(),
    }
}

fn named(label: &str, kind: &str) -> String {
    let label = label.trim();
    if label.is_empty() { kind } else { label }.to_owned()
}

fn first_words(text: &str, kind: &str) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.is_empty() {
        return kind.to_owned();
    }
    if words.chars().count() <= WORDS {
        return words;
    }
    let mut cut: String = words.chars().take(WORDS - 1).collect();
    cut.push('\u{2026}');
    cut
}
