/**
 * Local-file watcher integration test (issue #209).
 *
 * Drives the real runtime in-process: create a file entity pointing at a temp
 * file, edit the file on disk, and assert the watcher bumps `fileReloadVersion`
 * — the signal the renderers key off to re-fetch. Also guards teardown on
 * entity delete so no FSWatcher handle leaks.
 *
 * Mutation-verified by: commenting out the `watchEntityFile(...)` call in
 * `createFileEntity` (src/main/runtime/file-entity-state.ts) — "edit bumps
 * fileReloadVersion" fails because the version stays 0 after the disk write.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, rmSync, writeFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createFileEntity, deleteFileEntity } from '../../src/main/runtime/document-commands'
import { getFileReloadVersion } from '../../src/main/runtime/local-file-watcher'

let harness: WorkspaceHarness
let tmpDir: string | null = null

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

// fs.watch fires + 80ms debounce; poll until the version moves or we give up.
async function waitForVersionAbove(id: string, floor: number): Promise<number> {
  for (let i = 0; i < 40; i++) {
    const v = getFileReloadVersion(id)
    if (v > floor) return v
    await sleep(50)
  }
  return getFileReloadVersion(id)
}

describe('local-file watcher', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-watcher-'))
  })

  afterEach(() => {
    if (tmpDir) {
      try { rmSync(tmpDir, { recursive: true, force: true }) } catch {}
      tmpDir = null
    }
  })

  afterAll(() => harness?.dispose())

  it('bumps fileReloadVersion when a watched file changes on disk', async () => {
    const filePath = join(tmpDir!, 'test.html')
    writeFileSync(filePath, '<h1>v1</h1>', 'utf8')

    const { id } = createFileEntity({ canvasX: 0, canvasY: 0, file: filePath })
    expect(getFileReloadVersion(id)).toBe(0)

    await sleep(50) // let the watcher register before writing
    writeFileSync(filePath, '<h1>v2</h1>', 'utf8')

    expect(await waitForVersionAbove(id, 0)).toBeGreaterThan(0)
  })

  it('stops watching once the entity is deleted', async () => {
    const filePath = join(tmpDir!, 'deleted.html')
    writeFileSync(filePath, '<h1>v1</h1>', 'utf8')

    const { id } = createFileEntity({ canvasX: 0, canvasY: 0, file: filePath })
    await sleep(50)

    expect(deleteFileEntity(id)).toBe(true)
    // Version is cleared on unwatch, and later disk writes must not resurrect it.
    writeFileSync(filePath, '<h1>after-delete</h1>', 'utf8')
    await sleep(200)
    expect(getFileReloadVersion(id)).toBe(0)
  })
})
