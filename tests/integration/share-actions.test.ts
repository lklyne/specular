/**
 * Share-actions integration tests (ADR 0018 §4b, cloud-sync spike step 7).
 *
 * Boots the Worker under miniflare (`bootServerHarness`) and the desktop
 * runtime in-process (`bootWorkspaceHarness`), then drives the main-runtime
 * share actions the toolbar IPC handlers call — publish, copy-link, reset,
 * revoke, state — against the live server. The device session and the dev flag
 * are stored under the harness temp `userData` dir automatically (the electron
 * stub points `app.getPath('userData')` there).
 *
 * Mutation-verified by:
 *   - making `ensurePublished` skip `connectSyncTransport` — "publish attaches
 *     the transport" fails (getSyncProvider stays null, status never connects).
 *   - making `shareCopyLink` mint a fresh token each call (ignore the server's
 *     idempotent response and append Math.random) — "second copy returns the
 *     same url" fails.
 *   - dropping the `clipboard.writeText(url)` line — "copy writes the clipboard"
 *     fails.
 */

import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { clipboard } from 'electron'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { bootServerHarness, type ServerHarness } from '../../server/test/harness'
import { redeem } from '../../server/test/helpers'
import { parseShareLink } from '../../src/main/sync-client/share-link'
import { saveCloudShareConfig } from '../../src/main/runtime/preferences'
import { clearStoredSessions } from '../../src/main/runtime/cloud-credentials'
import {
  getSyncBinding,
  getSyncStatus,
  resetSyncState,
} from '../../src/main/runtime/workspace-sync'
import {
  disconnectSyncTransport,
  getSyncProvider,
} from '../../src/main/runtime/workspace-sync-transport'
import {
  shareCopyLink,
  shareListLinks,
  shareResetLink,
  shareRevokeLink,
  shareState,
  sharePublish,
} from '../../src/main/runtime/share-actions'

async function waitFor(predicate: () => boolean, timeoutMs = 10_000): Promise<void> {
  const start = Date.now()
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error('waitFor timed out')
    await new Promise((r) => setTimeout(r, 20))
  }
}

let server: ServerHarness
let harness: WorkspaceHarness

describe('cloud share — main-runtime actions', () => {
  beforeAll(async () => {
    server = await bootServerHarness()
    harness = bootWorkspaceHarness()
    clearStoredSessions()
    resetSyncState()
    saveCloudShareConfig({ enabled: true, serverUrl: server.url })
  }, 30_000)

  afterAll(async () => {
    disconnectSyncTransport()
    resetSyncState()
    harness?.dispose()
    await server?.dispose()
  })

  it('publish creates a doc + binding and attaches the transport', async () => {
    expect(getSyncBinding()).toBeNull()

    const result = await sharePublish()
    expect(result.ok).toBe(true)

    const binding = getSyncBinding()
    expect(binding).not.toBeNull()
    expect(binding?.docId).toBeTruthy()
    expect(binding?.url).toBe(server.url)

    // Transport attached and converges.
    expect(getSyncProvider()).not.toBeNull()
    await waitFor(() => getSyncStatus() === 'connected')
  }, 30_000)

  it('copyLink(comment) returns a redeemable comment-scope url and writes the clipboard', async () => {
    const result = await shareCopyLink('comment')
    expect(result.ok).toBe(true)
    if (!result.ok) return
    const url = result.value.url

    // The clipboard was written main-side.
    expect(clipboard.readText()).toBe(url)

    // The url's token redeems at the server with comment scope.
    const parsed = parseShareLink(url)
    expect(parsed.docId).toBe(getSyncBinding()?.docId)
    const redeemed = await redeem(server.url, parsed.token)
    expect(redeemed.status).toBe(201)
    expect(redeemed.body.scope).toBe('comment')
  }, 30_000)

  it('copyLink is idempotent per scope — a second copy returns the same url', async () => {
    const first = await shareCopyLink('comment')
    const second = await shareCopyLink('comment')
    expect(first.ok && second.ok).toBe(true)
    if (first.ok && second.ok) {
      expect(second.value.url).toBe(first.value.url)
    }
  }, 30_000)

  it('reset rotates the link token; revoke removes it from the list', async () => {
    const before = await shareListLinks()
    expect(before.ok).toBe(true)
    if (!before.ok) return
    const link = before.value.find((l) => l.scope === 'comment')
    expect(link).toBeDefined()
    if (!link) return
    const originalUrl = link.url

    // Reset keeps the grant but rotates the token → a different url.
    const reset = await shareResetLink(link.grantId)
    expect(reset.ok).toBe(true)
    if (reset.ok) {
      expect(reset.value.grantId).toBe(link.grantId)
      expect(reset.value.url).not.toBe(originalUrl)
    }

    // Revoke deletes the grant — it disappears from the list.
    const revoked = await shareRevokeLink(link.grantId)
    expect(revoked.ok).toBe(true)

    const after = await shareListLinks()
    expect(after.ok).toBe(true)
    if (after.ok) {
      expect(after.value.some((l) => l.grantId === link.grantId)).toBe(false)
    }
  }, 30_000)

  it('shareState reflects the flag, binding, status, and active links', async () => {
    // Mint a fresh link so the list is non-empty.
    await shareCopyLink('edit')
    const state = await shareState()
    expect(state.enabled).toBe(true)
    expect(state.serverUrl).toBe(server.url)
    expect(state.binding?.docId).toBe(getSyncBinding()?.docId)
    expect(state.status).toBe('connected')
    expect(state.links?.some((l) => l.scope === 'edit')).toBe(true)
  }, 30_000)
})
