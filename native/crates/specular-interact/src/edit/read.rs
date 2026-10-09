//! What the scene and the shell read about the text being edited.

use std::sync::Arc;

use specular_doc::{EntityId, Rect};

use super::{
    Measurer, StackCache, Target, TextEdit, TextFrame, TextLayout, TextMeasure, editable, frame,
    geometry, geometry_in, layout_of,
};
use crate::App;

/// What is drawn with the text being edited besides its glyphs. Rects are
/// in canvas space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EditMarks {
    /// The selected text, one rect per line it touches.
    pub selection: Vec<Rect>,
    /// The text the input method is composing, one rect per line, to
    /// underline.
    pub composition: Vec<Rect>,
    /// The caret: a rect with no width, one line tall.
    pub caret: Option<Rect>,
}

/// The rect of each line `range` touches.
fn range_rects(
    frame: &TextFrame,
    layout: &TextLayout,
    range: &std::ops::Range<usize>,
) -> Vec<Rect> {
    (layout.range_boxes(range).into_iter())
        .map(|line| frame.rect_of(layout, line))
        .collect()
}

/// What the scene draws for the text being edited. Rects are in canvas
/// space.
impl App {
    /// Installs the measure that lays text out for the editor. The shell
    /// gives one that shapes with the renderer's fonts.
    pub fn set_text_measure(&mut self, measure: Arc<dyn TextMeasure>) {
        self.measure = Measurer(measure);
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
                crate::scroll_follow::placed_rect(self, entity),
                self.session.notes.scroll(id),
            )),
            // A title sits outside the body; `edit_frame` places it.
            (Target::Title | Target::EdgeLabel | Target::Comment | Target::Field, _) => None,
            (Target::Text | Target::Label, _) => {
                frame::of(entity, crate::scroll_follow::placed_rect(self, entity))
            }
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
        self.edit_marks(&StackCache::default()).selection
    }

    /// The text the input method is composing, one rect per line it
    /// touches, to underline.
    pub fn composition_rects(&self) -> Vec<Rect> {
        self.edit_marks(&StackCache::default()).composition
    }

    /// The selection, the composition and the caret of the text being
    /// edited, for a frame. A Document's layout is looked up in `stacks`,
    /// which the caller keeps from one frame to the next.
    pub fn edit_marks(&self, stacks: &StackCache) -> EditMarks {
        let Some(edit) = &self.session.editing else {
            return EditMarks::default();
        };
        let Some((frame, layout)) = geometry_in(self, edit, stacks) else {
            return EditMarks::default();
        };
        EditMarks {
            selection: range_rects(&frame, &layout, &edit.selection()),
            composition: range_rects(&frame, &layout, &edit.composition().unwrap_or_default()),
            caret: (layout.caret_box(edit.caret)).map(|caret| frame.rect_of(&layout, caret)),
        }
    }
}
