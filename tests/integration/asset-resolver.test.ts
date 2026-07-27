/**
 * File-entity asset-reference resolution (ADR 0018 §3, cloud-sync spike step
 * 5), against the real runtime.
 *
 * A file entity whose `file` is an `asset://<hash>` reference is portable —
 * the doc stores the id, not a location — and `buildFileEntitySceneEntity`
 * (src/main/runtime/file-entity-state.ts) is the seam that resolves it per
 * environment before it reaches a renderer. This covers the desktop half:
 * once a sync binding exists (`workspace-sync.publishBinding`), an
 * unresolvable local reference (no matching file on disk) must resolve to
 * the sync server's asset URL, and clearing the binding must make the same
 * reference unresolvable (degrades to the raw reference) again.
 *
 * Mutation-verified by: removing the `resolveEntityFileField` call in
 * `buildFileEntitySceneEntity` (passing `entity.file` straight through) —
 * "resolves to the sync server's asset URL" fails because the scene entity's
 * `file` field stays the raw `asset://…` string instead of the resolved URL.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { createFileEntity } from '../../src/main/runtime/document-commands'
import { fileEntities, buildFileEntitySceneEntity } from '../../src/main/runtime/file-entity-state'
import { publishBinding, clearSyncBinding, resetSyncState } from '../../src/main/runtime/workspace-sync'

let harness: WorkspaceHarness

const HASH = 'b'.repeat(64)
const ORIGIN = { x: 0, y: 0 }
const PAN = { x: 0, y: 0 }

describe('file entity asset-reference resolution', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    resetSyncState()
  })

  afterAll(() => harness?.dispose())

  it('resolves an asset:// reference to the sync server URL once a binding exists', async () => {
    const entity = createFileEntity({
      canvasX: 0,
      canvasY: 0,
      file: `asset://${HASH}.png`,
      width: 300,
      height: 300,
    })
    await settleSync()

    // No local copy on disk and no binding yet: unresolvable, degrades to the
    // raw reference.
    const before = fileEntities.find((e) => e.id === entity.id)!
    const sceneBefore = buildFileEntitySceneEntity(before, 1, PAN, ORIGIN)
    expect(sceneBefore.file).toBe(`asset://${HASH}.png`)

    publishBinding({ docId: 'doc-1', url: 'https://sync.example.com/doc-1' })

    const after = fileEntities.find((e) => e.id === entity.id)!
    const sceneAfter = buildFileEntitySceneEntity(after, 1, PAN, ORIGIN)
    expect(sceneAfter.file).toBe(`https://sync.example.com/doc-1/assets/${HASH}`)

    clearSyncBinding()
    const sceneCleared = buildFileEntitySceneEntity(after, 1, PAN, ORIGIN)
    expect(sceneCleared.file).toBe(`asset://${HASH}.png`)
  })

  it('leaves a non-asset file reference (a local path) untouched', async () => {
    const entity = createFileEntity({
      canvasX: 0,
      canvasY: 0,
      file: '/tmp/some-local-file.png',
      width: 300,
      height: 300,
    })
    await settleSync()

    publishBinding({ docId: 'doc-2', url: 'https://sync.example.com/doc-2' })
    const after = fileEntities.find((e) => e.id === entity.id)!
    const scene = buildFileEntitySceneEntity(after, 1, PAN, ORIGIN)
    expect(scene.file).toBe('/tmp/some-local-file.png')
  })
})
