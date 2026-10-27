//! Facts on one entity, and counts.

use anyhow::{bail, ensure};
use specular_doc::EntityId;
use specular_interact::App;

use super::super::expect::{Counted, EntityFact};
use super::super::words::escaped;
use super::{NEAR, equal, kind_word, note_of, or_none, resolve, text_of};

pub(super) fn counted_name(what: &Counted) -> String {
    match what {
        Counted::Entities => "entities".to_owned(),
        Counted::Kind(kind) => format!("{kind} entities"),
        Counted::Edges => "edges".to_owned(),
        Counted::Comments => "comments".to_owned(),
        Counted::Canvases => "canvases".to_owned(),
        Counted::Selected => "selected items".to_owned(),
    }
}

pub(super) fn counted(app: &App, what: &Counted) -> usize {
    let document = app.document();
    match what {
        Counted::Entities => document.entities().count(),
        Counted::Kind(kind) => (document.entities())
            .filter(|entity| kind_word(entity) == kind)
            .count(),
        Counted::Edges => document.edges().count(),
        Counted::Comments => document.annotations().len(),
        Counted::Canvases => app.space().canvases().len(),
        Counted::Selected => app.session().selection.items().len(),
    }
}

fn near(found: &[f64], wanted: &[f64]) -> bool {
    found.iter().zip(wanted).all(|(a, b)| (a - b).abs() <= NEAR)
}

fn numbers(values: &[f64]) -> String {
    let shown: Vec<String> = values.iter().map(|value| format!("{value}")).collect();
    shown.join(" ")
}

pub(super) fn entity_fact(app: &App, name: &str, fact: &EntityFact) -> anyhow::Result<()> {
    let id = resolve(app, name)?;
    let entity = app.document().entity(&EntityId::from(id.as_str()));
    let Some(entity) = entity else {
        ensure!(
            *fact == EntityFact::Gone,
            "there is no entity `{id}`; there are {}",
            (app.document().entities())
                .map(|entity| entity.id.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
        return Ok(());
    };
    let rect = entity.rect;
    let what = |fact: &str| format!("entity {id} {fact}");
    match fact {
        EntityFact::Gone => bail!("entity {id} is still there"),
        EntityFact::Kind(kind) => equal(&what("kind"), kind_word(entity), kind.as_str()),
        EntityFact::Rect(wanted) => {
            let found = [rect.x, rect.y, rect.width, rect.height];
            ensure!(
                near(&found, wanted),
                "{} is {}, expected {}",
                what("rect"),
                numbers(&found),
                numbers(wanted)
            );
            Ok(())
        }
        EntityFact::At(wanted) => {
            let found = [rect.x, rect.y];
            ensure!(
                near(&found, wanted),
                "{} is {}",
                what("at"),
                numbers(&found)
            );
            Ok(())
        }
        EntityFact::Size(wanted) => {
            let found = [rect.width, rect.height];
            ensure!(
                near(&found, wanted),
                "{} is {}",
                what("size"),
                numbers(&found)
            );
            Ok(())
        }
        EntityFact::Text(text) => equal(&what("text"), escaped(text_of(entity)), escaped(text)),
        EntityFact::Note(text) => {
            let found = note_of(app, entity).map(escaped);
            equal(&what("note"), or_none(found), escaped(text))
        }
        EntityFact::Parent(parent) => {
            let found = entity.parent.as_ref().map(ToString::to_string);
            let wanted = parent
                .as_deref()
                .map(|name| resolve(app, name))
                .transpose()?;
            equal(&what("parent"), or_none(found), or_none(wanted))
        }
    }
}
