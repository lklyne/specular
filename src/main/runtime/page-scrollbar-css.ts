/**
 * Thin, theme-aware scrollbar CSS for page guest content.
 *
 * Device emulation (see layout-engine.ts) makes Blink paint root scrollbars
 * from the page's declared color-scheme only, so a page that never declares
 * `color-scheme` (most sites) gets thick light scrollbars even when the
 * guest's prefers-color-scheme is dark. Inserting this at user origin lets
 * it follow prefers-color-scheme while still losing to any scrollbar styling
 * the page authors themselves.
 */

import type { WebContents } from 'electron'

const SCROLLBAR_CSS = `
@media (prefers-color-scheme: dark) {
  :root {
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.25) transparent;
  }
}
@media (prefers-color-scheme: light) {
  :root {
    scrollbar-width: thin;
    scrollbar-color: rgba(0, 0, 0, 0.25) transparent;
  }
}
`

/**
 * Re-inserts the scrollbar CSS on every top-level navigation. No removal
 * bookkeeping is needed: 'dom-ready' fires once per document, and a
 * user-origin insertion dies with the document it was inserted into.
 */
export function installScrollbarCss(webContents: WebContents): void {
  webContents.on('dom-ready', () => {
    webContents.insertCSS(SCROLLBAR_CSS, { cssOrigin: 'user' }).catch(() => {})
  })
}
