//! The cursor on the canvas verbs: an agent's API call that changes the
//! canvas sends its cursor to what changed, one stop at a time, and the
//! CLI's presence report on a read sweeps it over what is in view.
//!
//! The cursor sets off as the change lands, from a comparison of the
//! document before and after, so every route that edits the canvas shows
//! its agent without a word about presence in the route, and it never
//! heads for somewhere a refused change would not have been. Electron's
//! canvas routes did the same. ADR 0029's hold is for live pages, where an
//! act would otherwise outrun its cursor.

use std::collections::VecDeque;

use glam::DVec2;
use specular_doc::{
    Annotation, AnnotationAnchor, Document, Edge, Entity, EntityId, ItemId, Kind, Rect,
};

use super::cursor::{Anchor, Cursor, FADE_MS, IDLE_MS, Stop, THINKING_MS, travel_ms};
use crate::showing::Showing;
use crate::{Action, ApiRun, App, Effect};

/// How long the cursor rests on a stop before it goes on to the next.
const STOP_DWELL_MS: u64 = 160;
/// The rest on an item while reading: a glance.
const READ_DWELL_MS: u64 = 30;
/// The most stops one act visits: enough to show a spread, few enough that
/// an arrange of fifty items does not keep the cursor busy for seconds.
const MAX_STOPS: usize = 5;
/// The most items a read visits, spread over everything in view, so a full
/// canvas reads as a scan of all of it in about two seconds.
const MAX_READ_STOPS: usize = 24;

/// How long the cursor rests on a stop with this label.
pub(super) fn dwell_ms(label: &str) -> u64 {
    if label == "reading" {
        READ_DWELL_MS
    } else {
        STOP_DWELL_MS
    }
}
/// How far inside an item's corner the cursor points, in canvas units.
const CORNER_INSET: f64 = 24.0;
/// Where a cursor that was not on the canvas comes in from, in screen
/// pixels from its first stop.
const ENTRY: DVec2 = DVec2::new(-56.0, -40.0);

