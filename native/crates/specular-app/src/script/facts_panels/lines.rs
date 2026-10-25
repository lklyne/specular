//! Each topic as lines of text: what an `expect` on it reads.

use std::fmt::Write as _;

use specular_doc::{AnnotationAnchor, AnnotationId, Rect, RegionAnchor};
use specular_interact::panel::{
    Control, ControlId, DropdownSection, MenuTarget, ToolbarSection, context_menu, dock, toolbar,
    view_strip,
};
use specular_interact::{
    App, ArtifactCard, Chord, Effect, Lens, Mark, MarkShape, MenuEntry, NoteState, PaletteRun,
    ScreenRect, Showing, SidebarRow, ThreadRole, chat, menus, onboarding, palette, repos_pane,
    settings, sidebar,
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
    let [x, y, width, height] = [rect.min.x, rect.min.y, rect.size.x, rect.size.y];
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

fn anchor(anchor: &AnnotationAnchor) -> String {
    match anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            format!("canvas-point {} {}", n(*canvas_x), n(*canvas_y))
        }
        AnnotationAnchor::Page {
            page_id,
            offset_x,
            offset_y,
        } => format!("page-point {page_id} {} {}", n(*offset_x), n(*offset_y)),
        AnnotationAnchor::Element {
            page_id, selector, ..
        } => format!("element {page_id} {selector}"),
        AnnotationAnchor::Region(RegionAnchor::Canvas { canvas_rect }) => {
            format!("canvas-region {}", rect(*canvas_rect))
        }
        AnnotationAnchor::Region(RegionAnchor::Document { doc_rect }) => {
            format!("page-region {}", rect(*doc_rect))
        }
    }
}

fn comments(app: &App, out: &mut Vec<String>) {
    for mark in app.comment_marks() {
        let (shape, at) = match mark.shape {
            MarkShape::Badge(pill) => ("pill", screen(pill)),
            MarkShape::Region(frame) => ("region", screen(frame)),
        };
        let mut line = format!("mark {} {shape} {at}", mark.annotation);
        if mark.focused {
            line.push_str(" focused");
        }
        if mark.count > 1 {
            let _ = write!(line, " count {}", mark.count);
        }
        if mark.members.len() > 1 {
            let members: Vec<&str> = mark.members.iter().map(AnnotationId::as_str).collect();
            let _ = write!(line, " members {}", members.join(","));
        }
        out.push(line);
    }
    if let Some(focused) = app.focused_comment() {
        out.push(format!("focus {focused}"));
    }
    if let Some(draft) = app.comment_draft() {
        out.push(format!("draft {}", anchor(&draft.anchor)));
        let page = (draft.page_anchor.as_ref()).map_or("none", |page| page.page_id.as_str());
        out.push(format!("draft page {page}"));
    }
    if let Some(card) = app.comment_composer() {
        out.push(format!("composer {}", rect(card)));
    }
    if let Some(preview) = app.session().comment_preview() {
        out.push(format!("preview {}", rect(preview)));
    }
    for outline in app.comment_highlights() {
        out.push(format!("highlight {}", screen(outline)));
    }
    for annotation in app.document().annotations() {
        let status = format!("{:?}", annotation.status).to_lowercase();
        let mut line = format!(
            "comment {} {status} {} \"{}\"",
            annotation.id,
            anchor(&annotation.anchor),
            annotation.text
        );
        let metadata = annotation.metadata.as_ref();
        if let Some(thread) = metadata
            .and_then(|metadata| metadata.get("threadId"))
            .and_then(|id| id.as_str())
        {
            let _ = write!(line, " thread {thread}");
        }
        if let Some(page) = &annotation.page_anchor {
            let _ = write!(line, " on {}", page.page_id);
        }
        out.push(line);
        if let Some(metadata) = metadata {
            let text = serde_json::to_string(metadata).unwrap_or_default();
            out.push(format!("comment {} metadata {text}", annotation.id));
        }
    }
}

