/**
 * A tab's `.canvas` file is keyed by its id, not its name (issue #361).
 *
 * Two tabs are allowed to share a name — the CLI refuses duplicates so refs
 * stay unambiguous, but nothing stops a rename in the UI. Before the id
 * suffix, same-named tabs resolved to one file: the second save overwrote the
 * first, and deleting or renaming either took the survivor's file with it.
 *
 * Mutation-verified by: (a) dropping the id suffix from `canvasFilePath` —
 * the distinct-files and delete cases fail; (b) dropping the legacy fallback
 * in `loadWorkspaceFromCanvasFiles` — the pre-suffix workspace loads as null.
 */

import { existsSync, writeFileSync } from 'fs'
import { join } from 'path'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { createTextEntity } from '../../src/main/runtime/document-commands'
import {
  createWorkspaceTab,
  deleteWorkspaceTab,
  renameWorkspaceTab,
  setActiveWorkspaceTab,
} from '../../src/main/runtime/workspace-tab-operations'
import { activeWorkspaceTabId, workspaceTabs } from '../../src/main/runtime/workspace-model'
import {
  DEFAULT_WORKSPACE_ID,
  canvasFilePath,
  loadWorkspaceFromCanvasFiles,
  readCanvasFile,
  writeWorkspaceMetaSync,
} from '../../src/main/runtime/workspace-persistence'

let harness: WorkspaceHarness

/** Two tabs both named `scratch` — the rename path has no duplicate guard. */
function makeTwinTabs(): [string, string] {
  const first = createWorkspaceTab('scratch')
  const second = createWorkspaceTab('other')
  expect(renameWorkspaceTab(second, 'scratch')).toBe(true)
  return [first, second]
}

function tabFile(tabId: string): string {
  const tab = workspaceTabs.find((candidate) => candidate.id === tabId)
  if (!tab) throw new Error(`no tab ${tabId}`)
  return canvasFilePath(harness.userDataPath, DEFAULT_WORKSPACE_ID, tab)
}

function textIn(filePath: string): string[] {
  const doc = readCanvasFile(filePath)
  return (doc?.nodes ?? []).map((node) => ('text' in node ? String(node.text) : '')).filter(Boolean)
}

describe('tab file identity', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('gives two same-named tabs distinct files', () => {
    const [first, second] = makeTwinTabs()

    setActiveWorkspaceTab(first)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'first canvas' })
    setActiveWorkspaceTab(second)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'second canvas' })
    harness.flush()

    expect(tabFile(first)).not.toBe(tabFile(second))
    expect(textIn(tabFile(first))).toEqual(['first canvas'])
    expect(textIn(tabFile(second))).toEqual(['second canvas'])
  })

  it('leaves a same-named sibling on disk when one is deleted', () => {
    const [first, second] = makeTwinTabs()
    setActiveWorkspaceTab(first)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'survivor' })
    harness.flush()
    const survivorFile = tabFile(first)

    expect(deleteWorkspaceTab(second)).toBe(true)

    expect(existsSync(survivorFile)).toBe(true)
    expect(textIn(survivorFile)).toEqual(['survivor'])
  })

  it('renames one twin without disturbing the other file', () => {
    const [first, second] = makeTwinTabs()
    setActiveWorkspaceTab(first)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'survivor' })
    harness.flush()
    const survivorFile = tabFile(first)

    expect(renameWorkspaceTab(second, 'renamed')).toBe(true)

    expect(existsSync(survivorFile)).toBe(true)
    expect(textIn(survivorFile)).toEqual(['survivor'])
  })

  it('loads a workspace written before filenames carried an id, and retires the file', () => {
    const userDataPath = harness.userDataPath
    const tabId = 'tab_legacy'
    const legacyPath = join(userDataPath, 'workspaces', DEFAULT_WORKSPACE_ID, 'Notes.canvas')
    setActiveWorkspaceTab(activeWorkspaceTabId)
    createTextEntity({ canvasX: 0, canvasY: 0, text: 'from the old file' })
    harness.flush()
    const doc = readCanvasFile(tabFile(activeWorkspaceTabId))
    expect(doc).toBeTruthy()
    writeFileSync(legacyPath, JSON.stringify(doc), 'utf8')
    writeWorkspaceMetaSync(userDataPath, DEFAULT_WORKSPACE_ID, {
      activeTabId: tabId,
      tabs: [{ id: tabId, name: 'Notes', updatedAt: new Date().toISOString(), expanded: true }],
    })

    const record = loadWorkspaceFromCanvasFiles(userDataPath, DEFAULT_WORKSPACE_ID)

    expect(record?.tabs.map((tab) => tab.name)).toEqual(['Notes'])
    expect(existsSync(legacyPath)).toBe(false)
  })
})
