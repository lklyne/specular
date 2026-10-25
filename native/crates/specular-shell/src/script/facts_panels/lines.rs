//! Each topic as lines of text: what an `expect` on it reads.

mod agent;
mod controls;
mod places;

use self::agent::{chat_lines, comments};
use self::controls::controls;
use self::places::{pages, sidebar_lines, tabs};

use std::fmt::Write as _;

use specular_doc::Rect;
use specular_interact::{
    App, Chord, Effect, MenuEntry, PaletteRun, ScreenRect, menus, onboarding, palette, repos_pane,
    settings,
};

use super::Topic;

/// The lines of `topic` now. The effects are kept by the runner, not here.
pub(super) fn lines(app: &App, topic: Topic) -> Vec<String> {
    let mut out = Vec::new();
    match topic {
        Topic::Chat => chat_lines(app, &mut out),
        Topic::Comments => comments(app, &mut out),
        Topic::Controls => controls(app, &mut out),
        Topic::Editor => editor(app, &mut out),
        Topic::Effects => {}
        Topic::Inspect => inspect(app, &mut out),
        Topic::Menubar => menubar(app, &mut out),
        Topic::Onboarding => first_run(app, &mut out),
        Topic::Pages => pages(app, &mut out),
        Topic::Palette => palette_lines(app, &mut out),
        Topic::Pointer => {
            if let Some(hover) = &app.session().hover {
                out.push(format!("hover {hover}"));
            }
        }
        Topic::Repos => repos(app, &mut out),
        Topic::Settings => settings_lines(app, &mut out),
        Topic::Sidebar => sidebar_lines(app, &mut out),
        Topic::Tabs => tabs(app, &mut out),
    }
    out
}

/// A number rounded to a tenth, written without a needless `.0`.
fn n(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    format!("{}", rounded + 0.0)
}

fn rect(rect: Rect) -> String {
    format!(
        "{} {} {} {}",
        n(rect.x),
        n(rect.y),
        n(rect.width),
        n(rect.height)
    )
}

fn screen(rect: ScreenRect) -> String {
    let ScreenRect {
        x,
        y,
        width,
        height,
    } = rect;
    format!(
        "{} {} {} {}",
        n(f64::from(x)),
        n(f64::from(y)),
        n(f64::from(width)),
        n(f64::from(height))
    )
}

fn yes(on: bool) -> &'static str {
    if on { "yes" } else { "no" }
}

fn chord(chord: Option<Chord>) -> String {
    chord.map_or_else(String::new, |chord| format!(" {}", chord.text()))
}

/// An effect as lines: its name in kebab case and its fields, and for a run
/// of the agent a line for each part of its request.
pub(super) fn effect(effect: &Effect, out: &mut Vec<String>) {
    if let Effect::RunAgent(request) = effect {
        out.push("run-agent".to_owned());
        let resume = request.resume.as_deref().unwrap_or("none");
        out.push(format!("run-agent resume {resume}"));
        let cwd = request.cwd.as_deref().unwrap_or("none");
        out.push(format!("run-agent cwd {cwd}"));
        let model = request.model.as_deref().unwrap_or("none");
        out.push(format!("run-agent model {model}"));
        out.push(format!("run-agent images {}", request.images.len()));
        for image in &request.images {
            out.push(format!("run-agent image {}", image.path));
        }
        for line in request.prompt.lines() {
            out.push(format!("run-agent prompt {line}"));
        }
        return;
    }
    let debug = format!("{effect:?}");
    let end = debug.find([' ', '(']).unwrap_or(debug.len());
    let (name, rest) = debug.split_at(end);
    let mut kebab = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            kebab.push('-');
        }
        kebab.extend(character.to_lowercase());
    }
    out.push(format!("{kebab}{rest}"));
}

