/**
 * Smart-paste routing (src/main/clipboard-paste.ts): the resolution order
 * entity JSON → file refs → image → structured → markdown → URL → sticky
 * decides what a paste creates. Guards the two ends of the chain the user
 * hits constantly and the native macOS image representation used by
 * screenshots.
 *
 * Mutation-verified by swapping the URL check below the sticky-text fallback
 * in pasteFromClipboard — "paste of a URL creates a page" then fails because
 * the URL lands as a text entity. Removing the native-format fallback makes
 * the screenshot test create no file entity.
 */

import { readFileSync } from 'fs'
import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { clipboard } from 'electron'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { pasteFromClipboard } from '../../src/main/clipboard-paste'
import { pages } from '../../src/main/runtime/runtime-context'
import {
  getFileEntities,
  getTextEntities,
} from '../../src/main/runtime/document-commands'

const ONE_PIXEL_PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
  'base64',
)

let harness: WorkspaceHarness

describe('smart paste', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    clipboard.clear()
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

  it('paste of a macOS native PNG creates an image file entity', async () => {
    clipboard.writeBuffer('public.png', ONE_PIXEL_PNG)

    pasteFromClipboard({ canvasX: 120, canvasY: 240 })
    await settleSync()

    const images = getFileEntities()
    expect(images).toHaveLength(1)
    expect(images[0]).toMatchObject({
      canvasX: 120,
      canvasY: 240,
      width: 1,
      height: 1,
    })
    expect(images[0].file).toMatch(/\.png$/)
    expect(readFileSync(images[0].file)).toEqual(ONE_PIXEL_PNG)
  })
})
