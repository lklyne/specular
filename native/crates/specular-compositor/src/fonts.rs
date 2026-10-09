//! [`Fonts`]: the one font system text is shaped with, shared by the scene
//! pass that draws glyphs and the measure that tells the editor where they
//! are.

use std::sync::{Arc, Mutex, PoisonError};

use glyphon::FontSystem;

/// The family sans-serif text is set in.
pub const SANS_FAMILY: &str = "Inter";
/// The family monospace text is set in.
pub const MONO_FAMILY: &str = "Geist Mono";

/// The font files the app ships, so text looks the same on a Mac that has
/// none of them installed: Inter, Geist Mono and Kalam (the hand font).
const BUNDLED: [&[u8]; 10] = [
    include_bytes!("../fonts/Inter-Regular.ttf"),
    include_bytes!("../fonts/Inter-Medium.ttf"),
    include_bytes!("../fonts/Inter-SemiBold.ttf"),
    include_bytes!("../fonts/Inter-Bold.ttf"),
    include_bytes!("../fonts/Inter-Italic.ttf"),
    include_bytes!("../fonts/Inter-BoldItalic.ttf"),
    include_bytes!("../fonts/GeistMono-Regular.ttf"),
    include_bytes!("../fonts/GeistMono-Bold.ttf"),
    include_bytes!("../fonts/Kalam-Regular.ttf"),
    include_bytes!("../fonts/Kalam-Bold.ttf"),
];

/// The bundled font files, for a text system other than the canvas's own
/// (the shell's panels) to load.
pub fn bundled_fonts() -> &'static [&'static [u8]] {
    &BUNDLED
}

/// The system fonts plus the bundled ones, with the bundled families as
/// what generic sans-serif and monospace mean.
pub(crate) fn font_system() -> FontSystem {
    let mut fonts = FontSystem::new();
    let db = fonts.db_mut();
    for font in BUNDLED {
        db.load_font_data(font.to_vec());
    }
    db.set_sans_serif_family(SANS_FAMILY);
    db.set_monospace_family(MONO_FAMILY);
    fonts
}

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
        with(fonts.get_or_insert_with(font_system))
    }
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts").finish_non_exhaustive()
    }
}
