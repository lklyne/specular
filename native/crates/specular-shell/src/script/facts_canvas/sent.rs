//! The `sent` facts: the effects of the last step, as words.

use anyhow::ensure;
use specular_core::{InputEvent, KeyEventKind, PointerEventKind, TouchPhase};
use specular_interact::{App, Effect};

use super::read::{kebab, words_of};
use super::{resolved, same_words};
use crate::script::CanvasFact;

/// Whether a `sent` fact holds for the effects of the last step.
pub(crate) fn check_sent(app: &App, sent: &[Effect], fact: &CanvasFact) -> anyhow::Result<()> {
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
            let words = &resolved(app, words)?;
            // Past the effect's name, the last word may be the start of the
            // effect's: a clipboard write is one long word.
            let starts = |effect: &Vec<String>| {
                let Some((last, before)) = words.split_last() else {
                    return true;
                };
                effect.len() >= words.len()
                    && same_words(&effect[..before.len()], before)
                    && (same_words(
                        &effect[before.len()..=before.len()],
                        std::slice::from_ref(last),
                    ) || (!before.is_empty()
                        && effect[before.len()].starts_with(last.as_str())))
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

/// An effect as the words a `sent` line matches.
fn effect_words(effect: &Effect) -> Vec<String> {
    let text = match effect {
        Effect::ForwardInput { page, event } => format!("forward {page} {}", input_words(event)),
        Effect::CreatePage {
            page,
            url,
            viewport,
        } => {
            format!(
                "create-page {page} {url} {} {}",
                viewport.width, viewport.height
            )
        }
        Effect::ClosePage(page) => format!("close-page {page}"),
        Effect::SetPageViewport { page, viewport } => {
            format!("viewport {page} {} {}", viewport.width, viewport.height)
        }
        Effect::SetCursor(cursor) => format!("cursor {}", kebab(&format!("{cursor:?}"))),
        Effect::FocusPage(page) => {
            format!(
                "focus-page {}",
                page.as_ref().map_or("none".to_owned(), ToString::to_string)
            )
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