fn editor(app: &App, out: &mut Vec<String>) {
    let Some(edit) = app.text_edit() else {
        return;
    };
    out.push(format!("caret {} {}", edit.caret(), edit.anchor()));
    if let Some(composition) = edit.composition() {
        out.push(format!(
            "composing {} {}",
            composition.start, composition.end
        ));
    }
    if edit.is_field() {
        out.push(format!("field {}", edit.entity()));
    }
    if let Some(caret) = app.caret_rect() {
        out.push(format!("caret-rect {}", rect(caret)));
    }
}

fn inspect(app: &App, out: &mut Vec<String>) {
    if let Some(model) = app.inspect() {
        out.push(format!("outline {}", screen(model.outline)));
        let popover = model.popover;
        out.push(format!("popover {}", screen(popover.rect)));
        out.push(format!("tag {}", popover.tag));
        out.push(format!("remainder {}", popover.remainder));
        out.push(format!("size {}", popover.size));
        if let Some(font) = popover.font {
            out.push(format!("font {} {}", font.family, font.detail));
        }
        for swatch in popover.swatches {
            out.push(format!("swatch {} {}", swatch.label, swatch.value));
        }
    }
    if let Some(picked) = app.inspected() {
        out.push(format!("picked {}", picked.page));
    }
    for outline in app.comment_highlights() {
        out.push(format!("highlight {}", screen(outline)));
    }
}

fn menubar(app: &App, out: &mut Vec<String>) {
    for menu in menus(app) {
        for entry in menu.entries {
            let MenuEntry::Item(item) = entry else {
                continue;
            };
            let mut line = format!("{} > {}", menu.title, item.label);
            if !item.enabled {
                line.push_str(" disabled");
            }
            if item.checked == Some(true) {
                line.push_str(" checked");
            }
            line.push_str(&chord(item.chord));
            out.push(line);
        }
    }
}

fn first_run(app: &App, out: &mut Vec<String>) {
    let Some(view) = onboarding(app) else {
        return;
    };
    out.push(format!("title {}", view.title));
    for line in &view.body {
        out.push(format!("body {line}"));
    }
    for choice in &view.choices {
        let primary = if choice.primary { " primary" } else { "" };
        out.push(format!(
            "choice {} \"{}\" \"{}\"{primary}",
            choice.name, choice.label, choice.detail
        ));
    }
    if let Some(note) = &view.note {
        out.push(format!("note {note}"));
    }
}

fn palette_lines(app: &App, out: &mut Vec<String>) {
    for item in palette(app) {
        let mut line = format!("{} > {}", item.group, item.label);
        if !item.enabled {
            line.push_str(" disabled");
        }
        if item.checked == Some(true) {
            line.push_str(" checked");
        }
        line.push_str(&chord(item.chord));
        if let PaletteRun::Shell(command) = &item.run {
            let _ = write!(line, " shell {command:?}");
        }
        out.push(line);
    }
}

fn repos(app: &App, out: &mut Vec<String>) {
    for repo in repos_pane(app).repos {
        out.push(format!("repo {} {}", repo.label, repo.path));
        for origin in repo.origins {
            out.push(format!(
                "origin {} auto {}",
                origin.origin,
                yes(origin.auto_fix)
            ));
        }
    }
}

fn settings_lines(app: &App, out: &mut Vec<String>) {
    let model = settings(app);
    if let Some(space) = &model.general.space {
        out.push(format!("space {} {}", space.name, space.path));
    }
    let toggles = model.general.toggles.iter().chain(&model.general.cursors);
    for toggle in toggles {
        out.push(format!(
            "{} {} \"{}\"",
            toggle.name,
            yes(toggle.on),
            toggle.label
        ));
    }
    for option in &model.general.tools.options {
        out.push(format!(
            "{} {} \"{}\"",
            option.name,
            yes(option.on),
            option.label
        ));
    }
    for row in &model.shortcuts {
        out.push(format!("shortcut {} {} {}", row.keys, row.label, row.place));
    }
    for row in &model.about {
        out.push(format!("about {} {}", row.name, row.version));
    }
}
