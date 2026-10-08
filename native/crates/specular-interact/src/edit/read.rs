//! What the scene and the shell read about the text being edited.

use std::sync::Arc;

use specular_doc::{EntityId, Rect};

use super::{
    Measurer, StackCache, Target, TextEdit, TextFrame, TextLayout, TextMeasure, editable, frame,
    geometry, layout_of,
};
use crate::App;

/// What the scene draws for the text being edited. Rects are in canvas
/// space.
impl App {
    /// Installs the measure that lays text out for the editor. The shell
    /// gives one that shapes with the renderer's fonts.
    pub fn set_text_measure(&mut self, measure: Arc<dyn TextMeasure>) {
        self.measure = Measurer(measure);
        self.stacks = StackCache::default();
        crate::panel::builtin::forget_layout(self);
    }

    /// The measure the editor lays text out with.
    pub fn text_measure(&self) -> &dyn TextMeasure {
        self.measure.0.as_ref()
    }

    /// The edit session, if text is being edited.
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.session.editing.as_ref()
    }

    /// The text to draw for `id` in place of the document's, when `id` is
    /// the entity being edited.
    pub fn editing_text(&self, id: &EntityId) -> Option<&str> {
        let edit = self.session.editing.as_ref()?;
        (edit.target.is_entity() && edit.entity == *id).then_some(edit.text.as_str())
    }

    /// Where `id`'s text is laid out and how it is set, for a text, a
    /// sticky, a shape, or a Document whose file has been read.
    pub fn text_frame(&self, id: &EntityId) -> Option<TextFrame> {
        let entity = self.document.entity(id)?;
        match editable(self, entity)? {
            (Target::Note, _) => Some(frame::note_frame(
                entity.rect,
                self.session.notes.scroll(id),
            )),
            // A title sits outside the body; `edit_frame` places it.
            (Target::Title | Target::EdgeLabel | Target::Comment | Target::Field, _) => None,
            (Target::Text | Target::Label, _) => frame::of(entity),
        }
    }

    /// The layout of the text being edited, as it stands.
    pub fn editing_layout(&self) -> Option<Arc<TextLayout>> {
        layout_of(self, self.session.editing.as_ref()?)
    }

    /// The caret: a rect with no width, one line tall. It is there whether
    /// or not anything is selected.
    pub fn caret_rect(&self) -> Option<Rect> {
        let edit = self.session.editing.as_ref()?;
        let (frame, layout) = geometry(self, edit)?;
        let caret = layout.caret_box(edit.caret)?;
        Some(frame.rect_of(&layout, caret))
    }

    /// The selected text, one rect per line it touches. Empty when nothing
    /// is selected.
    pub fn selection_rects(&self) -> Vec<Rect> {
        self.range_rects(TextEdit::selection)
    }

    /// The text the input method is composing, one rect per line it
    /// touches, to underline.
    pub fn composition_rects(&self) -> Vec<Rect> {
        self.range_rects(|edit| edit.composition().unwrap_or_default())
    }

    fn range_rects(&self, range: impl FnOnce(&TextEdit) -> std::ops::Range<usize>) -> Vec<Rect> {
        let Some(edit) = &self.session.editing else {
            return Vec::new();
        };
        let Some((frame, layout)) = geometry(self, edit) else {
            return Vec::new();
        };
        let boxes = layout.range_boxes(&range(edit));
        (boxes.into_iter())
            .map(|line| frame.rect_of(&layout, line))
            .collect()
    }
}
