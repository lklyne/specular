import type { Page } from './runtime-entities'
import type { PageColorScheme } from '../../shared/types'

/**
 * Electron's webContents has no native API for forcing a guest's
 * `prefers-color-scheme`, so a per-page colorScheme override rides the CDP
 * `Emulation.setEmulatedMedia` command over the page's debugger session — the
 * same session app-control-server.ts shares for input dispatch (Electron
 * allows only one attached debugger per WebContents, hence the isAttached()
 * guard).
 *
 * The override persists across navigations within an attached session, but a
 * debugger detach drops it; track attachment per WebContents so a later
 * re-attach re-applies rather than silently leaving the guest on the wrong
 * scheme.
 */
const detachListenerAttached = new WeakSet<Electron.WebContents>()

/**
 * Applies (scheme is 'light'/'dark') or clears (scheme is null) the page's
 * prefers-color-scheme override. Absent colorScheme (null here) means the
 * page must follow the app/OS theme live, so it gets no override at all.
 *
 * Returns true only when the override was actually dispatched. Attach failures
 * (most commonly: a DevTools frontend is already attached to the page, so
 * Electron refuses a second debugger client) return false so the caller leaves
 * lastColorSchemeKey stale and the next layout pass retries — otherwise a single
 * failed attach wedges the page on the wrong scheme until colorScheme changes
 * value again. Async send rejections reset the key from inside the catch for the
 * same reason. Enforcement layered on persisted state, never a source of truth,
 * so it must not crash the layout pass.
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
        // CDP has no per-feature clear: an override is removed by omitting the
        // feature, not by sending value ''. A non-empty value pins the page and
        // stops it from following nativeTheme (the app's global theme), so
        // 'System' (scheme null) must send empty features to truly release it.
        features: scheme ? [{ name: 'prefers-color-scheme', value: scheme }] : [],
      })
      .catch(() => {
        // Send failed after attach — force the next layout pass to retry.
        page.lastColorSchemeKey = undefined
      })
    return true
  } catch {
    return false
  }
}
