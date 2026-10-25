//! The canvas facts and steps: what the pointer and the selection show
//! (handles, anchors, guides, the marquee, the cursor), what a point hits,
//! where an item hooked to a page is seen, and what the app last asked the
//! shell to do.
//!
//! ```text
//! expect hit 400 185 anchor f1 top     # what is under a screen point
//! expect cursor resize-nwse            # the cursor the shell shows
//! expect hover p1                      # or `none`
//! expect marquee 600 600 3 4           # the rubber band, or `none`
//! expect mark p1 entered               # outline, entered or none
//! expect handles entity s1             # or `selection`, or `none`
//! expect anchors 4                     # how many edge anchors are offered
//! expect anchor-lit right 308 150      # the one under the pointer, or `none`
//! expect edge-preview snap             # an edge in flight: snap, free, none
//! expect guides horizontal top/top 100 bottom/bottom 200   # or `none`
//! expect guides none                  # no line and no gap shows
//! expect gaps horizontal 100 | 200 300 150 | 400 500 150   # or `none`
//! expect drop-target g                 # the group a release would join
//! expect entered-group outer           # the group stepped into, or `none`
//! expect seen s1 200 160 100 100       # where an item is drawn
//! expect page p1 100 100 400 300 viewport 400 300   # its document's place
//! expect place-preview sticky 100 100 200 200   # an armed tool's ghost
//! expect dots a b c                    # a line's reorder dots, or `none`
//! expect gap-strips 2                  # a line's gap handles
//! expect ghost a 20 40                 # the box a reorder floats, or `none`
//! expect caret 1 1                     # the caret and anchor, in bytes
//! expect scope g out | g a b out | 100 100 800 500   # members | operands | bounds
//! expect default shape ellipse         # a tool default: shape, brush, sticky-font
//! expect theme system dark             # the choice, and what is drawn
//! expect motion running                # a camera or turn spring: running, still
//! expect order a b @top c            # the stack, back to front, edges too
//! expect synced p1 no                  # in a sync set: yes or no
//! expect sent viewport p 500 300       # the last step asked the shell this
//! expect sent no save                  # it asked nothing that starts so
//! expect sent nothing                  # it asked nothing at all
//! ```
//!
//! Numbers compare within half a unit. `sent` reads the effects of the last
//! step that was not a check, which only a headless run sees: a window takes
//! a `sent` line as holding.
//!
//! ```text
//! page-says p1 scrolled 0 40           # the page reports a scroll
//! page-says p1 url https://example.com/b
//! page-says p1 fit covers              # or `fit inside`
//! page-says p1 element #hero 100 160   # where an element is, `fixed` after
//! page-says p1 composition 20 40 60 16 # the input method's box, or `none`
//! appearance dark                      # the system's light or dark
//! theme-loaded light                   # the choice read from preferences
//! ```

use anyhow::{Context as _, bail, ensure};
use glam::Vec2;
use specular_core::{ElementPlace, InputEvent, KeyEventKind, PixelRect, PointerEventKind, TouchPhase};
use specular_doc::{EdgeSide, EntityId};
use specular_interact::{
    Action, App, Appearance, Corner, Effect, Event, GuideAxis, GuideReference,
    Handle, HandleOwner, Hit, ImageState, LayoutHandle, Mark, PageNotice, PlacePreview,
    hit_test, shown_rect,
};

use super::words::{number, theme_named};

/// A canvas fact: something read from the app, compared as words.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasFact {
    /// What `query` reads now is `wanted`.
    Is(Query, Vec<String>),
    /// The last step asked the shell for something that starts with these
    /// words, or with `no` before them, asked for nothing that does.
    Sent {
        /// Whether no effect may start so.
        none: bool,
        /// The words an effect starts with.
        words: Vec<String>,
    },
    /// The last step asked the shell for nothing.
    SentNothing,
}

impl CanvasFact {
    /// Whether the fact is about effects, which only a headless run sees.
    pub fn is_sent(&self) -> bool {
        matches!(self, Self::Sent { .. } | Self::SentNothing)
    }
}

