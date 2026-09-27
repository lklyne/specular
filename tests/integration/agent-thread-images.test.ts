/**
 * Pasted images on the canvas agent thread: Send with images saves each one
 * beside the thread, records it on the user message (surviving a reload from
 * disk), and hands the turn's image bytes to the agent backend. An image-only
 * turn still sends. The agent backend is the process boundary and is stubbed.
 *
 * Mutation-verified by:
 * - dropping `images` from the `runFixAgent` options in `invokeThreadAgent` —
 *   the backend-bytes assertion fails;
 * - dropping `...parseImages(raw.images)` in `parseMessages` — the reload
 *   assertion fails;
 * - reverting `startThreadRun`'s content check to text-only — the image-only
 *   turn never reaches the backend.
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest'
import { existsSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import { _setBackendOverride, type FixResult, type InvokeOptions } from '../../src/main/agent-fix/agent-backend'
import { spaceDir } from '../../src/main/runtime/space-dir'
import {
  _resetThreadsForTests,
  getActiveThread,
  loadThreadsFromDisk,
  sendActiveThread,
} from '../../src/main/agent-thread/thread-runtime'
import type { ThreadImageUpload } from '../../src/shared/agent-thread'

let harness: WorkspaceHarness

// A 1×1 transparent PNG.
const PNG: ThreadImageUpload = {
  mediaType: 'image/png',
  data: 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=',
}

let calls: { prompt: string; options: InvokeOptions }[] = []

async function flush(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve()
}

describe('pasted images on the canvas agent thread', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    rmSync(join(spaceDir(), '.specular', 'threads'), { recursive: true, force: true })
    _resetThreadsForTests()
    calls = []
    _setBackendOverride(async (prompt, _cwd, options) => {
      calls.push({ prompt, options })
      return { summary: 'seen', shouldResolve: true, rawOutput: 'seen' } satisfies FixResult
    })
  })

  afterEach(() => _setBackendOverride(null))
  afterAll(() => harness?.dispose())

  it('saves pasted images beside the thread and hands their bytes to the agent', async () => {
    expect(sendActiveThread('what is wrong here?', [PNG, PNG])).toBe(true)
    await flush()

    expect(calls).toHaveLength(1)
    expect(calls[0].options.images).toEqual([PNG, PNG])
    expect(calls[0].prompt).toContain('[User] what is wrong here? [image: ')

    const images = getActiveThread()?.messages[0].images ?? []
    expect(images).toHaveLength(2)
    for (const image of images) expect(existsSync(join(spaceDir(), image.path))).toBe(true)

    _resetThreadsForTests()
    loadThreadsFromDisk()
    expect(getActiveThread()?.messages[0].images).toEqual(images)
  })

  it('sends a turn that is only an image', async () => {
    expect(sendActiveThread('', [PNG])).toBe(true)
    await flush()

    expect(calls).toHaveLength(1)
    expect(calls[0].options.images).toEqual([PNG])
    expect(getActiveThread()?.title).toBe('Image')
  })
})
