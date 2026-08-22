import type { WebContents } from 'electron'

/**
 * One CDP session per page, shared by everything that rides it (metrics
 * emulation, color-scheme media, input dispatch). Chromium drops every
 * override a session installed when that session detaches, so each owner
 * registers a detach handler that marks its state as needing re-apply.
 */
const detachHandlers = new WeakMap<WebContents, Set<() => void>>()

/**
 * Attaches the page's debugger if needed and registers `onDetach`. Returns
 * false when attach fails (a DevTools frontend may own the target); callers
 * leave their state unapplied so the next layout pass retries.
 */
export function ensurePageDebugger(wc: WebContents, onDetach: () => void): boolean {
  if (wc.isDestroyed()) return false
  try {
    if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
  } catch {
    return false
  }
  let handlers = detachHandlers.get(wc)
  if (!handlers) {
    handlers = new Set()
    detachHandlers.set(wc, handlers)
    const registered = handlers
    wc.debugger.on('detach', () => {
      for (const handler of registered) handler()
    })
  }
  handlers.add(onDetach)
  return true
}
