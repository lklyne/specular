import type { Page } from './runtime-entities'
import type { PageColorScheme } from '../../shared/types'
import { ensurePageDebugger } from './page-debugger'

// webContents has no native prefers-color-scheme override, so this rides CDP's
// Emulation.setEmulatedMedia over the page's (single, shared) debugger session.

/**
 * Returns true only when the override actually dispatched. Attach can fail
 * (e.g. a DevTools frontend already owns the debugger); the caller keeps
 * lastColorSchemeKey stale so the next layout pass retries instead of wedging
 * the page on the wrong scheme.
 */
export function applyPageColorScheme(page: Page, scheme: PageColorScheme | null): boolean {
  const wc = page.pageView.webContents
  if (wc.isDestroyed()) return false

  if (!ensurePageDebugger(wc, () => { page.lastColorSchemeKey = undefined })) return false
  try {
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
