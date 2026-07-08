import type { Page } from './runtime-entities'
import type { PageColorScheme } from '../../shared/types'

// webContents has no native prefers-color-scheme override, so this rides CDP's
// Emulation.setEmulatedMedia over the page's (single, shared) debugger session.
// A debugger detach silently drops the override, hence the re-apply bookkeeping.
const detachListenerAttached = new WeakSet<Electron.WebContents>()

/**
 * Returns true only when the override actually dispatched. Attach can fail
 * (e.g. a DevTools frontend already owns the debugger); the caller keeps
 * lastColorSchemeKey stale so the next layout pass retries instead of wedging
 * the page on the wrong scheme.
 */
export function applyPageColorScheme(page: Page, scheme: PageColorScheme | null): boolean {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return false

  try {
    if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
    if (!detachListenerAttached.has(wc)) {
      detachListenerAttached.add(wc)
      wc.debugger.on('detach', () => {
        detachListenerAttached.delete(wc)
        page.lastColorSchemeKey = undefined
      })
    }
    wc.debugger
      .sendCommand('Emulation.setEmulatedMedia', {
        // CDP clears an override by omitting the feature (not value ''), which
        // releases the page back to following nativeTheme.
        features: scheme ? [{ name: 'prefers-color-scheme', value: scheme }] : [],
      })
      .catch(() => {
        page.lastColorSchemeKey = undefined
      })
    return true
  } catch {
    return false
  }
}