/// What the app looked like before an agent's call, to tell what it did.
#[derive(Debug)]
pub(crate) struct Watch {
    agent: String,
    /// The document before the call.
    pub(crate) before: Document,
    selection: Vec<ItemId>,
    hint: Option<(Vec<ItemId>, &'static str)>,
}

impl Watch {
    /// A watch over `run`, by `agent`. `None` for a call with nothing to
    /// show on the canvas.
    pub(crate) fn new(app: &App, agent: &str, run: &ApiRun) -> Option<Self> {
        let hint = match run {
            ApiRun::Act { on, action } => {
                let label = match action {
                    Action::SetCamera(_) | Action::Show(_) => "focusing",
                    _ => "selecting",
                };
                match (on, action) {
                    (Some(items), _) => Some((items.clone(), label)),
                    (None, Action::Show(Showing::Item(id))) => {
                        Some((vec![ItemId::Entity(id.clone())], label))
                    }
                    (None, _) => None,
                }
            }
            ApiRun::Navigate { page, .. } => {
                Some((vec![ItemId::Entity(page.clone())], "navigating"))
            }
            ApiRun::Apply { .. } | ApiRun::ApplyWithFiles { .. } => None,
            ApiRun::NewCanvas { .. }
            | ApiRun::Share { .. }
            | ApiRun::Unshare
            | ApiRun::Report { .. } => {
                return None;
            }
        };
        Some(Self {
            agent: agent.to_owned(),
            before: app.document.clone(),
            selection: app.session.selection.items().to_vec(),
            hint,
        })
    }
}

/// Sends the watched agent's cursor to what its call changed: the items
/// added, moved, edited or removed, then what it selected or focused.
pub(crate) fn after(app: &mut App, watch: &Watch, effects: &mut Vec<Effect>) {
    let mut stops = changes(app, &watch.before);
    if stops.is_empty()
        && let Some((items, label)) = &watch.hint
    {
        stops = item_stops(app, items, label);
    }
    let selection = app.session.selection.items();
    if stops.is_empty() && !selection.is_empty() && selection != watch.selection {
        stops = item_stops(app, selection, "selecting");
    }
    go(app, &watch.agent, stops, effects);
}

/// What the CLI reports with a canvas verb that reads, by the key Electron
/// named it with.
pub(crate) fn on_report(app: &mut App, agent: &str, key: &str, effects: &mut Vec<Effect>) {
    if key == "open" {
        return;
    }
    let now = app.session.now_ms;
    let centre = || {
        let view = view_rect(app);
        DVec2::new(view.x + view.width / 2.0, view.y + view.height / 2.0)
    };
    let stops = match key {
        "scan_workspace" | "read_content" => in_view(app, "reading"),
        "find_placement" => vec![Stop {
            at: centre(),
            label: "finding space",
            item: None,
        }],
        "add_annotation" => vec![Stop {
            at: centre(),
            label: "commenting",
            item: None,
        }],
        "thinking" | "idle" | "departing" => {
            let rest = if key == "thinking" {
                THINKING_MS
            } else {
                IDLE_MS - FADE_MS
            };
            if let Some(index) = app.session.presence.held(agent) {
                let cursor = &mut app.session.presence.cursors[index];
                cursor.stops.clear();
                cursor.acted_ms = Some(now.saturating_sub(rest));
            }
            return;
        }
        _ => return,
    };
    go(app, agent, stops, effects);
}

/// Starts `agent`'s cursor on `stops`, from where it is, or from just off
/// the first stop when it has no cursor on this canvas.
fn go(app: &mut App, agent: &str, stops: Vec<Stop>, effects: &mut Vec<Effect>) {
    let mut stops = VecDeque::from(stops);
    let Some(first) = stops.pop_front() else {
        return;
    };
    let canvas = app.space.active().id.clone();
    let anchor = Anchor::Canvas {
        canvas,
        at: first.at,
    };
    let held = (app.session.presence.held(agent))
        .map(|index| app.session.presence.cursors[index].clone())
        .filter(|cursor| cursor.target(app).is_some());
    let mut cursor = if let Some(mut cursor) = held {
        cursor.head_to(app, anchor, first.label);
        cursor
    } else {
        let zoom = f64::from(app.session.camera.zoom);
        let from = first.at + ENTRY / zoom;
        let mut cursor = Cursor::new(agent, anchor, from, first.label);
        cursor.travel_ms = travel_ms(app, from, first.at, first.label);
        cursor
    };
    cursor.item = first.item;
    cursor.stops = stops;
    cursor.ripples = first.label != "reading";
    cursor.acted_ms = None;
    app.session.presence.put(cursor, effects);
}

/// The stops for what changed between `before` and the app's document, in
/// reading order.
fn changes(app: &App, before: &Document) -> Vec<Stop> {
    let after = &app.document;
    let mut found: Vec<(Stop, bool)> = Vec::new();
    for entity in after.entities() {
        let label = match before.entity(&entity.id) {
            None => "adding",
            Some(old) if old == entity => continue,
            Some(old) if old.rect == entity.rect => "editing",
            Some(old)
                if (old.rect.width, old.rect.height) == (entity.rect.width, entity.rect.height) =>
            {
                "moving"
            }
            Some(_) => "resizing",
        };
        let rect = crate::shown_rect(app, entity).unwrap_or(entity.rect);
        found.push((stop(rect, label), is_group(entity)));
    }
    for entity in before.entities() {
        if after.entity(&entity.id).is_none() {
            found.push((stop(entity.rect, "deleting"), is_group(entity)));
        }
    }
    for edge in after.edges() {
        let label = match before.edge(&edge.id) {
            None => "connecting",
            Some(old) if old == edge => continue,
            Some(_) => "editing",
        };
        found.extend(edge_point(after, edge).map(|at| {
            (
                Stop {
                    at,
                    label,
                    item: None,
                },
                false,
            )
        }));
    }
    for edge in before.edges() {
        if after.edge(&edge.id).is_none() {
            let at = edge_point(after, edge).or_else(|| edge_point(before, edge));
            found.extend(at.map(|at| {
                (
                    Stop {
                        at,
                        label: "deleting",
                        item: None,
                    },
                    false,
                )
            }));
        }
    }
    for annotation in after.annotations() {
        if before.annotation(&annotation.id) != Some(annotation) {
            let at = annotation_point(app, after, annotation);
            found.extend(at.map(|at| {
                (
                    Stop {
                        at,
                        label: "commenting",
                        item: None,
                    },
                    false,
                )
            }));
        }
    }
    for annotation in before.annotations() {
        if after.annotation(&annotation.id).is_none() {
            let at = annotation_point(app, before, annotation);
            found.extend(at.map(|at| {
                (
                    Stop {
                        at,
                        label: "deleting",
                        item: None,
                    },
                    false,
                )
            }));
        }
    }
    // A group refits around what moved in it, so it is shown only when it
    // is all that changed.
    let only_groups = found.iter().all(|(_, group)| *group);
    let stops = (found.into_iter())
        .filter(|(_, group)| only_groups || !group)
        .map(|(stop, _)| stop)
        .collect();
    in_reading_order(stops)
}

fn is_group(entity: &Entity) -> bool {
    matches!(entity.kind, Kind::Group(_))
}

/// A stop just inside `rect`'s top-left corner, where a hand would take
/// hold of it.
fn stop(rect: Rect, label: &'static str) -> Stop {
    Stop {
        at: DVec2::new(
            rect.x + (rect.width / 2.0).min(CORNER_INSET),
            rect.y + (rect.height / 2.0).min(CORNER_INSET),
        ),
        label,
        item: None,
    }
}

fn item_stops(app: &App, items: &[ItemId], label: &'static str) -> Vec<Stop> {
    let stops = (items.iter())
        .filter_map(|item| match item {
            ItemId::Entity(id) => entity_rect(app, id).map(|rect| stop(rect, label)),
            ItemId::Edge(id) => (app.document.edge(id))
                .and_then(|edge| edge_point(&app.document, edge))
                .map(|at| Stop {
                    at,
                    label,
                    item: None,
                }),
        })
        .collect();
    in_reading_order(stops)
}

fn entity_rect(app: &App, id: &EntityId) -> Option<Rect> {
    let entity = app.document.entity(id)?;
    Some(crate::shown_rect(app, entity).unwrap_or(entity.rect))
}

/// The middle of the line between an edge's ends.
fn edge_point(document: &Document, edge: &Edge) -> Option<DVec2> {
    let centre = |id: &EntityId| {
        let rect = document.entity(id)?.rect;
        Some(DVec2::new(
            rect.x + rect.width / 2.0,
            rect.y + rect.height / 2.0,
        ))
    };
    Some(centre(&edge.from)?.midpoint(centre(&edge.to)?))
}

/// Where an annotation's pin sits on the canvas.
fn annotation_point(app: &App, document: &Document, annotation: &Annotation) -> Option<DVec2> {
    let page_corner = |page: &EntityId, offset: DVec2| {
        let rect = document.entity(page)?.rect;
        Some(DVec2::new(rect.x, rect.y) + offset)
    };
    match &annotation.anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => Some(DVec2::new(*canvas_x, *canvas_y)),
        AnnotationAnchor::Page {
            page_id,
            offset_x,
            offset_y,
        } => page_corner(page_id, DVec2::new(*offset_x, *offset_y)),
        AnnotationAnchor::Element { page_id, .. } => crate::element_on_canvas(app, annotation)
            .map(|rect| DVec2::new(rect.x, rect.y))
            .or_else(|| page_corner(page_id, DVec2::ZERO)),
        AnnotationAnchor::Region(_) => {
            crate::region_on_canvas(app, annotation).map(|rect| DVec2::new(rect.x, rect.y))
        }
    }
}

