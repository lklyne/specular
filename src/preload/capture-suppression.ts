export const CAPTURE_SUPPRESSION_STYLE_ID = '__canvas-capture-suppression'

// Prefix-based so any future overlay following the __canvas- id convention is
// excluded from captures without having to be re-listed here. Matches the same
// class-of-node treatment in gesture-forwarding.ts.
export const CAPTURE_SUPPRESSION_CSS = `
[id^="__canvas-dom-inspection-"],
[id^="__canvas-comment-"] {
  display: none !important;
  visibility: hidden !important;
}
`
