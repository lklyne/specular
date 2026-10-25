//! The binding table itself: no key fires two rows, and an entered page
//! keeps every key but the ones a browser's chrome keeps. What the keys do
//! is in the scenarios (`tools.txt`, `moves.txt`).

use specular_interact::{BINDINGS, Chord, SHELL_KEYS, kept_from_page};

#[test]
fn no_two_rows_can_fire_for_the_same_key() {
    for (index, binding) in BINDINGS.iter().enumerate() {
        // The bracket keys restack on the canvas and walk the history of an
        // entered page: one key, two rows, never both at once.
        let shadowed = BINDINGS[..index].iter().any(|earlier| {
            earlier.chord == binding.chord && earlier.context.overlaps(binding.context)
        });
        assert!(!shadowed, "{:?} is bound twice", binding.chord);
    }
}

#[test]
fn an_entered_page_has_every_key_but_the_ones_a_browsers_chrome_keeps() {
    // Zoom to fit and Escape are the ways back out; then a browser's history,
    // reload, stop and address; the frame, turn and color scheme it is
    // shown in; a browser's tabs; then the window's own. Every other
    // key is the page's, undo, copy and select all included.
    let kept: Vec<String> = kept_from_page().map(Chord::text).collect();
    assert_eq!(
        kept,
        [
            "⌘1",
            "⌘[",
            "⌘]",
            "⌘R",
            "⌘.",
            "⌘L",
            "⌥⌘F",
            "⌥⇧⌘F",
            "⌥⌘R",
            "⌥⌘L",
            "⌘T",
            "⌘W",
            "⌥⌘→",
            "⌥⌘←",
            "⎋",
            "⌘Q",
            "⇧⌘O",
            "⌘O",
            "⌘S",
            "⌘,",
            "⌘H",
            "⌥⌘H",
            "⌘M",
            "⌘K"
        ]
    );
    // The window's keys fire wherever the keys are, so none is a row's.
    for (chord, command) in SHELL_KEYS {
        let taken = BINDINGS.iter().any(|binding| binding.chord == *chord);
        assert!(!taken, "{command:?} is on a key the table binds");
    }
}
