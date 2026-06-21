/**
 * Smoke coverage for the local-file watcher (issue #209).
 *
 * Mutation-verified by commenting out `onChangedCallback?.(changedEntityIds)`
 * in local-file-watcher.ts and confirming the test hangs waiting for the
 * version to increment (i.e. it never sees fileReloadVersion > 0).
 */
import { describe, it, expect, afterEach } from 'vitest'
import { writeFileSync, mkdtempSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import {
  createFileEntity,
  deleteFileEntity,
  getFileEntities,
} from './app-client'
import { wait, waitFor } from './test-utils'

const createdIds: string[] = []
let tmpDir: string | null = null

afterEach(async () => {
  if (createdIds.length) {
    for (const id of createdIds.splice(0)) {
      await deleteFileEntity(id).catch(() => {})
    }
  }
  if (tmpDir) {
    try { rmSync(tmpDir, { recursive: true, force: true }) } catch {}
    tmpDir = null
  }
})

describe('local-file watcher', () => {
  it('increments fileReloadVersion when a watched file changes on disk', async () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-watcher-test-'))
    const filePath = join(tmpDir, 'test.html')
    writeFileSync(filePath, '<h1>v1</h1>', 'utf8')

    const { id } = await createFileEntity({ canvasX: 0, canvasY: 0, file: filePath, width: 300, height: 300 })
    createdIds.push(id)

    // Verify initial version is 0.
    const { fileEntities: before } = await getFileEntities()
    const before0 = before.find((e) => e.id === id)
    expect(before0).toBeDefined()
    expect(before0!.fileReloadVersion).toBe(0)

    // Give the watcher a moment to register before we write.
    await wait(50)

    // Modify the file on disk.
    writeFileSync(filePath, '<h1>v2</h1>', 'utf8')

    // Wait for the watcher debounce (80 ms) + layout pass to propagate.
    const result = await waitFor(
      () => getFileEntities(),
      ({ fileEntities }) => {
        const e = fileEntities.find((fe) => fe.id === id)
        return (e?.fileReloadVersion ?? 0) > 0
      },
      'fileReloadVersion did not increment after disk write',
      { maxAttempts: 30, intervalMs: 100 },
    )

    const updated = result.fileEntities.find((e) => e.id === id)
    expect(updated!.fileReloadVersion).toBeGreaterThan(0)
  })

  it('does not increment fileReloadVersion when file content is unchanged', async () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-watcher-test-'))
    const filePath = join(tmpDir, 'stable.html')
    writeFileSync(filePath, '<h1>same</h1>', 'utf8')

    const { id } = await createFileEntity({ canvasX: 100, canvasY: 100, file: filePath, width: 300, height: 300 })
    createdIds.push(id)
    await wait(50)

    // Write the exact same bytes.
    writeFileSync(filePath, '<h1>same</h1>', 'utf8')

    // Wait past the debounce window; version should remain 0.
    await wait(300)

    const { fileEntities } = await getFileEntities()
    const e = fileEntities.find((fe) => fe.id === id)
    expect(e!.fileReloadVersion).toBe(0)
  })

  it('stops watching when the entity is deleted', async () => {
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-watcher-test-'))
    const filePath = join(tmpDir, 'deleted.html')
    writeFileSync(filePath, '<h1>initial</h1>', 'utf8')

    const { id } = await createFileEntity({ canvasX: 200, canvasY: 200, file: filePath, width: 300, height: 300 })
    await wait(50)

    await deleteFileEntity(id)
    await wait(50)

    // Modify file after entity is gone — should not error and the entity is gone.
    writeFileSync(filePath, '<h1>after-delete</h1>', 'utf8')
    await wait(300)

    const { fileEntities } = await getFileEntities()
    expect(fileEntities.find((e) => e.id === id)).toBeUndefined()
  })
})
