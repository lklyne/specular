/**
 * File-entity device-frame (border) patch, against the real runtime.
 *
 * `applyCanvasPatch` — the same door `POST /canvas/apply`, `specular update`,
 * and the MCP `upsert_entities` tool all use — must honor a `showDeviceFrame`
 * patch on a `file` entity the same way it already does for a `page` entity.
 * Before this test, `fileKind.update` (src/main/entities/builtin/file.ts)
 * only wired `file`/`subpath`/`objectFit`/`width`/`height`/`canvasX`/`canvasY`,
 * so a `showDeviceFrame: true` patch on a file entity (e.g. an html iframe)
 * was silently dropped — the headless/agent path had no way to add a border,
 * even though the inspector's "Show device page" checkbox worked fine.
 *
 * Mutation-verified by: removing the `patch.showDeviceFrame !== undefined`
 * branch from `fileKind.update` — "sets showDeviceFrame on a file entity via
 * an update patch" fails (metadata stays without `showDeviceFrame`).
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, rmSync, writeFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { applyCanvasPatch } from '../../src/main/canvas-apply'
import { fileEntities } from '../../src/main/runtime/file-entity-state'
import { showDeviceFrameFromMetadata } from '../../src/main/runtime/runtime-entities'

let harness: WorkspaceHarness
let tmpDir: string | null = null

describe('file entity device frame (border) patch', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-file-frame-'))
  })

  afterEach(() => {
    if (tmpDir) {
      try { rmSync(tmpDir, { recursive: true, force: true }) } catch {}
      tmpDir = null
    }
  })

  afterAll(() => harness?.dispose())

  it('sets showDeviceFrame on a file entity via an update patch', async () => {
    const filePath = join(tmpDir!, 'widget.html')
    writeFileSync(filePath, '<h1>hi</h1>', 'utf8')

    // Explicit width/height skips on-disk image probing (see
    // canvas-format.test.ts) — no real image decode needed for this test.
    const { created } = applyCanvasPatch({
      entities: [{ kind: 'file', file: filePath, canvasX: 0, canvasY: 0, width: 300, height: 300 }],
    })
    const id = created[0]
    await settleSync()

    const before = fileEntities.find((e) => e.id === id)
    expect(before && showDeviceFrameFromMetadata(before.metadata)).toBe(false)

    applyCanvasPatch({ entities: [{ id, showDeviceFrame: true }] })
    await settleSync()

    const after = fileEntities.find((e) => e.id === id)
    expect(after && showDeviceFrameFromMetadata(after.metadata)).toBe(true)

    applyCanvasPatch({ entities: [{ id, showDeviceFrame: false }] })
    await settleSync()

    const cleared = fileEntities.find((e) => e.id === id)
    expect(cleared && showDeviceFrameFromMetadata(cleared.metadata)).toBe(false)
  })
})
