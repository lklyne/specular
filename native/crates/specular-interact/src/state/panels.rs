//! The sidebar and the canvas as keys (ADR 0059): what a mod needs to draw
//! them again. A row or a control is written with the id `control.press
//! {id}` presses. No rect is in any of them, so a drag moves no key.

use serde_json::{Value, json};
use specular_doc::{EntityId, ItemId, Kind};

use crate::{App, CanvasRow, ProjectSection, SectionHead, SidebarModel, SidebarRow, sidebar};

fn head(head: &SectionHead) -> Value {
    json!({ "id": head.id, "title": head.title, "folded": head.folded })
}

fn row(row: &SidebarRow) -> Value {
    json!({
        "id": row.id,
        "label": row.label,
        "icon": row.glyph,
        "trailing": row.trailing,
        "selected": row.selected,
        "dimmed": row.dimmed,
        "expanded": row.expanded,
        "children": row.children.iter().map(self::row).collect::<Vec<_>>(),
    })
}

fn section(kind: &str, head: &SectionHead, rows: &[SidebarRow]) -> Value {
    let mut out = self::head(head);
    out["kind"] = Value::from(kind);
    out["rows"] = rows.iter().map(row).collect();
    out
}

fn canvas_rows(canvases: &[CanvasRow]) -> Value {
    (canvases.iter())
        .map(|canvas| {
            json!({
                "id": canvas.control,
                "canvas": canvas.id.as_str(),
                "project": canvas.project.as_str(),
                "label": canvas.label,
                "active": canvas.active,
                "count": canvas.entity_count,
                "rename": canvas.rename.id,
            })
        })
        .collect()
}

/// A project as a section of the sidebar (ADR 0062): its head is the
/// section's, its canvases its rows, and its details' controls by id.
fn project(section: &ProjectSection) -> Value {
    let head = &section.control;
    let git = match section.git {
        None => Value::Null,
        Some(None) => {
            json!({ "choice": null, "commit": head.child("git").child("commit"), "ignore": head.child("git").child("ignore") })
        }
        Some(Some(choice)) => {
            json!({ "choice": choice.as_str(), "commit": head.child("git").child("commit"), "ignore": head.child("git").child("ignore") })
        }
    };
    let origins: Vec<Value> = (section.origins.iter().enumerate())
        .map(|(index, origin)| {
            let id = head.child("origin").child(index + 1);
            json!({
                "origin": origin.origin,
                "autoFix": origin.auto_fix,
                "auto": id.child("auto"),
                "remove": id.child("remove"),
            })
        })
        .collect();
    json!({
        "id": head,
        "kind": "project",
        "project": section.id.as_str(),
        "title": section.label,
        "folder": section.folder,
        "repo": section.repo,
        "branch": section.branch,
        "active": section.active,
        "folded": section.folded,
        "add": head.child("add"),
        "details": head.child("details"),
        "detailsShown": section.details,
        "reveal": head.child("reveal"),
        "close": section.close.as_ref().map(|_| head.child("close")),
        "git": git,
        "origins": origins,
        "rows": if section.folded { json!([]) } else { canvas_rows(&section.canvases) },
    })
}

/// The `panels/sidebar` key: a section for each project, then the active
/// canvas's notes and pages. Canvases in no folder are one Canvases
/// section.
pub(super) fn sidebar_json(app: &App) -> Value {
    let SidebarModel {
        visible,
        canvases_head,
        canvases,
        projects,
        notes_head,
        notes,
        pages_head,
        pages,
        title,
        ..
    } = sidebar(app);
    let mut sections: Vec<Value> = if projects.is_empty() {
        let mut canvas_section = head(&canvases_head);
        canvas_section["rows"] = canvas_rows(&canvases);
        vec![canvas_section]
    } else {
        projects.iter().map(project).collect()
    };
    sections.push(section("notes", &notes_head, &notes));
    sections.push(section("pages", &pages_head, &pages));
    json!({
        "visible": visible,
        "add": "sidebar.add",
        "open": "sidebar.open",
        "title": serde_json::to_value(&title).unwrap_or(Value::Null),
        "sections": sections,
    })
}

/// The `selection` key: the entities and the edges selected.
pub(super) fn selection_json(app: &App) -> Value {
    let (mut ids, mut edges) = (Vec::new(), Vec::new());
    for item in app.session.selection.items() {
        match item {
            ItemId::Entity(id) => ids.push(id.as_str().to_owned()),
            ItemId::Edge(id) => edges.push(id.as_str().to_owned()),
        }
    }
    json!({ "ids": ids, "edges": edges })
}

/// What an item says of itself beyond its kind: a page's address, a
/// text's or a shape's words and form, a mod item's mod.
fn detail(kind: &Kind) -> Value {
    match kind {
        Kind::Page(page) => json!({ "url": page.url }),
        Kind::Text(text) => json!({ "text": text.text }),
        Kind::Shape(shape) => json!({
            "shape": shape.shape,
            "text": shape.text,
            "custom": shape.custom.as_ref().map(|custom| json!({
                "shape": custom.name,
                "params": custom.params,
            })),
        }),
        Kind::Mod(item) => json!({ "mod": item.name }),
        Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => json!({}),
    }
}

/// The `canvas/items` key: every item of the active canvas, back to
/// front, without its place or size.
pub(super) fn items_json(app: &App) -> Value {
    let items = app.document.entities().map(|entity| {
        let mut item = json!({
            "id": entity.id.as_str(),
            "kind": entity.kind.name(),
            "label": entity.label,
            "parent": entity.parent.as_ref().map(EntityId::as_str),
        });
        if let (Value::Object(item), Value::Object(detail)) = (&mut item, detail(&entity.kind)) {
            item.extend(detail);
        }
        item
    });
    Value::Array(items.collect())
}

/// The `projects` key (ADR 0062): every project open, in the sidebar's
/// order, without controls.
pub(super) fn projects_json(app: &App) -> Value {
    let active = &app.space.active().project;
    (app.space.projects().iter())
        .map(|project| {
            let canvases: Vec<Value> = (app.space.canvases_of(&project.id))
                .map(|canvas| json!({ "id": canvas.id.as_str(), "name": canvas.name, "active": canvas.is_active() }))
                .collect();
            json!({
                "id": project.id.as_str(),
                "label": project.label,
                "folder": project.folder,
                "canvasFolder": project.canvases,
                "repo": project.repo,
                "branch": project.branch,
                "git": project.git.map(crate::GitChoice::as_str),
                "active": project.id == *active,
                "canvases": canvases,
            })
        })
        .collect()
}

/// The `canvas/info` key: the canvas showing, and every canvas of the space.
pub(super) fn info_json(app: &App) -> Value {
    let active = app.space.active();
    let canvases: Vec<Value> = (app.space.canvases().iter())
        .map(|canvas| json!({ "id": canvas.id.as_str(), "name": canvas.name, "project": canvas.project.as_str(), "active": canvas.is_active() }))
        .collect();
    json!({
        "id": active.id.as_str(),
        "name": active.name,
        "project": app.space.active_project().map(|project| project.id.as_str()),
        "items": app.document.entities().count(),
        "edges": app.document.edges().count(),
        "canvases": canvases,
    })
}
