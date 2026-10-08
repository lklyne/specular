//! `POST /tasks/apply`: the `breakpoints` verb. One URL becomes a group of
//! pages, one per viewport preset, laid left to right in a free spot and
//! linked by `breakpoint_variant` edges. The whole cluster is one
//! [`Command::Batch`], so it is one undo step.

use serde_json::{Map, Value, json};
use specular_doc::{
    Command, Document, Edge, EdgeId, EdgeKind, Entity, EntityId, Group, ItemId, JsonMap, Kind,
    LayoutMode, Page, PageSource, Rect, VIEWPORT_PRESETS,
};
use specular_interact::ApiRun;

use crate::ids::Ids;
use crate::reply::Reply;
use crate::{Response, Step, placement};

/// The room between the cluster and its group's edge.
const GROUP_PADDING: f64 = 24.0;
/// The room between neighbouring pages in the row.
const GUTTER: f64 = 80.0;
/// The only task kind there is.
const BREAKPOINT_MAP: &str = "breakpoint_map";
/// The presets used when the request names none.
const DEFAULT_PRESETS: [&str; 3] = ["iPhone 14 Pro", "iPad Mini", "Desktop"];

/// Whether `host` is a machine on the way to a dev server, which defaults
/// to `http`.
fn is_local(host: &str) -> bool {
    let host = host.trim().to_lowercase();
    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect();
    let private = match octets[..] {
        [a, b, _, _] if host.split('.').count() == 4 => {
            a == 10
                || a == 127
                || (a == 192 && b == 168)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 169 && b == 254)
        }
        _ => false,
    };
    matches!(host.as_str(), "localhost" | "::1" | "[::1]")
        || private
        || host.rsplit_once('.').is_some_and(|(_, tld)| tld == "local")
}

fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once("://") else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// The host of a URL's authority, without credentials or port.
fn hostname(authority: &str) -> String {
    let host = authority.rsplit('@').next().unwrap_or_default();
    if host.starts_with('[') {
        let end = host.find(']').map_or(host.len(), |end| end + 1);
        return host[..end].to_lowercase();
    }
    host.split(':').next().unwrap_or_default().to_lowercase()
}

/// A URL as a user typed it, made whole: a scheme when it has none (`http`
/// for a local host, else `https`) and a path.
fn normalize_url(text: &str) -> Result<String, Response> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Response::bad_request("URL cannot be empty"));
    }
    if text.len() > 5
        && (text[..5].eq_ignore_ascii_case("data:") || text[..5].eq_ignore_ascii_case("blob:"))
    {
        return Ok(text.to_owned());
    }
    let whole = if has_scheme(text) {
        text.to_owned()
    } else {
        let bare = text.trim_start_matches("//");
        let authority = bare.split(['/', '?', '#']).next().unwrap_or_default();
        let scheme = if is_local(&hostname(authority)) {
            "http"
        } else {
            "https"
        };
        format!("{scheme}://{bare}")
    };
    let (scheme, rest) = whole.split_once("://").unwrap_or_default();
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if hostname(authority).is_empty() && scheme != "file" {
        return Err(Response::bad_request(format!("Invalid URL: {text}")));
    }
    let tail = if tail.is_empty() || !tail.starts_with('/') {
        format!("/{tail}")
    } else {
        tail.to_owned()
    };
    Ok(format!(
        "{}://{}{tail}",
        scheme.to_lowercase(),
        authority.to_lowercase()
    ))
}

/// The group's title: the custom label, else `Breakpoints: host/path`.
fn label(url: &str, custom: Option<&str>) -> String {
    if let Some(custom) = custom.map(str::trim).filter(|custom| !custom.is_empty()) {
        return custom.to_owned();
    }
    let rest = url.split_once("://").map_or("", |(_, rest)| rest);
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    let path = tail.split(['?', '#']).next().unwrap_or_default();
    let path = if path == "/" { "" } else { path };
    format!("Breakpoints: {}{path}", hostname(authority))
}