/// What a canvas fact reads.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// What is under a screen point.
    Hit(Vec2),
    /// The cursor the shell was last asked to show.
    Cursor,
    /// The hovered entity.
    Hover,
    /// The marquee's rect on the canvas.
    Marquee,
    /// How an entity is marked.
    Mark(String),
    /// Who owns the resize handles.
    Handles,
    /// How many edge anchors are offered.
    Anchors,
    /// The anchor under the pointer.
    AnchorLit,
    /// An edge being dragged.
    EdgePreview,
    /// The alignment guides along an axis.
    Guides(GuideAxis),
    /// The distribution guides.
    Gaps,
    /// Whether any guide shows.
    AnyGuides,
    /// The group a release would drop into.
    DropTarget,
    /// The group stepped into.
    EnteredGroup,
    /// Where an entity is drawn.
    Seen(String),
    /// Where a page's document is laid out.
    Page(String),
    /// An armed tool's ghost.
    PlacePreview,
    /// A line's reorder dots.
    Dots,
    /// A line's gap strips.
    GapStrips,
    /// The box a reorder floats.
    Ghost,
    /// The caret and anchor of the text being edited.
    Caret,
    /// The selection's members, operands and bounds.
    Scope,
    /// A tool default.
    Default(String),
    /// The theme choice and what is drawn.
    Theme,
    /// Whether a camera or turn spring is running.
    Motion,
    /// Whether a page is in a sync set.
    Synced(String),
    /// An image file's state.
    Image(String),
    /// The stack, back to front, edges included.
    Order,
}

/// The canvas fact `what` and `tail` state, or `None` when `what` is not one
/// of them.
pub(super) fn fact(what: &str, tail: &str) -> anyhow::Result<Option<CanvasFact>> {
    let words: Vec<&str> = tail.split_whitespace().collect();
    let owned = |words: &[&str]| words.iter().map(|&word| word.to_owned()).collect();
    let id = |words: &[&str]| -> anyhow::Result<(String, Vec<String>)> {
        match words {
            [id, rest @ ..] => Ok(((*id).to_owned(), owned(rest))),
            [] => bail!("`{what}` names an entity"),
        }
    };
    let is = |query: Query, wanted: &[&str]| Ok(Some(CanvasFact::Is(query, owned(wanted))));
    match what {
        "hit" => match words.as_slice() {
            [x, y, wanted @ ..] => {
                let at = Vec2::new(number(x)?, number(y)?);
                is(Query::Hit(at), wanted)
            }
            _ => bail!("`hit X Y WHAT`"),
        },
        "cursor" => is(Query::Cursor, &words),
        "hover" => is(Query::Hover, &words),
        "marquee" => is(Query::Marquee, &words),
        "handles" => is(Query::Handles, &words),
        "anchors" => is(Query::Anchors, &words),
        "anchor-lit" => is(Query::AnchorLit, &words),
        "edge-preview" => is(Query::EdgePreview, &words),
        "gaps" => is(Query::Gaps, &words),
        "drop-target" => is(Query::DropTarget, &words),
        "entered-group" => is(Query::EnteredGroup, &words),
        "place-preview" => is(Query::PlacePreview, &words),
        "dots" => is(Query::Dots, &words),
        "gap-strips" => is(Query::GapStrips, &words),
        "ghost" => is(Query::Ghost, &words),
        "caret" => is(Query::Caret, &words),
        "scope" => is(Query::Scope, &words),
        "theme" => is(Query::Theme, &words),
        "motion" => is(Query::Motion, &words),
        "order" => is(Query::Order, &words),
        "guides" => match words.as_slice() {
            ["horizontal", wanted @ ..] => is(Query::Guides(GuideAxis::Horizontal), wanted),
            ["vertical", wanted @ ..] => is(Query::Guides(GuideAxis::Vertical), wanted),
            ["none"] => is(Query::AnyGuides, &["none"]),
            _ => bail!("`guides horizontal|vertical ..` or `guides none`"),
        },
        "mark" | "seen" | "page" | "synced" | "default" | "image" => {
            let (name, wanted) = id(&words)?;
            let query = match what {
                "mark" => Query::Mark(name),
                "seen" => Query::Seen(name),
                "page" => Query::Page(name),
                "synced" => Query::Synced(name),
                "default" => Query::Default(name),
                _ => Query::Image(name),
            };
            Ok(Some(CanvasFact::Is(query, wanted)))
        }
        "sent" => Ok(Some(match words.as_slice() {
            ["nothing"] => CanvasFact::SentNothing,
            ["no", rest @ ..] if !rest.is_empty() => CanvasFact::Sent {
                none: true,
                words: owned(rest),
            },
            [] => bail!("`sent WORDS`, `sent no WORDS` or `sent nothing`"),
            _ => CanvasFact::Sent {
                none: false,
                words: owned(&words),
            },
        })),
        _ => Ok(None),
    }
}

