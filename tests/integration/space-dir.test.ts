/**
 * The user-chosen space folder (ADR 0033 §1).
 *
 * With `spacePath` unset, everything resolves under the legacy
 * `userData/workspaces/default` — covered implicitly by every other
 * integration test, since the harness never sets `spacePath`. These tests
 * cover the other half: once a space is chosen, `.canvas` files, workspace
 * metadata, image assets, and notes all land directly inside it, with no
 * `workspaces/<id>/` nesting, and a meta file left at the pre-`.specular/`
 * root location is still read.
 *
 * Mutation-verified by: (a) hard-coding `spaceDir()` to ignore `getSpacePath()`
 * — the "no workspaces/ nesting" and asset-location assertions fail; (b)
 * changing `writeSpaceMetaSync` to write straight into `spacePath` instead
 * of `spacePath/.specular` — the `.specular/` assertion fails; (c) dropping the
 * legacy-path fallback branch in `readSpaceMeta` — the root-meta test
 * fails to find anything.
 */

import { mkdtempSync, rmSync, existsSync, readFileSync, writeFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import { setSpacePath } from '../../src/main/runtime/preferences'
import { saveImageBuffer } from '../../src/main/runtime/image-assets'
import { createNoteFile } from '../../src/main/runtime/note-assets'
import { readSpaceMeta } from '../../src/main/runtime/space-persistence'

let harness: WorkspaceHarness
const chosenDirs: string[] = []

function freshSpaceDir(): string {
  const dir = mkdtempSync(join(tmpdir(), 'specular-space-'))
  chosenDirs.push(dir)
  return dir
}

describe('user-chosen space folder', () => {
  // Captured per test, not re-derived via spaceDir() — assertions must check
  // against the literal folder handed to setSpacePath, or a spaceDir() bug
  // that breaks production and the test in the same way would go uncaught.
  let chosen = ''

  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    chosen = freshSpaceDir()
    setSpacePath(chosen)
    harness.reset()
  })

  afterAll(() => {
    harness?.dispose()
    for (const dir of chosenDirs) rmSync(dir, { recursive: true, force: true })
  })

  it('writes a .canvas file directly into the chosen folder, with no workspaces/ nesting', () => {
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'hello' })
    harness.flush()

    const path = harness.diskPath()
    expect(path.startsWith(chosen)).toBe(true)
    expect(path).not.toContain('workspaces')
  })

  it('writes workspace-meta.json into .specular/', () => {
    harness.flush()

    expect(existsSync(join(chosen, '.specular', 'workspace-meta.json'))).toBe(true)
    expect(existsSync(join(chosen, 'workspace-meta.json'))).toBe(false)
  })

  it('saves image assets and notes into the chosen folder', () => {
    const imagePath = saveImageBuffer(Buffer.from('fake-png-bytes'), 'png')
    expect(imagePath.startsWith(join(chosen, 'assets'))).toBe(true)

    const notePath = createNoteFile('Space Note', 'hello world')
    expect(notePath.startsWith(chosen)).toBe(true)
    expect(readFileSync(notePath, 'utf8')).toBe('hello world')
  })

  it('still reads a workspace-meta.json left at the legacy root location', () => {
    const dir = mkdtempSync(join(tmpdir(), 'specular-space-legacy-'))
    try {
      const legacyMeta = {
        activeTabId: 'tab_root',
        tabs: [{ id: 'tab_root', name: 'Root Notes', updatedAt: new Date().toISOString(), expanded: true }],
      }
      writeFileSync(join(dir, 'workspace-meta.json'), JSON.stringify(legacyMeta), 'utf8')

      expect(readSpaceMeta(dir)).toEqual(legacyMeta)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })
})
