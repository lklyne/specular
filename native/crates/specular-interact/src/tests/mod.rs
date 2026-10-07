//! Routing tests: scripted events through [`update`] with the local helpers
//! below, asserting on the session and the returned effects. Gesture and
//! feature tests are under `tests/`, on `specular-testkit`.

mod routing;

use glam::Vec2;
use specular_core::{Camera, Modifiers, PointerButton, PointerEventKind};
use specular_doc::{Command, Document, Entity, EntityId, ItemId, Kind, Page, Rect};

use crate::{Action, App, Effect, Event, Key, KeyInput, PointerInput, Tool, update};

const ALT: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: true,
    meta: false,
};
const CMD: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: true,
};

fn page(id: &str, rect: Rect) -> Entity {
    let page = Page {
        url: format!("https://example.com/{id}"),
        ..Page::default()
    };
    Entity::new(id, rect, Kind::Page(page))
}

fn document(entities: Vec<Entity>) -> Document {
    let mut document = Document::new();
    for entity in entities {
        let at = document.stack_len();
        document
            .apply(Command::InsertEntity {
                entity: Box::new(entity),
                at,
            })
            .unwrap();
    }
    document
}

/// Two 400x300 pages side by side, `p1` at (100, 100) and `p2` at (700, 100).
fn two_pages() -> Vec<Entity> {
    vec![
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        page("p2", Rect::new(700.0, 100.0, 400.0, 300.0)),
    ]
}

/// An app showing `entities` at zoom 1 with no pan.
fn app_with(entities: Vec<Entity>) -> App {
    let mut app = App::new(0);
    update(
        &mut app,
        Event::DocumentOpened(Box::new(document(entities))),
    );
    app
}

fn app() -> App {
    app_with(two_pages())
}

fn id(id: &str) -> EntityId {
    EntityId::from(id)
}

fn selected(app: &App) -> Option<EntityId> {
    app.session().selection.single_entity().cloned()
}

fn act(app: &mut App, action: Action) -> Vec<Effect> {
    update(app, Event::Action(action))
}

fn select(app: &mut App, entity: &str) {
    act(app, Action::Select(vec![ItemId::Entity(id(entity))]));
}

fn set_zoom(app: &mut App, zoom: f32) {
    act(app, Action::SetCamera(Camera::new(Vec2::ZERO, zoom)));
}

fn pointer(app: &mut App, kind: PointerEventKind, at: (f32, f32), mods: Modifiers) -> Vec<Effect> {
    update(
        app,
        Event::Pointer(PointerInput {
            kind,
            screen: Vec2::new(at.0, at.1),
            modifiers: mods,
        }),
    )
}

fn press_with(app: &mut App, at: (f32, f32), mods: Modifiers) -> Vec<Effect> {
    let kind = PointerEventKind::Down {
        button: PointerButton::Left,
        click_count: 1,
    };
    pointer(app, kind, at, mods)
}

fn press(app: &mut App, at: (f32, f32)) -> Vec<Effect> {
    press_with(app, at, Modifiers::default())
}

fn drag_to(app: &mut App, at: (f32, f32)) -> Vec<Effect> {
    pointer(app, PointerEventKind::Move, at, Modifiers::default())
}

fn release(app: &mut App, at: (f32, f32)) -> Vec<Effect> {
    let kind = PointerEventKind::Up {
        button: PointerButton::Left,
        click_count: 1,
    };
    pointer(app, kind, at, Modifiers::default())
}

fn key_with(app: &mut App, key: Key, pressed: bool, mods: Modifiers) -> Vec<Effect> {
    let text = match key {
        Key::Char(character) if pressed => Some(character.to_string()),
        _ => None,
    };
    update(
        app,
        Event::Key(KeyInput {
            key,
            pressed,
            repeat: false,
            text,
            modifiers: mods,
            windows_key_code: 0,
            native_key_code: 0,
        }),
    )
}

fn key(app: &mut App, key: Key) -> Vec<Effect> {
    key_with(app, key, true, Modifiers::default())
}

/// An app with the comment tool armed.
fn armed() -> App {
    let mut app = app();
    key(&mut app, Key::Char('c'));
    assert_eq!(app.session().tool, Tool::Comment);
    app
}
