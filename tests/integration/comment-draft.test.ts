/**
 * The comment draft (ADR 0006 amendment): a canvas gesture opens a draft in
 * main; the sidebar composer finishes it. Submitting saves the comment
 * through the same `createAnnotation` → `setOnAnnotationCreated` →
 * fix-orchestrator path as any other comment, so it queues into the active
 * thread exactly like today; cancelling leaves no annotation behind.
 *
 * Mutation-verified by:
 * - making `submitCommentDraft` skip `clearCommentDraft()` — the "cleared
 *   after submit" assertion fails;
 * - dropping the `!trimmed && images.length === 0` guard — an empty submit
 *   would create a blank-text annotation instead of being a no-op.
 */

import { afterAll, beforeEach, describe, expect, it } from 'vitest'
import { rmSync } from 'node:fs'
import { join } from 'node:path'
import { bootWorkspaceHarness, settleSync, type WorkspaceHarness } from './harness'
import { initFixOrchestrator } from '../../src/main/agent-fix/fix-orchestrator'
import {
  clearCommentDraft,
  getCommentDraft,
  makeCommentDraftId,
  setCommentDraft,
  submitCommentDraft,
} from '../../src/main/runtime/comment-draft'
import { spaceDir } from '../../src/main/runtime/space-dir'
import { _resetThreadsForTests, getAgentThreads } from '../../src/main/agent-thread/thread-runtime'
import { workspaceAnnotations } from '../../src/main/runtime/space-model'

let harness: WorkspaceHarness

describe('comment draft', () => {
  beforeEach(() => {
    harness ??= bootWorkspaceHarness()
    harness.reset()
    // Threads live on disk beside the canvas and reload lazily, so a fresh
    // in-memory list alone would inherit the previous test's threads.
    rmSync(join(spaceDir(), '.specular', 'threads'), { recursive: true, force: true })
    _resetThreadsForTests()
    initFixOrchestrator()
  })

  afterAll(() => harness?.dispose())

  it('submits a canvas-point draft into the active thread and clears the draft', async () => {
    setCommentDraft({
      id: makeCommentDraftId(),
      kind: 'point',
      canvasX: 120,
      canvasY: 80,
      label: 'Canvas point',
    })
    expect(getCommentDraft()).not.toBeNull()

    submitCommentDraft('make this sticky')
    await settleSync()

    expect(getCommentDraft()).toBeNull()
    expect(workspaceAnnotations).toHaveLength(1)
    const annotation = workspaceAnnotations[0]
    expect(annotation.anchor).toEqual({ type: 'canvas', canvasX: 120, canvasY: 80 })
    expect(annotation.text).toBe('make this sticky')

    const threads = getAgentThreads()
    expect(threads).toHaveLength(1)
    const queued = threads[0].messages.find((m) => m.annotationId === annotation.id)
    expect(queued?.text).toBe('make this sticky')
    expect(queued?.queued).toBe(true)
  })

  it('cancelling a draft leaves no annotation behind', () => {
    setCommentDraft({
      id: makeCommentDraftId(),
      kind: 'region',
      canvasRect: { x: 0, y: 0, width: 200, height: 100 },
      label: 'Area',
    })
    expect(getCommentDraft()).not.toBeNull()

    clearCommentDraft()

    expect(getCommentDraft()).toBeNull()
    expect(workspaceAnnotations).toHaveLength(0)
    expect(getAgentThreads()).toHaveLength(0)
  })
})
