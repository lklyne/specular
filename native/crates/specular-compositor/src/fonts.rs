//! [`Fonts`]: the one font system text is shaped with, shared by the scene
//! pass that draws glyphs and the measure that tells the editor where they
//! are.

use std::sync::{Arc, Mutex, PoisonError};

use glyphon::FontSystem;

/// A handle on the font system. Clones share it. The system fonts are
/// loaded by the first use, which takes a moment.
#[derive(Clone, Default)]
pub(crate) struct Fonts(Arc<Mutex<Option<FontSystem>>>);

impl Fonts {
    /// Runs `with` on the font system, loading the system fonts first if
    /// nothing has used them yet.
    pub(crate) fn with<R>(&self, with: impl FnOnce(&mut FontSystem) -> R) -> R {
        // A panic while shaping leaves the font system usable.
        let mut fonts = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        with(fonts.get_or_insert_with(FontSystem::new))
    }
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts").finish_non_exhaustive()
    }
}
