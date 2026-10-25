//! The canvas steps: what a page or the system says, and menu bar items chosen.

use anyhow::{Context as _, bail, ensure};
use glam::Vec2;
use specular_core::{ElementPlace, Modifiers, PixelRect};
use specular_interact::{
    Action, App, Appearance, CanvasAction, Event, KeyInput, Orientation, PageNotice, Property,
    Showing,
};

use super::super::words::{key_named, number, theme_named};

/// A canvas step: something a page or the system says, as the shell would
/// pass it on, or a menu bar item chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasStep {
    /// This event, as the shell sends it.
    Send(Event),
    /// `menu Edit > Undo`: the item of that label in that menu, chosen.
    Menu(String),
}

impl CanvasStep {
    /// The event the step sends to `app` now.
    pub(crate) fn event(&self, app: &App) -> anyhow::Result<Event> {
        match self {
            Self::Send(event) => Ok(event.clone()),
            Self::Menu(path) => {
                let (title, label) = (path.split_once('>'))
                    .map(|(title, label)| (title.trim(), label.trim()))
                    .with_context(|| format!("`menu {path}` is `menu Title > Item`"))?;
                let menu = (specular_interact::menus(app).into_iter())
                    .find(|menu| menu.title == title)
                    .with_context(|| format!("no menu `{title}`"))?;
                let item = (menu.entries.into_iter())
                    .find_map(|entry| match entry {
                        specular_interact::MenuEntry::Item(item) if item.label == label => {
                            Some(item)
                        }
                        _ => None,
                    })
                    .with_context(|| format!("no item `{label}` in `{title}`"))?;
                ensure!(item.enabled, "`{title} > {label}` is not enabled");
                Ok(Event::Action(item.action))
            }
        }
    }
}

/// The canvas step `verb` and `rest` name, or `None` when `verb` is not one
/// of them.
pub(crate) fn step(verb: &str, rest: &str) -> anyhow::Result<Option<CanvasStep>> {
    let words: Vec<&str> = rest.split_whitespace().collect();
    let event = match (verb, words.as_slice()) {
        ("page-says", [page, said @ ..]) => Event::Page {
            page: (*page).into(),
            notice: notice(said)?,
        },
        ("appearance", ["light"]) => Event::SystemAppearance(Appearance::Light),
        ("appearance", ["dark"]) => Event::SystemAppearance(Appearance::Dark),
        ("theme-loaded", [name]) => Event::ThemeLoaded(theme_named(name)?),
        ("repeat", [key]) => Event::Key(KeyInput {
            key: key_named(key)?,
            pressed: true,
            repeat: true,
            text: None,
            character: None,
            modifiers: Modifiers::default(),
            windows_key_code: 0,
            native_key_code: 0,
            commands: Vec::new(),
        }),
        ("page-says" | "appearance" | "theme-loaded" | "repeat", _) => {
            bail!("the wrong arguments")
        }
        ("menu", _) if rest.contains('>') => return Ok(Some(CanvasStep::Menu(rest.to_owned()))),
        _ => return Ok(None),
    };
    Ok(Some(CanvasStep::Send(event)))
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
        ["fit", "covers"] => PageNotice::ViewportFit {
            covers: true,
            color: None,
        },
        ["fit", "inside"] => PageNotice::ViewportFit {
            covers: false,
            color: None,
        },
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
pub(crate) fn action_named(name: &str) -> Option<Action> {
    if let Some(id) = name.strip_prefix("show:") {
        return Some(Action::Show(Showing::Item(id.into())));
    }
    Some(match name {
        "show-canvas" => Action::Show(Showing::Canvas),
        "landscape" => Action::SetProperty(Property::Orientation(Orientation::Landscape)),
        "portrait" => Action::SetProperty(Property::Orientation(Orientation::Portrait)),
        "bring-forward" => Action::BringForward,
        "send-backward" => Action::SendBackward,
        "bring-to-front" => Action::BringToFront,
        "send-to-back" => Action::SendToBack,
        "group" => Action::Group,
        "auto-layout" => Action::AutoLayout,
        "duplicate" => Action::Duplicate,
        "copy" => Action::Copy,
        "delete" => Action::Delete,
        "new-canvas" => Action::Canvas(CanvasAction::New),
        "duplicate-canvas" => Action::Canvas(CanvasAction::Duplicate(None)),
        "delete-canvas" => Action::Canvas(CanvasAction::Delete(None)),
        _ => return None,
    })
}
