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
//! expect guides horizontal top/b.top 100 bottom/b.bottom 200   # or `none`
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
//! expect edge-ends e1 208 150 492 150  # an edge's ends on screen; `@last`
//! expect edge @last @3 @4              # what an edge runs from and to
//! expect shown-control group.gap.inc no   # whether a control is offered
//! expect anchor @top @8              # the page an item is hooked to, or `none`
//! expect order a b @top c            # the stack, back to front, edges too
//! expect synced p1 no                  # in a sync set: yes or no
//! expect sent viewport p 500 300       # the last step asked the shell this
//! expect sent clipboard specular:canvas:  # its last word may be a prefix
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
//! repeat down                          # a key held down repeats
//! menu Edit > Undo                     # a menu bar item chosen
//! act show:p1                          # p1 alone in its tab; `act show-canvas`
//! ```

mod read;
mod sent;
mod step;

use self::read::read;
pub(crate) use self::sent::check_sent;
pub use self::step::CanvasStep;
pub(super) use self::step::{action_named, step};

use anyhow::{bail, ensure};
use glam::Vec2;
use specular_interact::{App, GuideAxis};

use super::words::number;

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
    pub(crate) fn is_sent(&self) -> bool {
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
    /// Where an edge leaves and reaches its entities, on screen.
    EdgeEnds(String),
    /// The entities an edge runs from and to.
    Edge(String),
    /// Whether a control of that name is offered now.
    Control(String),
    /// The page an entity is hooked to.
    Anchor(String),
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
        "mark" | "seen" | "page" | "synced" | "default" | "image" | "edge-ends" | "edge"
        | "shown-control" | "anchor" => {
            let (name, wanted) = id(&words)?;
            let query = match what {
                "mark" => Query::Mark(name),
                "seen" => Query::Seen(name),
                "page" => Query::Page(name),
                "synced" => Query::Synced(name),
                "default" => Query::Default(name),
                "edge-ends" => Query::EdgeEnds(name),
                "edge" => Query::Edge(name),
                "shown-control" => Query::Control(name),
                "anchor" => Query::Anchor(name),
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
    let wanted = resolved(app, wanted)?;
    ensure!(
        same_words(&found, &wanted),
        "{query:?} is `{}`, expected `{}`",
        found.join(" "),
        wanted.join(" ")
    );
    Ok(())
}

/// `words` with each `@` name read as the entity it stands for now.
fn resolved(app: &App, words: &[String]) -> anyhow::Result<Vec<String>> {
    (words.iter())
        .map(|word| match word.strip_prefix('@') {
            Some(_) => super::check::resolve(app, word),
            None => Ok(word.clone()),
        })
        .collect()
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
