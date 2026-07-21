/**
 * Workspace cloud-sync transport.
 *
 * Attaches a `y-partyserver` `YProvider` to the active workspace `Y.Doc` so
 * remote peers (other desktops, headless agents) and this process converge over
 * the Durable Object. This is the network half of the seam whose in-memory
 * shape lives in `workspace-sync.ts`:
 *
 *   - Remote updates the provider applies carry the *provider instance* as the
 *     transaction origin. We register it in `workspace-sync-state`'s
 *     remote-origin set so the Y.Doc→runtime observer patches the runtime
 *     arrays for them (as it already does for `REMOTE_SYNC_ORIGIN`), while the
 *     UndoManager — tracking only {null,'user'} — leaves them out of the local
 *     undo stack with no extra wiring.
 *   - Local `'user'` transactions still flow out through the provider's own doc
 *     observer, so desktop edits reach every peer.
 *
 * Status is mirrored into `setSyncStatus` so the UI/indicator can read one
 * value: 'connecting' on attach, 'connected' once the first sync completes,
 * 'error' on a socket failure or close.
 */

import WebSocket from 'ws'
import YProvider from 'y-partyserver/provider'
import { getActiveDoc } from './workspace-doc'
import { getSyncBinding, setSyncStatus, registerRemoteOrigin, unregisterRemoteOrigin } from './workspace-sync-state'

/** The party name the DO routes under (`CANVAS_DOC` → `canvas-doc`). */
const CANVAS_DOC_PARTY = 'canvas-doc'

let activeProvider: YProvider | null = null

/** The provider's host is scheme-less; it derives ws:// vs wss:// from the host. */
function hostOf(url: string): string {
  return new URL(url).host
}

/**
 * Attach the transport to the active doc using the current binding and a
 * connection `token` (redeemed elsewhere — the transport never mints one). No
 * binding means nothing to connect to; the caller should have published first.
 * Returns the provider so callers/tests can await sync or dispose it directly.
 */
export function connectSyncTransport(token: string): YProvider | null {
  const binding = getSyncBinding()
  if (!binding) return null

  disconnectSyncTransport()

  const doc = getActiveDoc()
  const provider = new YProvider(hostOf(binding.url), binding.docId, doc, {
    party: CANVAS_DOC_PARTY,
    params: { token },
    WebSocketPolyfill: WebSocket as unknown as typeof globalThis.WebSocket,
    disableBc: true,
  })

  // Remote transactions land under the provider instance; the observer must
  // treat them like `REMOTE_SYNC_ORIGIN`.
  registerRemoteOrigin(provider)

  setSyncStatus('connecting')
  provider.on('sync', (isSynced: boolean) => {
    if (isSynced) setSyncStatus('connected')
  })
  provider.on('connection-error', () => setSyncStatus('error'))
  provider.on('connection-close', () => setSyncStatus('error'))

  activeProvider = provider
  return provider
}

/** Tear down the transport: destroy the provider, drop its remote-origin
 *  registration, and mark the workspace disconnected. Safe to call when nothing
 *  is attached. */
export function disconnectSyncTransport(): void {
  if (!activeProvider) return
  const provider = activeProvider
  activeProvider = null
  unregisterRemoteOrigin(provider)
  provider.destroy()
  // Leave a published workspace's status untouched (a reconnect sets its own);
  // an unpublished one has no rendezvous, so it is plainly 'off'.
  if (!getSyncBinding()) setSyncStatus('off')
}

/** The live provider, or null when nothing is attached (tests, status checks). */
export function getSyncProvider(): YProvider | null {
  return activeProvider
}
