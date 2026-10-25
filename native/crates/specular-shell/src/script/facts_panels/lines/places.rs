//! The `pages`, `tabs` and `sidebar` topics: where the open items are.

use std::fmt::Write as _;

use specular_interact::panel::{ControlId, view_strip};
use specular_interact::{App, Lens, Mark, NoteState, Showing, SidebarRow, sidebar};

use super::{n, yes};

pub(super) fn pages(app: &App, out: &mut Vec<String>) {
    // Each sync set by its members, which a script can name, not its id.
    let mut sets: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
    for entity in app.document().entities() {
        if let specular_doc::Kind::Page(page) = &entity.kind
            && let Some(set) = &page.sync_id
        {
            sets.entry(set).or_default().push(entity.id.as_str());
        }
    }
    let mut sets: Vec<String> = sets.into_values().map(|pages| pages.join(" ")).collect();
    sets.sort();
    out.extend(sets.into_iter().map(|pages| format!("sync {pages}")));
    for entity in app.document().entities() {
        let specular_doc::Kind::File(file) = &entity.kind else {
            continue;
        };
        let Some(state) = app.note(&file.file) else {
            continue;
        };
        let id = &entity.id;
        let state = match state {
            NoteState::Loading => "loading".to_owned(),
            NoteState::Ready(text) => format!("ready {}", text.replace('\n', "\\n")),
            NoteState::Missing => "missing".to_owned(),
            NoteState::Failed => "failed".to_owned(),
        };
        out.push(format!("{id} note {state}"));
        out.push(format!(
            "{id} note-scroll {}",
            n(f64::from(app.note_scroll(id)))
        ));
    }
    for (id, placement) in app.pages() {
        let viewport = placement.viewport;
        out.push(format!(
            "{id} viewport {}x{}",
            viewport.width, viewport.height
        ));
        let mark = match app.mark(id) {
            Some(Mark::Outline) => "outline",
            Some(Mark::Entered) => "entered",
            None => "none",
        };
        out.push(format!("{id} mark {mark}"));
        out.push(format!("{id} touch {}", yes(app.page_device(id).touch)));
        let Some(state) = app.page_state(id) else {
            continue;
        };
        if let Some(url) = &state.url {
            out.push(format!("{id} url {url}"));
        }
        if !state.title.is_empty() {
            out.push(format!("{id} title {}", state.title));
        }
        out.push(format!("{id} loading {}", yes(state.loading)));
        out.push(format!("{id} back {}", yes(state.can_go_back)));
        out.push(format!("{id} forward {}", yes(state.can_go_forward)));
        out.push(format!(
            "{id} scroll {} {}",
            n(state.scroll.x),
            n(state.scroll.y)
        ));
        out.push(format!("{id} cursor {:?}", state.cursor));
        if let Some(devtools) = &state.devtools_url {
            out.push(format!("{id} devtools {devtools}"));
        }
    }
}

fn sidebar_row(row: &SidebarRow, parent: Option<&ControlId>, out: &mut Vec<String>) {
    let mut line = format!("{} \"{}\" {:?}", row.id, row.label, row.glyph);
    if let Some(trailing) = &row.trailing {
        let _ = write!(line, " \"{trailing}\"");
    }
    for (on, word) in [
        (row.selected, " selected"),
        (row.dimmed, " dimmed"),
        (row.expanded == Some(true), " open"),
        (row.expanded == Some(false), " closed"),
    ] {
        if on {
            line.push_str(word);
        }
    }
    if let Some(parent) = parent {
        let _ = write!(line, " in {parent}");
    }
    out.push(line);
    if row.expanded != Some(false) {
        for child in &row.children {
            sidebar_row(child, Some(&row.id), out);
        }
    }
}

pub(super) fn sidebar_lines(app: &App, out: &mut Vec<String>) {
    let model = sidebar(app);
    out.push(format!("visible {}", yes(model.visible)));
    let head = |head: &specular_interact::SectionHead| {
        let folded = if head.folded { " folded" } else { "" };
        format!("{} \"{}\"{folded}", head.id, head.title)
    };
    out.push(head(&model.canvases_head));
    for canvas in &model.canvases {
        let active = if canvas.active { " active" } else { "" };
        out.push(format!(
            "{} \"{}\" {}{active}",
            canvas.control, canvas.label, canvas.entity_count
        ));
    }
    out.push(head(&model.notes_head));
    for row in &model.notes {
        sidebar_row(row, None, out);
    }
    out.push(head(&model.pages_head));
    for row in &model.pages {
        sidebar_row(row, None, out);
    }
}

pub(super) fn tabs(app: &App, out: &mut Vec<String>) {
    let strip = view_strip(app);
    for tab in &strip.tabs {
        let mut line = format!("{} \"{}\" {:?}", tab.id, tab.label, tab.icon);
        if tab.active {
            line.push_str(" active");
        }
        if tab.close.is_some() {
            line.push_str(" closes");
        }
        if tab.favicon.is_some() {
            line.push_str(" favicon");
        }
        out.push(line);
    }
    out.push(match app.showing() {
        Showing::Canvas => "showing canvas".to_owned(),
        Showing::Item(id) => format!("showing {id}"),
    });
    out.push(match app.lens() {
        Some(Lens::Fill) => "lens fill".to_owned(),
        Some(Lens::Device) => "lens device".to_owned(),
        None => "lens none".to_owned(),
    });
    out.push(format!("others {}", yes(app.shows_others())));
    if let Some(page) = app.prepared_page() {
        out.push(format!("prepared {page}"));
    }
    // What an item view lets be seen beside its item.
    if matches!(app.showing(), Showing::Item(_)) {
        for entity in app.document().entities() {
            if specular_interact::seen(app, entity).is_some() {
                out.push(format!("seen {}", entity.id));
            }
        }
    }
}
