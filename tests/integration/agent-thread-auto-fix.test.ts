/**
 * Auto-fix on the canvas agent thread: a comment on an origin whose repo
 * binding has `autoFix` is sent the moment it is placed — no Send — aimed at
 * that pin and written to the bound repo. A comment placed while a run is in
 * flight queues behind it and drains into the same thread (same Claude
 * session) as soon as the run ends. Without auto-fix a comment only queues
 * into a draft. The agent backend is the process boundary and is stubbed.
 *
 * Mutation-verified by:
 * - making `initFixOrchestrator` always call `queueCommentOnAnnotation`
 *   (drop the `autoFixOn` branch) — the sends-at-once and drain cases fail;
 * - dropping the drain (`if (queuedTurnText(thread)) startThreadRun(...)`) at
 *   the tail of `runThreadAgent` — the drain case fails;
 * - passing `captureThreadPill()` instead of the annotation pill in
 *   `sendCommentOnAnnotation` — the write-target assertion fails (an unfocused
 *   canvas resolves to the space folder, not the repo).
 */

import { afterAll, afterEach, beforeEach, describe, expect, it } from 'vitest'
import { rmSync } from 'node:fs'
import { join } from 'node:path'
import { bootWorkspaceHarness, type WorkspaceHarness } from './harness'
import type { JsonCanvasLinkNode } from '../../src/shared/json-canvas-types'
import { createAnnotation } from '../../src/main/workspace-annotations'
import { initFixOrchestrator } from '../../src/main/agent-fix/fix-orchestrator'
import { _setBackendOverride, type FixResult } from '../../src/main/agent-fix/agent-backend'
import {
  bindOriginToRepoPath,
  __resetDevServerManagerForTests,
} from '../../src/main/runtime/dev-server-manager'
import { spaceDir } from '../../src/main/runtime/space-dir'
import {
  _resetThreadsForTests,
  getActiveThread,
  getAgentThreads,
} from '../../src/main/agent-thread/thread-runtime'

let harness: WorkspaceHarness

const PAGE_ID = 'page-auto-fix-host'
const ORIGIN = 'http://localhost:4321'
const PAGE_URL = `${ORIGIN}/pricing`
const REPO_PATH = '/tmp/specular-auto-fix-repo'

interface BackendCall {
  prompt: string
  cwd: string
  resumeSessionId?: string
  finish: (summary: string) => void
}

let calls: BackendCall[] = []

/** Each backend call parks until the test releases it with `finish`. */
function stubBackend(): void {
  _setBackendOverride(
    (prompt, cwd, options) =>
      new Promise<FixResult>((resolve) => {
        calls.push({
          prompt,
          cwd,
          resumeSessionId: options.resumeSessionId,
          finish: (summary) =>
            resolve({ summary, shouldResolve: true, rawOutput: summary, sessionId: 'sess_auto' }),
        })
      }),
  )
}

function loadHostPage(): void {
  harness.loadFixture({
    name: 'Auto-fix host',
    doc: {
      nodes: [
        {
          id: PAGE_ID,
          type: 'link',
          x: 120,
          y: 120,
          width: 375,
          height: 667,
          url: PAGE_URL,
          presetIndex: 0,
        } satisfies JsonCanvasLinkNode,
      ],
      edges: [],
      appState: { zoom: 1, pan: { x: 0, y: 0 } },
    },
  })
}

function comment(text: string) {
  return createAnnotation({
    anchor: { type: 'page', pageId: PAGE_ID, offsetX: 0.5, offsetY: 0.4 },
    text,
  })
}

/** The backend is awaited on a microtask after the call is queued. */
async function flush(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve()
}

describe('auto-fix on the canvas agent thread', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    __resetDevServerManagerForTests()
    // Threads live on disk beside the canvas and reload lazily, so a fresh
    // in-memory list alone would inherit the previous test's threads.
    rmSync(join(spaceDir(), '.specular', 'threads'), { recursive: true, force: true })
    _resetThreadsForTests()
    calls = []
    stubBackend()
    initFixOrchestrator()
    loadHostPage()
  })

  afterEach(async () => {
    // A parked run holds its thread in flight; release it so the next test
    // does not queue behind it.
    for (const call of calls) call.finish('cleanup')
    await flush()
    _setBackendOverride(null)
  })

  afterAll(() => harness?.dispose())

  it('sends a comment on an auto-fix origin at once, aimed at the pin and its repo', async () => {
    bindOriginToRepoPath(ORIGIN, REPO_PATH, true)

    const pin = comment('make the hero sticky')
    await flush()

    expect(calls).toHaveLength(1)
    expect(calls[0].prompt).toContain('make the hero sticky')
    expect(calls[0].prompt).toContain(`in the repo at ${REPO_PATH}`)
    expect(calls[0].prompt).toContain(`selected comment ${pin.id}`)

    const thread = getActiveThread()
    expect(thread?.annotationIds).toEqual([pin.id])
    expect(thread?.messages.some((message) => message.queued)).toBe(false)

    calls[0].finish('done')
    await flush()
    expect(getActiveThread()?.status).toBe('open')
    expect(getActiveThread()?.messages.at(-1)).toMatchObject({ role: 'agent', text: 'done' })
  })

  it('queues a comment behind a run in flight and drains it into the same session', async () => {
    bindOriginToRepoPath(ORIGIN, REPO_PATH, true)

    comment('first')
    await flush()
    const second = comment('second')
    await flush()

    expect(calls).toHaveLength(1)
    const thread = getActiveThread()
    expect(getAgentThreads()).toHaveLength(1)
    expect(thread?.messages.filter((message) => message.queued).map((m) => m.text)).toEqual(['second'])

    calls[0].finish('first done')
    await flush()

    expect(calls).toHaveLength(2)
    expect(calls[1].resumeSessionId).toBe('sess_auto')
    expect(calls[1].prompt).toContain('[User] second')
    expect(calls[1].prompt).toContain(`selected comment ${second.id}`)
    expect(getActiveThread()?.messages.some((message) => message.queued)).toBe(false)
  })

  it('only queues into a draft when the origin has not opted in', async () => {
    bindOriginToRepoPath(ORIGIN, REPO_PATH, false)

    comment('wait for me')
    await flush()

    expect(calls).toHaveLength(0)
    const thread = getActiveThread()
    expect(thread?.status).toBe('draft')
    expect(thread?.messages.map((message) => message.queued)).toEqual([true])
  })
})
