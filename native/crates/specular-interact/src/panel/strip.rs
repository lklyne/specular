//! [`view_strip`]: the tabs of the tab row. The first shows the canvas, and
//! one follows for each page and Document of the active canvas, which a
//! press shows alone.

use std::sync::Arc;

use specular_doc::{Entity, Kind};

use super::{Button, ControlId, Face, Icon, Label};
use crate::labels::{file_icon, file_label, page_icon, page_label};
use crate::{Action, App, Showing, binding_of, showing};

/// One tab of the tab row.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewTab {
    /// Its name.
    pub id: ControlId,
    /// What it is called.
    pub label: Label,
    /// The glyph before its label.
    pub icon: Icon,
    /// A page's own icon as a PNG, which a renderer that draws images
    /// shows in place of the glyph.
    pub favicon: Option<Arc<[u8]>>,
    /// Whether it is what the window shows now.
    pub active: bool,
    /// What pressing it does: show the canvas, or its item alone.
    pub action: Action,
    /// What the press does before the click, for a renderer that sees the
    /// button go down: lay its page out for the tab. `None` for a tab with
    /// nothing to lay out.
    pub prepare: Option<Action>,
}

/// The tab row as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewStrip {
    /// The tabs, left to right: the canvas first.
    pub tabs: Vec<ViewTab>,
    /// The button after the last tab, which makes a page and shows its tab.
    pub add: Button,
}

impl ViewStrip {
    /// The action of the tab named `id`.
    pub fn action(&self, id: &ControlId) -> Option<Action> {
        (self.tabs.iter())
            .find(|tab| tab.id == *id)
            .map(|tab| tab.action.clone())
            .or_else(|| (self.add.id == *id).then(|| self.add.action.clone()))
    }
}

fn item_tab(app: &App, entity: &Entity) -> Option<ViewTab> {
    let (label, icon) = match &entity.kind {
        Kind::Page(page) => (page_label(app, entity, page), page_icon(entity.rect.width)),
        Kind::File(file) => (file_label(&file.file), file_icon(&file.file)),
        Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => return None,
    };
    Some(ViewTab {
        id: ControlId::new("view.item").child(entity.id.as_str()),
        label: label.into(),
        icon,
        favicon: (app.page_state(&entity.id)).and_then(|state| state.favicon.clone()),
        active: app.shown_item() == Some(&entity.id),
        action: Action::Show(Showing::Item(entity.id.clone())),
        prepare: matches!(entity.kind, Kind::Page(_))
            .then(|| Action::PrepareShow(Some(entity.id.clone()))),
    })
}

/// The tab row for `app` as it is now. The item tabs keep the order they
/// were first listed in, whatever the stack does.
pub fn view_strip(app: &App) -> ViewStrip {
    let canvas = ViewTab {
        id: ControlId::new("view.canvas"),
        label: "Canvas".into(),
        icon: Icon::File,
        favicon: None,
        active: app.shown_item().is_none(),
        action: Action::Show(Showing::Canvas),
        prepare: None,
    };
    let items = showing::listed(&app.document, &app.session.tabs.order);
    let tabs = std::iter::once(canvas)
        .chain(items.into_iter().filter_map(|entity| item_tab(app, entity)))
        .collect();
    let add = Button {
        id: ControlId::new("view.add"),
        label: "New page tab".into(),
        face: Face::icon(Icon::Plus),
        enabled: true,
        chord: binding_of(&Action::NewPageTab).map(|binding| binding.chord),
        action: Action::NewPageTab,
    };
    ViewStrip { tabs, add }
}