/// What a read sweeps over: the items in view, groups left out, each lit
/// as the cursor lands on it. A full view is sampled evenly, so the scan
/// crosses all of it rather than stopping at its top row.
fn in_view(app: &App, label: &'static str) -> Vec<Stop> {
    let view = view_rect(app);
    let mut stops: Vec<Stop> = (app.document.entities())
        .filter(|entity| !is_group(entity))
        .filter_map(|entity| crate::shown_rect(app, entity))
        .filter(|rect| overlaps(*rect, view))
        .map(|rect| Stop {
            item: Some(rect),
            ..stop(rect, label)
        })
        .collect();
    stops.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
    let count = stops.len();
    if count <= MAX_READ_STOPS {
        return stops;
    }
    (0..MAX_READ_STOPS)
        .map(|step| stops[step * (count - 1) / (MAX_READ_STOPS - 1)].clone())
        .collect()
}

/// Top to bottom, then left to right, close stops merged, at most
/// [`MAX_STOPS`].
fn in_reading_order(mut stops: Vec<Stop>) -> Vec<Stop> {
    stops.sort_by(|a, b| a.at.y.total_cmp(&b.at.y).then(a.at.x.total_cmp(&b.at.x)));
    stops.dedup_by(|next, kept| (next.at - kept.at).length() < CORNER_INSET);
    stops.truncate(MAX_STOPS);
    stops
}

/// The part of the canvas the window shows.
fn view_rect(app: &App) -> Rect {
    let camera = app.session.camera;
    let zoom = f64::from(camera.zoom);
    Rect::new(
        -f64::from(camera.pan.x) / zoom,
        -f64::from(camera.pan.y) / zoom,
        f64::from(app.session.viewport.x) / zoom,
        f64::from(app.session.viewport.y) / zoom,
    )
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}
