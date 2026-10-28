//! What each canvas query reads of the app, as words.

use anyhow::{Context as _, bail};
use specular_doc::{EdgeId, EdgeSide, EntityId};
use specular_interact::{
    App, Appearance, Corner, GuideAxis, GuideReference, Handle, HandleOwner, Hit, ImageState,
    LayoutHandle, Mark, PlacePreview, hit_test, shown_rect,
};

use super::Query;

fn none() -> Vec<String> {
    vec!["none".to_owned()]
}

pub(super) fn words_of(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn numbers(values: &[f64]) -> Vec<String> {
    values.iter().map(|value| format!("{value}")).collect()
}

fn rect_words(rect: specular_doc::Rect) -> Vec<String> {
    numbers(&[rect.x, rect.y, rect.width, rect.height])
}

fn entity_id(app: &App, name: &str) -> anyhow::Result<EntityId> {
    Ok(EntityId::from(
        super::super::check::resolve(app, name)?.as_str(),
    ))
}

/// What `query` reads of `app`, as words.
#[expect(
    clippy::too_many_lines,
    reason = "one short arm a fact, read side by side"
)]
pub(super) fn read(app: &App, query: &Query) -> anyhow::Result<Vec<String>> {
    let session = app.session();
    Ok(match query {
        Query::Hit(at) => hit_words(&hit_test(app, *at)),
        Query::Cursor => vec![kebab(&format!("{:?}", session.cursor))],
        Query::Hover => session
            .hover
            .as_ref()
            .map_or_else(none, |id| vec![id.to_string()]),
        Query::Marquee => app.marquee().map_or_else(none, rect_words),
        Query::Mark(name) => match app.mark(&entity_id(app, name)?) {
            Some(Mark::Outline) => words_of("outline"),
            Some(Mark::Entered) => words_of("entered"),
            None => none(),
        },
        Query::Handles => match app.handles() {
            Some((HandleOwner::Entity(id), _)) => vec!["entity".to_owned(), id.to_string()],
            Some((HandleOwner::Selection, _)) => words_of("selection"),
            None => none(),
        },
        Query::Anchors => vec![app.anchors().len().to_string()],
        Query::AnchorLit => {
            let lit = app.anchors().into_iter().find(|anchor| anchor.active);
            lit.map_or_else(none, |anchor| {
                let mut words = vec![side_word(anchor.side).to_owned()];
                words.extend(numbers(&[
                    f64::from(anchor.point.x),
                    f64::from(anchor.point.y),
                ]));
                words
            })
        }
        Query::EdgePreview => match app.edge_preview() {
            Some(preview) if preview.snap.is_some() => words_of("snap"),
            Some(_) => words_of("free"),
            None => none(),
        },
        Query::Guides(axis) => {
            let guides = app.guides();
            let lines: Vec<String> = (guides.alignment.iter())
                .filter(|guide| guide.axis == *axis)
                .flat_map(|guide| {
                    [
                        format!(
                            "{}/{}.{}",
                            reference_word(guide.dragged_reference),
                            guide.candidate,
                            reference_word(guide.candidate_reference)
                        ),
                        format!("{}", guide.coordinate),
                    ]
                })
                .collect();
            if lines.is_empty() { none() } else { lines }
        }
        Query::AnyGuides => words_of(if app.guides().is_empty() {
            "none"
        } else {
            "some"
        }),
        Query::Gaps => {
            let guides = app.guides();
            if guides.distribution.is_empty() {
                none()
            } else {
                let mut words = Vec::new();
                for run in &guides.distribution {
                    words.push(axis_word(run.axis).to_owned());
                    words.push(format!("{}", run.gap));
                    for gap in &run.gaps {
                        words.push("|".to_owned());
                        words.extend(numbers(&[gap.start, gap.end, gap.cross]));
                    }
                }
                words
            }
        }
        Query::DropTarget => app
            .group_drop_target()
            .map_or_else(none, |id| vec![id.to_string()]),
        Query::EnteredGroup => app
            .entered_group()
            .map_or_else(none, |id| vec![id.to_string()]),
        Query::Seen(name) => {
            let id = entity_id(app, name)?;
            let entity = app
                .document()
                .entity(&id)
                .with_context(|| format!("no entity `{id}`"))?;
            shown_rect(app, entity).map_or_else(none, rect_words)
        }
        Query::Page(name) => {
            let id = entity_id(app, name)?;
            match app.page_placement(&id) {
                Some(placement) => {
                    let mut words = rect_words(placement.rect);
                    words.push("viewport".to_owned());
                    words.push(placement.viewport.width.to_string());
                    words.push(placement.viewport.height.to_string());
                    words
                }
                None => none(),
            }
        }
        Query::PlacePreview => match app.place_preview() {
            Some(PlacePreview::Entity(entity)) => {
                let mut words = vec![super::super::check::kind_word(&entity).to_owned()];
                words.extend(rect_words(entity.rect));
                words
            }
            Some(PlacePreview::Document(rect)) => {
                let mut words = vec!["document".to_owned()];
                words.extend(rect_words(rect));
                words
            }
            None => none(),
        },
        Query::Dots => {
            let dots: Vec<String> = (app.reorder_dots().iter())
                .map(|dot| dot.entity.to_string())
                .collect();
            if dots.is_empty() { none() } else { dots }
        }
        Query::GapStrips => vec![app.gap_handles().len().to_string()],
        Query::Ghost => app.reorder_ghost().map_or_else(none, |ghost| {
            let mut words = vec![ghost.entity.to_string()];
            words.extend(numbers(&[ghost.delta.x, ghost.delta.y]));
            words
        }),
        Query::Caret => match app.text_edit() {
            Some(edit) => vec![edit.caret().to_string(), edit.anchor().to_string()],
            None => none(),
        },
        Query::Scope => {
            let scope = app.selection_scope();
            let ids = |ids: &[EntityId]| ids.iter().map(ToString::to_string).collect::<Vec<_>>();
            let mut words = ids(&scope.members);
            words.push("|".to_owned());
            words.extend(ids(&scope.operands));
            words.push("|".to_owned());
            words.extend(scope.bounds.map_or_else(none, rect_words));
            words
        }
        Query::Default(name) => {
            let defaults = app.tool_defaults();
            let word = match name.as_str() {
                "shape" => format!("{:?}", defaults.shape.kind),
                "brush" => format!("{:?}", defaults.draw.brush),
                "sticky-font" => format!("{:?}", defaults.sticky.font),
                other => bail!("unknown default `{other}` (shape, brush or sticky-font)"),
            };
            vec![kebab(&word)]
        }
        Query::Theme => {
            let appearance = match app.appearance() {
                Appearance::Light => "light",
                Appearance::Dark => "dark",
            };
            vec![app.theme().key().to_owned(), appearance.to_owned()]
        }
        Query::Motion => {
            let running = app.motion_running() || app.turn_running();
            words_of(if running { "running" } else { "still" })
        }
        Query::Synced(name) => {
            let synced = app.is_synced(&entity_id(app, name)?);
            words_of(if synced { "yes" } else { "no" })
        }
        Query::Anchor(name) => {
            let id = entity_id(app, name)?;
            let entity = app
                .document()
                .entity(&id)
                .with_context(|| format!("no entity `{id}`"))?;
            (entity.anchor.as_ref()).map_or_else(none, |anchor| vec![anchor.page_id.to_string()])
        }
        Query::Control(name) => {
            let shown = specular_interact::control_named(app, name).is_ok();
            words_of(if shown { "yes" } else { "no" })
        }
        Query::Edge(name) => {
            let edge = edge_id(app, name).and_then(|id| app.document().edge(&id).cloned());
            edge.map_or_else(none, |edge| {
                vec![edge.from.to_string(), edge.to.to_string()]
            })
        }
        Query::EdgeEnds(name) => {
            let path = edge_id(app, name).and_then(|id| app.edge_path(&id));
            path.map_or_else(none, |path| {
                let (from, to) = (path.from(), path.to());
                let ends = [from.x, from.y, to.x, to.y];
                numbers(&ends.map(f64::from))
            })
        }
        Query::Order => (app.document().order().iter())
            .map(|item| item.as_str().to_owned())
            .collect(),
        Query::Image(file) => match app.image(file).map(|image| &image.state) {
            Some(ImageState::Loading) => words_of("loading"),
            Some(ImageState::Ready { width, height }) => {
                vec!["ready".to_owned(), width.to_string(), height.to_string()]
            }
            Some(other) => vec![kebab(&format!("{other:?}"))],
            None => none(),
        },
    })
}

