/**
 * Smart-paste routing (src/main/clipboard-paste.ts): the resolution order
 * entity JSON → file refs → image → structured → markdown → URL → sticky
 * decides what a paste creates. Guards the two ends of the chain the user
 * hits constantly: a URL pastes as a live page, plain text pastes as a
 * sticky text entity.
 *
 * Mutation-verified by swapping the URL check below the sticky-text fallback
 * in pasteFromClipboard — "paste of a URL creates a page" then fails because
 * the URL lands as a text entity.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { clipboard } from 'electron'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { pasteFromClipboard } from '../../src/main/clipboard-paste'
import { pages } from '../../src/main/runtime/runtime-context'
import { getTextEntities } from '../../src/main/runtime/document-commands'

let harness: WorkspaceHarness

describe('smart paste', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
  })

  afterAll(() => harness?.dispose())

  it('paste of a URL creates a page', async () => {
    clipboard.writeText('https://example.com/pasted')
    pasteFromClipboard({ canvasX: 100, canvasY: 100 })
    await settleSync()

    expect(pages.some((p) => p.url === 'https://example.com/pasted')).toBe(true)
    expect(getTextEntities()).toHaveLength(0)
  })

  it('paste of plain text creates a sticky text entity', async () => {
    clipboard.writeText('just a thought')
    pasteFromClipboard({ canvasX: 50, canvasY: 50 })
    await settleSync()

    const stickies = getTextEntities()
    expect(stickies).toHaveLength(1)
    expect(stickies[0].text).toBe('just a thought')
    expect(pages).toHaveLength(0)
  })
})
