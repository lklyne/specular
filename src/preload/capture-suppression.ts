export const CAPTURE_SUPPRESSION_STYLE_ID = '__canvas-capture-suppression'

export const CAPTURE_SUPPRESSION_CSS = `
#__canvas-dom-inspection-highlight,
#__canvas-dom-inspection-pinned-highlight,
#__canvas-dom-inspection-label,
[id^="__canvas-dom-inspection-margin-"],
[id^="__canvas-dom-inspection-padding-"],
[id^="__canvas-dom-inspection-gap-"],
#__canvas-comment-preview-layer,
#__canvas-comment-badges-layer,
#__canvas-comment-hover-highlight,
#__canvas-comment-hover-summary {
  display: none !important;
  visibility: hidden !important;
}
`
