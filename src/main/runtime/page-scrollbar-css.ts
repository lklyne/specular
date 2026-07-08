// Under device emulation Blink paints root scrollbars from the page's declared
// color-scheme only, so sites that never declare it get thick light scrollbars
// even when the guest prefers dark. User-origin insertion still loses to any
// scrollbar styling the page authors themselves.

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

// No removal bookkeeping: a user-origin insertion dies with its document.
export function installScrollbarCss(webContents: WebContents): void {
  webContents.on('dom-ready', () => {
    webContents.insertCSS(SCROLLBAR_CSS, { cssOrigin: 'user' }).catch(() => {})
  })
}