/// Whether `fact` holds for `app`. A `sent` fact holds: only a headless run
/// sees effects, and it checks those with [`check_sent`].
pub(super) fn check(app: &App, fact: &CanvasFact) -> anyhow::Result<()> {
    let CanvasFact::Is(query, wanted) = fact else {
        return Ok(());
    };
    let found = read(app, query)?;
    let wanted: Vec<String> = (wanted.iter())
        .map(|word| match word.strip_prefix('@') {
            Some(_) => super::check::resolve(app, word),
            None => Ok(word.clone()),
        })
        .collect::<anyhow::Result<_>>()?;
    ensure!(
        same_words(&found, &wanted),
        "{query:?} is `{}`, expected `{}`",
        found.join(" "),
        wanted.join(" ")
    );
    Ok(())
}

/// Whether a `sent` fact holds for the effects of the last step.
pub fn check_sent(sent: &[Effect], fact: &CanvasFact) -> anyhow::Result<()> {
    let told: Vec<Vec<String>> = sent.iter().map(effect_words).collect();
    let listed = || {
        let lines: Vec<String> = told.iter().map(|words| words.join(" ")).collect();
        if lines.is_empty() {
            " nothing".to_owned()
        } else {
            format!("\n  {}", lines.join("\n  "))
        }
    };
    match fact {
        CanvasFact::SentNothing => ensure!(told.is_empty(), "the last step sent:{}", listed()),
        CanvasFact::Sent { none, words } => {
            let starts = |effect: &Vec<String>| {
                effect.len() >= words.len() && same_words(&effect[..words.len()], words)
            };
            let found = told.iter().any(starts);
            ensure!(
                found != *none,
                "the last step {} `{}`; it sent:{}",
                if *none { "sent" } else { "did not send" },
                words.join(" "),
                listed()
            );
        }
        CanvasFact::Is(..) => {}
    }
    Ok(())
}

/// Whether two lists of words are the same, numbers within half a unit.
fn same_words(found: &[String], wanted: &[String]) -> bool {
    found.len() == wanted.len()
        && found.iter().zip(wanted).all(|(found, wanted)| {
            match (found.parse::<f64>(), wanted.parse::<f64>()) {
                (Ok(found), Ok(wanted)) => (found - wanted).abs() <= 0.5,
                _ => found == wanted,
            }
        })
}

fn none() -> Vec<String> {
    vec!["none".to_owned()]
}

