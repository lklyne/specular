//! Select all: every entity that is not inside a group, back-to-front. A
//! group stands for what is in it (ADR 0034).

use specular_doc::ItemId;

use crate::App;

pub(crate) fn run(app: &mut App) {
    let top_level: Vec<ItemId> = (app.document.entities())
        .filter(|entity| entity.parent.is_none())
        .map(|entity| ItemId::Entity(entity.id.clone()))
        .collect();
    app.session.selection.set(top_level);
}
