/**
 * Records whether the keyboard or the pointer last moved focus, as
 * `data-focus-modality` on the root element.
 *
 * Chromium matches `:focus-visible` on text entry even for pointer focus, so
 * the selector alone cannot tell a Tab from a click there and the stylesheet
 * has to ask. Absent the attribute the app reads as pointer-driven, which is
 * how a fresh window with an autofocused field should behave.
 */

/** Keys that move focus. Typing must not count, or clicking into a field and
 *  then typing would light the ring mid-sentence. */
export function keyMovesFocus(key: string): boolean {
  return key === 'Tab' || key.startsWith('Arrow') || key === 'Home' || key === 'End'
}

export function installFocusModality(doc: Document = document): void {
  const set = (mode: 'keyboard' | 'pointer') => {
    doc.documentElement.dataset.focusModality = mode
  }
  // Capture phase: the modality must be recorded before focus lands, since the
  // ring is decided on the focus that this very event is about to move.
  doc.addEventListener('keydown', (e) => {
    if (keyMovesFocus((e as KeyboardEvent).key)) set('keyboard')
  }, true)
  doc.addEventListener('pointerdown', () => set('pointer'), true)
}