/// The preset rows `labels` name, or the `400` for the first unknown one.
fn resolve(labels: &[String]) -> Result<Vec<usize>, Response> {
    (labels.iter())
        .map(|name| {
            (VIEWPORT_PRESETS
                .iter()
                .position(|preset| preset.label == name))
            .ok_or_else(|| Response::bad_request(format!("Unknown preset label: {name}")))
        })
        .collect()
}

fn requested_labels(input: &Value) -> Result<Vec<String>, Response> {
    let given = match input.get("presets") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => (items.iter())
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| Response::bad_request("input.presets: expected an array of labels"))?,
        Some(_) => {
            return Err(Response::bad_request(
                "input.presets: expected an array of labels",
            ));
        }
    };
    Ok(if given.is_empty() {
        DEFAULT_PRESETS.map(str::to_owned).to_vec()
    } else {
        given
    })
}

/// Whether a breakpoint cluster for this URL and these presets is already
/// on the canvas.
fn is_duplicate(document: &Document, url: &str, labels: &[String]) -> bool {
    document.entities().any(|entity| {
        let Kind::Group(Group {
            metadata: Some(meta),
            ..
        }) = &entity.kind
        else {
            return false;
        };
        meta.get("taskKind") == Some(&json!(BREAKPOINT_MAP))
            && meta.get("url") == Some(&json!(url))
            && meta.get("presets") == Some(&json!(labels))
    })
}

fn metadata(pairs: &[(&str, Value)]) -> JsonMap {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert((*key).to_owned(), value.clone());
    }
    map
}

/// What a breakpoint cluster is made of: ids, sizes and where it goes.
struct Cluster<'a> {
    url: &'a str,
    labels: &'a [String],
    rows: &'a [usize],
    title: String,
    origin: (f64, f64),
    size: (f64, f64),
    task_id: &'a str,
    group_id: &'a str,
    sync_id: Option<&'a str>,
    page_ids: &'a [String],
    edge_ids: &'a [String],
}

impl Cluster<'_> {
    /// The group, its pages and the edges between them, inserted from
    /// stack index `at` up.
    fn command(&self, mut at: usize) -> Command {
        let (labels, rows) = (self.labels, self.rows);
        let url = self.url.to_owned();
        let (x, y) = self.origin;
        let (width, height) = self.size;
        let task_id = self.task_id.to_owned();
        let group_id = self.group_id;
        let (page_ids, edge_ids) = (self.page_ids, self.edge_ids);
        let sync_id = self.sync_id.map(str::to_owned);
        let group = Entity {
            label: Some(self.title.clone()),
            ..Entity::new(
                group_id,
                Rect::new(
                    x - GROUP_PADDING,
                    y - GROUP_PADDING,
                    width + GROUP_PADDING * 2.0,
                    height + GROUP_PADDING * 2.0,
                ),
                Kind::Group(Group {
                    layout_mode: Some(LayoutMode::Row),
                    managed_layout: Some(true),
                    layout_gap: Some(GUTTER),
                    source_task_id: Some(task_id.clone()),
                    metadata: Some(metadata(&[
                        ("taskKind", json!(BREAKPOINT_MAP)),
                        ("url", json!(url)),
                        ("presets", json!(labels)),
                    ])),
                    ..Group::default()
                }),
            )
        };
        let mut commands = vec![Command::InsertEntity {
            entity: Box::new(group),
            at,
        }];
        let mut cursor = x;
        for (index, &row) in rows.iter().enumerate() {
            let preset = &VIEWPORT_PRESETS[row];
            at += 1;
            let page = Entity {
                parent: Some(EntityId::from(group_id)),
                ..Entity::new(
                    page_ids[index].as_str(),
                    Rect::new(cursor, y, preset.width, preset.height),
                    Kind::Page(Page {
                        url: url.clone(),
                        preset_index: u32::try_from(row).ok(),
                        sync_id: sync_id.clone(),
                        source: Some(PageSource::Generated),
                        metadata: Some(metadata(&[
                            ("taskKind", json!(BREAKPOINT_MAP)),
                            ("url", json!(url)),
                            ("preset", json!(preset.label)),
                        ])),
                        ..Page::default()
                    }),
                )
            };
            commands.push(Command::InsertEntity {
                entity: Box::new(page),
                at,
            });
            cursor += preset.width + GUTTER;
        }
        for (index, edge_id) in edge_ids.iter().enumerate() {
            at += 1;
            let edge = Edge {
                kind: Some(EdgeKind::BreakpointVariant),
                metadata: Some(metadata(&[
                    ("taskKind", json!(BREAKPOINT_MAP)),
                    ("url", json!(url)),
                ])),
                ..Edge::new(
                    edge_id.as_str(),
                    page_ids[index].as_str(),
                    page_ids[index + 1].as_str(),
                )
            };
            commands.push(Command::InsertEdge {
                edge: Box::new(edge),
                at,
            });
        }
        Command::Batch(commands)
    }
}