fn cards(cards: &[ArtifactCard], out: &mut Vec<String>) {
    for card in cards {
        let verb = if card.created { "new" } else { "edited" };
        let live = if card.focus.is_some() { " live" } else { "" };
        out.push(format!(
            "card \"{}\" {} {verb}{live}",
            card.label, card.detail
        ));
    }
}

fn chat_lines(app: &App, out: &mut Vec<String>) {
    let model = chat(app);
    out.push(format!(
        "chat available={} visible={} width={} title=\"{}\"",
        yes(model.available),
        yes(model.visible),
        model.width,
        model.title
    ));
    for row in &model.threads {
        let mut line = format!("thread \"{}\"", row.title);
        for (on, word) in [
            (row.active, " active"),
            (row.draft, " draft"),
            (row.running, " running"),
        ] {
            if on {
                line.push_str(word);
            }
        }
        out.push(line);
    }
    out.push(format!(
        "controls back={} close={}",
        yes(model.back.is_some()),
        yes(model.close.is_some())
    ));
    match &model.transcript {
        Some(transcript) => transcript_lines(transcript, out),
        None => out.push("list".to_owned()),
    }
    let composer = &model.composer;
    if let Some(draft) = &composer.draft {
        out.push(format!("draft {:?} \"{}\"", draft.kind, draft.label));
    }
    if let Some(open) = &composer.open_comments {
        out.push(format!("open-comments {}", open.count));
    }
    for chip in &composer.queued {
        let pinned = if chip.annotation.is_some() {
            " pinned"
        } else {
            ""
        };
        out.push(format!("queued \"{}\"{pinned}", chip.text));
    }
    out.push(format!("placeholder \"{}\"", composer.placeholder));
    out.push(format!("send-empty {}", yes(composer.can_send_empty)));
    out.push(format!("running {}", yes(composer.running)));
    out.push(format!(
        "pill {:?} \"{}\"",
        composer.pill.kind, composer.pill.label
    ));
    if let Some(folder) = &composer.folder {
        out.push(format!("folder {folder}"));
    }
    if let Some(auto) = &composer.auto {
        out.push(format!("auto {} {}", auto.origin, yes(auto.on)));
    }
    out.push(format!("model {}", composer.model.label));
    for option in &composer.model.options {
        let chosen = if option.chosen { " chosen" } else { "" };
        out.push(format!("model-option \"{}\"{chosen}", option.name));
    }
    if let Some(button) = toolbar(app).chat {
        let open = if button.open { "open" } else { "closed" };
        out.push(format!("toggle {open} \"{}\"", button.label));
    }
    for thread in app.threads().all() {
        let status = format!("{:?}", thread.status).to_lowercase();
        let session = thread.claude_session_id.as_deref().unwrap_or("none");
        out.push(format!(
            "stored \"{}\" {status} messages {} session {session}",
            thread.title,
            thread.messages.len()
        ));
    }
}