/// The edge `name` names: its id, or `@last` for the front-most.
fn edge_id(app: &App, name: &str) -> Option<EdgeId> {
    match name {
        "@last" => app.document().edges().last().map(|edge| edge.id.clone()),
        _ => Some(EdgeId::from(name)),
    }
}

fn hit_words(hit: &Hit) -> Vec<String> {
    let words = match hit {
        Hit::Comment { annotation } => format!("comment {annotation}"),
        Hit::GroupLabel { group } => format!("label {group}"),
        Hit::Handle { owner, handle } => {
            let owner = match owner {
                HandleOwner::Entity(id) => id.to_string(),
                HandleOwner::Selection => "selection".to_owned(),
            };
            let handle = match handle {
                Handle::Corner(corner) => corner_word(*corner).to_owned(),
                Handle::Side(side) => side_word(*side).to_owned(),
            };
            format!("handle {owner} {handle}")
        }
        Hit::Anchor { entity, side } => format!("anchor {entity} {}", side_word(*side)),
        Hit::Layout(LayoutHandle::Reorder { entity }) => format!("dot {entity}"),
        Hit::Layout(LayoutHandle::Gap { axis, .. }) => {
            format!("gap {}", kebab(&format!("{axis:?}")))
        }
        Hit::PageContent { page, local } => format!("page {page} {} {}", local.x, local.y),
        Hit::EntityBody { entity } => format!("body {entity}"),
        Hit::GroupBorder { group } => format!("border {group}"),
        Hit::Edge { edge } => format!("edge {edge}"),
        Hit::Empty => "empty".to_owned(),
    };
    words_of(&words)
}