/// `POST /tasks/apply`.
pub(crate) fn apply(
    ids: &mut Ids,
    app: &specular_interact::App,
    body: &Value,
) -> Result<Step, Response> {
    let kind = body
        .get("taskKind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if kind != BREAKPOINT_MAP {
        return Err(Response::bad_request(format!(
            "Unsupported task kind: {kind}"
        )));
    }
    let input = &body["input"];
    let url = input
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| Response::bad_request("input.url is required"))
        .and_then(normalize_url)?;
    let labels = requested_labels(input)?;
    let mut seen = labels.clone();
    seen.sort();
    seen.dedup();
    if seen.len() != labels.len() {
        return Err(Response::bad_request(
            "Duplicate preset labels are not allowed",
        ));
    }
    let rows = resolve(&labels)?;
    let sizes: Vec<(f64, f64)> = (rows.iter())
        .map(|&row| (VIEWPORT_PRESETS[row].width, VIEWPORT_PRESETS[row].height))
        .collect();
    let width = sizes.iter().map(|size| size.0).sum::<f64>() + GUTTER * (sizes.len() - 1) as f64;
    let height = sizes.iter().map(|size| size.1).fold(0.0, f64::max);

    let anchor = body["options"]
        .get("anchor")
        .cloned()
        .unwrap_or(Value::Null);
    let spot = placement::locate(app, width, height, &json!({ "anchor": anchor }));
    let (x, y) = (
        spot["canvasX"].as_f64().unwrap_or_default(),
        spot["canvasY"].as_f64().unwrap_or_default(),
    );

    let document = app.document();
    let warnings: Vec<&str> = is_duplicate(document, &url, &labels)
        .then_some("A breakpoint cluster for this URL already exists")
        .into_iter()
        .collect();

    let mut taken: Vec<String> = Vec::new();
    let mut fresh = |prefix: &str| {
        let id = ids.fresh(prefix, |id| {
            taken.iter().any(|used| used == id)
                || document.entity(&EntityId::from(id)).is_some()
                || document.edge(&EdgeId::from(id)).is_some()
        });
        taken.push(id.clone());
        id
    };
    let task_id = fresh("task");
    let group_id = fresh("group");
    let sync_id = (rows.len() > 1).then(|| fresh("sync"));
    let page_ids: Vec<String> = rows.iter().map(|_| fresh("page")).collect();
    let edge_ids: Vec<String> = (1..rows.len()).map(|_| fresh("edge")).collect();

    let cluster = Cluster {
        url: &url,
        labels: &labels,
        rows: &rows,
        title: label(&url, input.get("label").and_then(Value::as_str)),
        origin: (x, y),
        size: (width, height),
        task_id: &task_id,
        group_id: &group_id,
        sync_id: sync_id.as_deref(),
        page_ids: &page_ids,
        edge_ids: &edge_ids,
    };
    let command = cluster.command(document.stack_len());
    document
        .clone()
        .apply(command.clone())
        .map_err(|error| Response::bad_request(error.to_string()))?;

    let select = (body["options"]["focus"] != json!(false))
        .then(|| vec![ItemId::Entity(EntityId::from(group_id.as_str()))]);
    let reply = json!({
        "taskId": task_id,
        "taskKind": BREAKPOINT_MAP,
        "groupId": group_id,
        "pageIds": page_ids,
        "edgeIds": edge_ids,
        "resolvedPresets": labels,
        "placement": spot,
        "warnings": warnings,
    });
    Ok(Step::Run(
        ApiRun::Apply { command, select },
        Reply::Fixed(reply),
    ))
}