fn words_of(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn numbers(values: &[f64]) -> Vec<String> {
    values.iter().map(|value| format!("{value}")).collect()
}

fn rect_words(rect: specular_doc::Rect) -> Vec<String> {
    numbers(&[rect.x, rect.y, rect.width, rect.height])
}

fn entity_id(app: &App, name: &str) -> anyhow::Result<EntityId> {
    Ok(EntityId::from(super::check::resolve(app, name)?.as_str()))
}

/// What `query` reads of `app`, as words.
fn read(app: &App, query: &Query) -> anyhow::Result<Vec<String>> {
    let session = app.session();
    Ok(match query {
        Query::Hit(at) => hit_words(&hit_test(app, *at)),
        Query::Cursor => vec![kebab(&format!("{:?}", session.cursor))],
        Query::Hover => session.hover.as_ref().map_or_else(none, |id| vec![id.to_string()]),
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
                words.extend(numbers(&[f64::from(anchor.point.x), f64::from(anchor.point.y)]));
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
                            "{}/{}",
                            reference_word(guide.dragged_reference),
                            reference_word(guide.candidate_reference)
                        ),
                        format!("{}", guide.coordinate),
                    ]
                })
                .collect();
            if lines.is_empty() { none() } else { lines }
        }
        Query::AnyGuides => words_of(if app.guides().is_empty() { "none" } else { "some" }),
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
        Query::DropTarget => app.group_drop_target().map_or_else(none, |id| vec![id.to_string()]),
        Query::EnteredGroup => app.entered_group().map_or_else(none, |id| vec![id.to_string()]),
        Query::Seen(name) => {
            let id = entity_id(app, name)?;
            let entity = app.document().entity(&id).with_context(|| format!("no entity `{id}`"))?;
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
                let mut words = vec![super::check::kind_word(&entity).to_owned()];
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
        Hit::Layout(LayoutHandle::Gap { axis, .. }) => format!("gap {}", kebab(&format!("{axis:?}"))),
        Hit::PageContent { page, local } => format!("page {page} {} {}", local.x, local.y),
        Hit::EntityBody { entity } => format!("body {entity}"),
        Hit::GroupBorder { group } => format!("border {group}"),
        Hit::Edge { edge } => format!("edge {edge}"),
        Hit::Empty => "empty".to_owned(),
        Hit::Panel { .. } => "panel".to_owned(),
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
fn kebab(debug: &str) -> String {
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

/// An effect as the words a `sent` line matches.
fn effect_words(effect: &Effect) -> Vec<String> {
    let text = match effect {
        Effect::ForwardInput { page, event } => format!("forward {page} {}", input_words(event)),
        Effect::CreatePage { page, url, viewport } => {
            format!("create-page {page} {url} {} {}", viewport.width, viewport.height)
        }
        Effect::ClosePage(page) => format!("close-page {page}"),
        Effect::SetPageViewport { page, viewport } => {
            format!("viewport {page} {} {}", viewport.width, viewport.height)
        }
        Effect::SetCursor(cursor) => format!("cursor {}", kebab(&format!("{cursor:?}"))),
        Effect::FocusPage(page) => {
            format!("focus-page {}", page.as_ref().map_or("none".to_owned(), ToString::to_string))
        }
        Effect::WriteClipboard(text) => format!("clipboard {text}"),
        Effect::CaptureElement { page, point, .. } => {
            format!("capture {page} {} {}", point.x, point.y)
        }
        Effect::TrackElements { page, selectors } => {
            format!("track {page} {}", selectors.join(" "))
        }
        Effect::SetImeCursorArea { origin, size } => {
            format!("ime-area {} {} {} {}", origin.x, origin.y, size.x, size.y)
        }
        Effect::SaveTheme(theme) => format!("save-theme {}", theme.key()),
        Effect::SetPageColorScheme { page, scheme } => format!(
            "scheme {page} {}",
            scheme.map_or("none".to_owned(), |scheme| kebab(&format!("{scheme:?}")))
        ),
        Effect::WriteAsset { file, .. } => format!("write-asset {file}"),
        Effect::CopyAsset { from, file } => format!("copy-asset {from} {file}"),
        Effect::LoadImage { file, .. } => format!("load-image {file}"),
        Effect::LoadNote { file } => format!("load-note {file}"),
        other => kebab(&format!("{other:?}")),
    };
    words_of(&text)
}

fn input_words(event: &InputEvent) -> String {
    match event {
        InputEvent::Pointer(pointer) => {
            let kind = match pointer.kind {
                PointerEventKind::Down { .. } => "down",
                PointerEventKind::Up { .. } => "up",
                PointerEventKind::Move => "move",
                PointerEventKind::Leave => "leave",
            };
            format!("{kind} {} {}", pointer.position.x, pointer.position.y)
        }
        InputEvent::Touch(touch) => {
            let phase = match touch.phase {
                TouchPhase::Start => "start",
                TouchPhase::Move => "move",
                TouchPhase::End => "end",
            };
            format!("touch {phase} {} {}", touch.position.x, touch.position.y)
        }
        InputEvent::Wheel(wheel) => format!("wheel {} {}", wheel.delta.x, wheel.delta.y),
        InputEvent::Key(key) => {
            let kind = match key.kind {
                KeyEventKind::RawDown => "down",
                KeyEventKind::Up => "up",
                KeyEventKind::Char => "char",
            };
            format!("key {kind} {}", key.native_key_code)
        }
        InputEvent::Ime(_) => "ime".to_owned(),
        InputEvent::Edit(edit) => format!("edit {}", kebab(&format!("{edit:?}"))),
    }
}

/// A canvas step: something a page or the system says, as the shell would
/// pass it on.
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasStep(Event);

impl CanvasStep {
    /// The event the step sends.
    pub fn event(&self) -> Event {
        self.0.clone()
    }
}

/// The canvas step `verb` and `rest` name, or `None` when `verb` is not one
/// of them.
pub(super) fn step(verb: &str, rest: &str) -> anyhow::Result<Option<CanvasStep>> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let event = match (verb, words.as_slice()) {
        ("page-says", [page, said @ ..]) => Event::Page {
            page: (*page).into(),
            notice: notice(said)?,
        },
        ("appearance", ["light"]) => Event::SystemAppearance(Appearance::Light),
        ("appearance", ["dark"]) => Event::SystemAppearance(Appearance::Dark),
        ("theme-loaded", [name]) => Event::ThemeLoaded(theme_named(name)?),
        ("page-says" | "appearance" | "theme-loaded", _) => bail!("the wrong arguments"),
        _ => return Ok(None),
    };
    Ok(Some(CanvasStep(event)))
}

fn notice(said: &[&str]) -> anyhow::Result<PageNotice> {
    fn whole<T: std::str::FromStr>(word: &str) -> anyhow::Result<T> {
        word.parse()
            .ok()
            .with_context(|| format!("`{word}` is not a whole number"))
    }
    Ok(match said {
        ["scrolled", x, y] => PageNotice::Scrolled {
            x: f64::from(number(x)?),
            y: f64::from(number(y)?),
        },
        ["url", url] => PageNotice::Url((*url).to_owned()),
        ["fit", "covers"] => PageNotice::ViewportFit { covers: true, color: None },
        ["fit", "inside"] => PageNotice::ViewportFit { covers: false, color: None },
        ["element", selector, x, y, fixed @ ..] => PageNotice::ElementPlaces(vec![(
            (*selector).to_owned(),
            Some(ElementPlace {
                doc: Vec2::new(number(x)?, number(y)?),
                viewport_positioned: fixed == ["fixed"],
            }),
        )]),
        ["element", selector, "gone"] => {
            PageNotice::ElementPlaces(vec![((*selector).to_owned(), None)])
        }
        ["composition", "none"] => PageNotice::ImeCompositionBounds(None),
        ["composition", x, y, width, height] => PageNotice::ImeCompositionBounds(Some(
            PixelRect::new(whole(x)?, whole(y)?, whole(width)?, whole(height)?),
        )),
        _ => bail!(
            "a page says `scrolled X Y`, `url U`, `fit covers|inside`, `element SEL X Y [fixed]` or `composition X Y W H`"
        ),
    })
}

/// The canvas verbs `act` names beyond the shared ones.
pub(super) fn action_named(name: &str) -> Option<Action> {
    Some(match name {
        "bring-forward" => Action::BringForward,
        "send-backward" => Action::SendBackward,
        "bring-to-front" => Action::BringToFront,
        "send-to-back" => Action::SendToBack,
        "group" => Action::Group,
        "auto-layout" => Action::AutoLayout,
        "duplicate" => Action::Duplicate,
        "copy" => Action::Copy,
        "delete" => Action::Delete,
        _ => return None,
    })
}
