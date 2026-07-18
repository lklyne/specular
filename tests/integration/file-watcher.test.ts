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
 *
 * The atomic-save test is mutation-verified by emptying the dir watcher's
 * callback in `startWatcher` (`src/main/runtime/local-file-watcher.ts`),
 * leaving only the file-level watch — the rename in that test silently kills
 * that watch, and the post-rename write never bumps the version again.
 *
 * The in-place-write test locks the observable contract for direct
 * overwrites (fs.writeFile, `cat new > file`). Caveat on its mutation: in
 * this harness environment Node's directory watch also reports in-place
 * writes, so deleting the `rearmFileWatch` calls does not fail it here — the
 * dir-watch-only blind spot it guards was reproduced manually on the
 * packaged macOS app (FSEvents caveat), which is why the file-level watch
 * leg exists.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, renameSync, rmSync, writeFileSync } from 'fs'
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

  it('bumps fileReloadVersion on repeated in-place writes (no rename)', async () => {
    // Direct overwrites (fs.writeFile, `cat new > file`) reuse the inode. On
    // macOS a directory watch alone does not reliably report these, so the
    // watcher also holds a file-level watch — this guards that leg.
    const filePath = join(tmpDir!, 'inplace.html')
    writeFileSync(filePath, '<h1>v1</h1>', 'utf8')

    const { id } = createFileEntity({ canvasX: 0, canvasY: 0, file: filePath })
    await sleep(50)

    writeFileSync(filePath, '<h1>v2</h1>', 'utf8')
    const first = await waitForVersionAbove(id, 0)
    expect(first).toBeGreaterThan(0)

    await sleep(50)
    writeFileSync(filePath, '<h1>v3</h1>', 'utf8')
    expect(await waitForVersionAbove(id, first)).toBeGreaterThan(first)
  })

  it('keeps reporting changes after an atomic save (write-temp-then-rename)', async () => {
    // Editors and agents commonly save atomically: write a temp file, then
    // rename it over the target. The rename replaces the target's inode —
    // a watch on the file itself would fire once for the rename and then go
    // silent forever, which is the bug this test guards against.
    const filePath = join(tmpDir!, 'atomic.html')
    writeFileSync(filePath, '<h1>v1</h1>', 'utf8')

    const { id } = createFileEntity({ canvasX: 0, canvasY: 0, file: filePath })
    await sleep(50)

    const tmpPath = join(tmpDir!, '.atomic.html.tmp')
    writeFileSync(tmpPath, '<h1>v2</h1>', 'utf8')
    renameSync(tmpPath, filePath)
    const afterRename = await waitForVersionAbove(id, 0)
    expect(afterRename).toBeGreaterThan(0)

    // The watcher must still be alive post-rename: a subsequent in-place
    // write should keep bumping the version, not silently stop.
    await sleep(50)
    writeFileSync(filePath, '<h1>v3</h1>', 'utf8')
    expect(await waitForVersionAbove(id, afterRename)).toBeGreaterThan(afterRename)
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