fn transcript_lines(transcript: &specular_interact::Transcript, out: &mut Vec<String>) {
    for bubble in &transcript.messages {
        let role = match bubble.role {
            ThreadRole::User => "user",
            ThreadRole::Agent => "agent",
        };
        let mut line = format!("bubble {role} \"{}\"", bubble.text);
        if bubble.annotation.is_some() {
            line.push_str(" pinned");
        }
        if bubble.focused {
            line.push_str(" focused");
        }
        if bubble.focus.is_some() {
            line.push_str(" live");
        }
        out.push(line);
        for image in &bubble.images {
            out.push(format!("image {image}"));
        }
        cards(&bubble.artifacts, out);
    }
    if let Some(text) = &transcript.streaming {
        out.push(format!("streaming \"{text}\""));
    }
    cards(&transcript.making, out);
    if let Some(run) = &transcript.run {
        out.push(format!("run \"{}\"", run.label));
        for line in &run.log {
            out.push(format!("log \"{line}\""));
        }
    }
    if let Some(error) = &transcript.error {
        out.push(format!("error \"{error}\""));
    }
    if let Some(hint) = &transcript.empty_hint {
        out.push(format!("hint \"{hint}\""));
    }
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

fn pages(app: &App, out: &mut Vec<String>) {
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

fn sidebar_lines(app: &App, out: &mut Vec<String>) {
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

fn tabs(app: &App, out: &mut Vec<String>) {
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

/// A control's line: its name, `on` when it is on or chosen, `disabled`,
/// `= value` for what it shows, and its label in quotes.
fn line(id: &ControlId, on: bool, enabled: bool, value: Option<&str>, label: &str) -> String {
    let mut line = id.as_str().to_owned();
    if on {
        line.push_str(" on");
    }
    if !enabled {
        line.push_str(" disabled");
    }
    if let Some(value) = value {
        let _ = write!(line, " = {value}");
    }
    let _ = write!(line, " \"{label}\"");
    line
}

fn sections(content: &[DropdownSection], out: &mut Vec<String>) {
    for section in content {
        match section {
            DropdownSection::Options { options, .. } => {
                for option in options {
                    let value = option.trailing.as_deref();
                    let label = &option.label;
                    out.push(line(
                        &option.id,
                        option.selected,
                        option.enabled,
                        value,
                        label,
                    ));
                    for field in &option.fields {
                        out.push(line(
                            &field.id,
                            false,
                            true,
                            Some(&field.value),
                            &field.label,
                        ));
                    }
                }
            }
            DropdownSection::Controls(controls) => {
                for control in controls {
                    control_lines(control, out);
                }
            }
        }
    }
}

fn control_lines(control: &Control, out: &mut Vec<String>) {
    match control {
        Control::Button(button) => {
            out.push(line(&button.id, false, button.enabled, None, &button.label));
        }
        Control::Toggle(toggle) => {
            out.push(line(
                &toggle.id,
                toggle.on,
                toggle.enabled,
                None,
                &toggle.label,
            ));
        }
        Control::Swatches(row) => {
            for swatch in &row.options {
                out.push(line(
                    &swatch.id,
                    swatch.selected,
                    row.enabled,
                    None,
                    &swatch.label,
                ));
            }
        }
        Control::Dropdown(dropdown) => {
            let value = dropdown.summary.text.as_deref();
            out.push(line(&dropdown.id, false, true, value, &dropdown.label));
            sections(&dropdown.content, out);
        }
        Control::Stepper(stepper) => {
            out.push(line(
                &stepper.id,
                false,
                true,
                Some(&stepper.value),
                &stepper.label,
            ));
            let halves = [
                ("dec", stepper.can_decrement),
                ("inc", stepper.can_increment),
            ];
            for (part, enabled) in halves {
                out.push(line(&stepper.id.child(part), false, enabled, None, part));
            }
        }
        Control::Field(field) => {
            out.push(line(
                &field.id,
                false,
                true,
                Some(&field.value),
                &field.label,
            ));
        }
        Control::Choices(choices) => sections(&choices.content, out),
        Control::Separator => {}
    }
}

fn controls(app: &App, out: &mut Vec<String>) {
    let strip = view_strip(app);
    for tab in &strip.tabs {
        out.push(line(&tab.id, tab.active, true, None, &tab.label));
    }
    out.push(line(
        &strip.add.id,
        false,
        strip.add.enabled,
        None,
        &strip.add.label,
    ));
    let bar = toolbar(app);
    if let Some(button) = &bar.chat {
        out.push(line(&button.id, button.open, true, None, &button.label));
    }
    out.push(line(&bar.theme.id, false, true, None, &bar.theme.label));
    for section in &bar.sections {
        match section {
            ToolbarSection::Tools(tools) => {
                for tool in tools {
                    out.push(line(&tool.id, tool.active, true, None, &tool.label));
                }
            }
            ToolbarSection::Zoom(zoom) => control_lines(&Control::Dropdown(zoom.clone()), out),
        }
    }
    for control in &bar.view {
        control_lines(control, out);
    }
    for control in dock(app).iter().flat_map(|dock| &dock.controls) {
        control_lines(control, out);
    }
    if let Some(open) = &app.session().panel.menu {
        out.push(format!("menu {:?}", open.target));
        let target: &MenuTarget = &open.target;
        for control in context_menu(app, target)
            .iter()
            .flat_map(|menu| &menu.controls)
        {
            control_lines(control, out);
        }
    }
}
