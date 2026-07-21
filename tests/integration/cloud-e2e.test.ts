/**
 * Cloud-sync end-to-end: the HTML prototyping loop across a real transport
 * (ADR 0018 §5, cloud-sync spike step 6).
 *
 * Boots the Worker under miniflare (`bootServerHarness`) and the desktop
 * runtime in-process (`bootWorkspaceHarness`), attaches the real desktop
 * transport to the workspace doc, and drives a headless agent peer through the
 * loop: upload HTML bytes → write a file entity pointing at the content hash →
 * the Durable Object propagates → the desktop's runtime reconstructs the entity
 * and resolves its `asset://` reference to the sync server's asset URL, which
 * serves the exact bytes. Repointing to new bytes carries a new hash (the
 * structural reload signal). A desktop-side move round-trips back to the agent,
 * proving the transport is bidirectional.
 *
 * Mutation-verified by:
 *   - reverting the observer guard's `|| isRemoteOrigin(...)` branch (so only
 *     the REMOTE_SYNC_ORIGIN symbol counts as remote) — "the agent's write
 *     reaches the desktop runtime" fails: the transport-provider transaction is
 *     no longer recognized, so the reverse sync never rebuilds fileEntities.
 *   - making `writeHtmlEntity` skip the `existing.set('file', …)` repoint (leave
 *     the old hash) — "repoint carries the new content hash" fails.
 */

import { afterAll, beforeAll, describe, expect, it } from 'vitest'
import * as Y from 'yjs'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { fileEntities, buildFileEntitySceneEntity } from '../../src/main/runtime/file-entity-state'
import { updateFileEntity } from '../../src/main/runtime/document-commands'
import { DOC_MAP_ENTITIES } from '../../src/main/runtime/workspace-doc'
import { publishBinding, resetSyncState } from '../../src/main/runtime/workspace-sync'
import {
  connectSyncTransport,
  disconnectSyncTransport,
} from '../../src/main/runtime/workspace-sync-transport'
import { SyncClientSession } from '../../src/main/sync-client'
import { bootServerHarness, type ServerHarness } from '../../server/test/harness'
import {
  signInAnonymous,
  createDoc,
  createLink,
  ownerConnect,
  waitFor,
} from '../../server/test/helpers'

const ORIGIN = { x: 0, y: 0 }
const PAN = { x: 0, y: 0 }

let server: ServerHarness
let harness: WorkspaceHarness

/** Build the scene entity the renderer would see for a runtime file entity. */
function sceneFileFor(entityId: string): string | undefined {
  const entity = fileEntities.find((e) => e.id === entityId)
  if (!entity) return undefined
  return buildFileEntitySceneEntity(entity, 1, PAN, ORIGIN).file
}

async function fetchText(url: string): Promise<string> {
  const res = await fetch(url)
  return res.text()
}

describe('cloud sync — HTML prototyping loop, end to end', () => {
  beforeAll(async () => {
    server = await bootServerHarness()
    harness = bootWorkspaceHarness()
  }, 30_000)

  afterAll(async () => {
    disconnectSyncTransport()
    resetSyncState()
    harness?.dispose()
    await server?.dispose()
  })

  it('agent writes HTML → desktop resolves and serves it; repoint + local move round-trip', async () => {
    // --- Owner publishes the workspace and attaches the desktop transport ---
    const owner = await signInAnonymous(server.url)
    const docId = await createDoc(server.url, { cookie: owner.cookie })
    publishBinding({ docId, url: server.url })

    const ownerToken = (await ownerConnect(server.url, { cookie: owner.cookie }, docId)).token
    const transport = connectSyncTransport(ownerToken)!
    expect(transport).not.toBeNull()
    await waitFor(() => transport.synced)

    // --- Agent joins via a minted edit link ---
    const editLink = await createLink(server.url, owner.cookie, docId, 'edit')
    const client = await SyncClientSession.join(`${server.url}${editLink.url}`)

    try {
      // --- v1: upload + write a fresh file entity ---
      const v1 = await client.writeHtmlEntity('<html>v1</html>', {
        canvasX: 100,
        canvasY: 100,
        width: 400,
        height: 300,
      })

      // The entity lands in the desktop runtime through the transport.
      await waitFor(() => fileEntities.some((e) => e.id === v1.entityId))
      const desktopEntity = fileEntities.find((e) => e.id === v1.entityId)!
      expect(desktopEntity.canvasX).toBe(100)
      expect(desktopEntity.width).toBe(400)

      // Its asset:// reference resolves to the sync server's asset URL...
      expect(sceneFileFor(v1.entityId)).toBe(`${server.url}/assets/${v1.assetId}`)
      // ...which serves the exact bytes the agent uploaded.
      expect(await fetchText(`${server.url}/assets/${v1.assetId}`)).toBe('<html>v1</html>')

      // --- v2: repoint the SAME entity to new bytes (new hash = reload signal) ---
      const v2 = await client.writeHtmlEntity('<html>v2</html>', { id: v1.entityId })
      expect(v2.entityId).toBe(v1.entityId)
      expect(v2.assetId).not.toBe(v1.assetId)

      await waitFor(() => fileEntities.find((e) => e.id === v1.entityId)?.file === `asset://${v2.assetId}.html`)
      expect(sceneFileFor(v1.entityId)).toBe(`${server.url}/assets/${v2.assetId}`)
      expect(await fetchText(`${server.url}/assets/${v2.assetId}`)).toBe('<html>v2</html>')

      // --- Bidirectional: a desktop-side move round-trips back to the agent ---
      updateFileEntity(v1.entityId, { canvasX: 640 })
      const clientEntities = client.doc.getMap(DOC_MAP_ENTITIES) as Y.Map<Y.Map<unknown>>
      await waitFor(() => clientEntities.get(v1.entityId)?.get('canvasX') === 640)
      expect(clientEntities.get(v1.entityId)!.get('file')).toBe(`asset://${v2.assetId}.html`)
    } finally {
      client.close()
    }
  }, 30_000)
})
