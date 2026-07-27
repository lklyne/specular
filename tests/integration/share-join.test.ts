/**
 * Join integration tests (ADR 0018 §4b).
 *
 * Join is the receiving half of a share link: redeem the grant, adopt the
 * remote docId as this workspace's binding, attach the transport. The
 * interesting case is the refusal — the runtime owns one Y.Doc for the whole
 * workspace, so joining from a populated workspace would merge both sides'
 * entities in both directions. Until per-canvas doc granularity lands, join
 * only accepts an empty workspace.
 *
 * A separate file from `share-actions.test.ts` because join needs a workspace
 * that has never published, and the runtime binding is a module singleton.
 *
 * Mutation-verified by:
 *   - dropping the `isWorkspaceEmpty()` guard — "refuses to join when the
 *     workspace has entities" fails (the join succeeds and binds).
 *   - dropping `connectSyncTransport(connection.token)` from `shareJoin` —
 *     "join attaches the transport and converges" fails (provider stays null).
 *   - having `shareJoin` bind `parsed.docId` from the link instead of the
 *     redeemed `connection.docId` still passes, so the assertion checks the
 *     doc's contents arrived, not just that a binding exists.
 */

import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { bootServerHarness, type ServerHarness } from '../../server/test/harness'
import { createDoc, createLink, signInAnonymous } from '../../server/test/helpers'
import { saveCloudShareConfig } from '../../src/main/runtime/preferences'
import { clearStoredSessions } from '../../src/main/runtime/cloud-credentials'
import { buildShareLink } from '../../src/main/sync-client/share-link'
import { getSyncBinding, getSyncStatus, resetSyncState } from '../../src/main/runtime/workspace-sync'
import {
  disconnectSyncTransport,
  getSyncProvider,
} from '../../src/main/runtime/workspace-sync-transport'
import { shareJoin } from '../../src/main/runtime/share-actions'
import { createTextEntity } from '../../src/main/runtime/text-entity-state'

async function waitFor(predicate: () => boolean, timeoutMs = 10_000): Promise<void> {
  const start = Date.now()
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error('waitFor timed out')
    await new Promise((r) => setTimeout(r, 20))
  }
}

/** A doc owned by someone else, plus an edit link to it. */
async function remoteCanvas(serverUrl: string): Promise<{ docId: string; link: string }> {
  const owner = await signInAnonymous(serverUrl)
  const docId = await createDoc(serverUrl, { cookie: owner.cookie })
  const link = await createLink(serverUrl, owner.cookie, docId, 'edit')
  return { docId, link: buildShareLink({ base: serverUrl, docId, token: link.token }) }
}

let server: ServerHarness
let harness: WorkspaceHarness

describe('cloud share — join', () => {
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

  it('rejects a link that is not a share link', async () => {
    const result = await shareJoin('https://example.com/not-a-share-link')
    expect(result.ok).toBe(false)
    expect(getSyncBinding()).toBeNull()
  }, 30_000)

  it('joins an empty workspace to a remote doc and attaches the transport', async () => {
    const remote = await remoteCanvas(server.url)

    const result = await shareJoin(remote.link)
    expect(result.ok).toBe(true)

    const binding = getSyncBinding()
    expect(binding?.docId).toBe(remote.docId)
    expect(binding?.url).toBe(server.url)

    expect(getSyncProvider()).not.toBeNull()
    await waitFor(() => getSyncStatus() === 'connected')
  }, 30_000)

  it('refuses to join when the workspace is already bound', async () => {
    // The previous test left this workspace joined.
    expect(getSyncBinding()).not.toBeNull()
    const remote = await remoteCanvas(server.url)

    const result = await shareJoin(remote.link)
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.error).toMatch(/already published/)
    // Still bound to the first doc — a failed join never rebinds.
    expect(getSyncBinding()?.docId).not.toBe(remote.docId)
  }, 30_000)

  it('refuses to join when the workspace has entities', async () => {
    disconnectSyncTransport()
    resetSyncState()
    expect(getSyncBinding()).toBeNull()

    createTextEntity({ canvasX: 0, canvasY: 0, text: 'local work' })
    const remote = await remoteCanvas(server.url)

    const result = await shareJoin(remote.link)
    expect(result.ok).toBe(false)
    if (!result.ok) expect(result.error).toMatch(/merge/)
    expect(getSyncBinding()).toBeNull()
    expect(getSyncProvider()).toBeNull()
  }, 30_000)
})
