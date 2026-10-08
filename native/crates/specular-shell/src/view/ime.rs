//! The canvas as a text input client. GPUI's `NSTextInputClient` calls
//! this with marked text and commits, and they go to the app as
//! [`ImeEvent`]s: into the text being edited, or on to the entered page.
//!
//! A plain key press does not come this way. The canvas slot takes it as a
//! key first, so only what an input method composed arrives here.

use std::ops::Range;

use gpui_kit::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, px};
use specular_core::ImeEvent;
use specular_interact::Event;

use super::ShellView;
use crate::canvas;

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

impl EntityInputHandler for ShellView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) -> Option<String> {
        let units: Vec<u16> = self.marked.encode_utf16().collect();
        let end = range.end.min(units.len());
        Some(String::from_utf16_lossy(&units[range.start.min(end)..end]))
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) -> Option<UTF16Selection> {
        let end = utf16_len(&self.marked);
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<'_, Self>) -> Option<Range<usize>> {
        let end = utf16_len(&self.marked);
        (end > 0).then_some(0..end)
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<'_, Self>) {
        self.marked.clear();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) {
        self.marked.clear();
        canvas::dispatch(Event::Ime(ImeEvent::Commit {
            text: text.to_owned(),
            replacement: None,
        }));
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) {
        new_text.clone_into(&mut self.marked);
        if new_text.is_empty() {
            canvas::dispatch(Event::Ime(ImeEvent::Cancel));
            return;
        }
        let end = utf16_len(new_text);
        let selection = new_selected_range.map_or(end..end, |range| range.start..range.end);
        canvas::dispatch(Event::Ime(ImeEvent::SetComposition {
            text: new_text.to_owned(),
            selection: selection.start as u32..selection.end as u32,
            replacement: None,
        }));
    }

    /// Where the caret is, for the candidate window: what the app last said
    /// through `Effect::SetImeCursorArea`, from the slot's corner.
    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) -> Option<Bounds<Pixels>> {
        let (origin, size) = self.asks.ime_area.get();
        Some(Bounds::new(
            element_bounds.origin + gpui_kit::point(px(origin.x), px(origin.y)),
            gpui_kit::size(px(size.x.max(1.0)), px(size.y.max(1.0))),
        ))
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) -> Option<usize> {
        None
    }
}