const fn side_word(side: EdgeSide) -> &'static str {
    match side {
        EdgeSide::Top => "top",
        EdgeSide::Right => "right",
        EdgeSide::Bottom => "bottom",
        EdgeSide::Left => "left",
    }
}

const fn corner_word(corner: Corner) -> &'static str {
    match corner {
        Corner::TopLeft => "top-left",
        Corner::TopRight => "top-right",
        Corner::BottomRight => "bottom-right",
        Corner::BottomLeft => "bottom-left",
    }
}

const fn axis_word(axis: GuideAxis) -> &'static str {
    match axis {
        GuideAxis::Horizontal => "horizontal",
        GuideAxis::Vertical => "vertical",
    }
}

const fn reference_word(reference: GuideReference) -> &'static str {
    match reference {
        GuideReference::Top => "top",
        GuideReference::Bottom => "bottom",
        GuideReference::HCenter => "middle",
        GuideReference::Left => "left",
        GuideReference::Right => "right",
        GuideReference::VCenter => "centre",
    }
}

/// `ResizeNwse` as `resize-nwse`: the variant name of a `Debug` text.
pub(super) fn kebab(debug: &str) -> String {
    let name = debug
        .split(|character: char| !character.is_alphanumeric())
        .next()
        .unwrap_or_default();
    let mut out = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('-');
        }
        out.push(character.to_ascii_lowercase());
    }
    out
}
