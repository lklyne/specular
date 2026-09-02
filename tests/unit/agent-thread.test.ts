/**
 * Canvas agent thread: pill specificity, draft start, disk parse.
 *
 * Mutation-verified by swapping DOM vs annotation order in resolveThreadPill
 * (DOM-vs-annotation test fails), dropping 'draft' from parseStatus
 * (round-trip test fails), mapping legacy 'closed' to null instead of 'open'
 * (legacy test fails), and dropping selectionFocusPrompt's id list
 * (focus prompt test fails).
 */

import { afterEach, describe, expect, it } from 'vitest'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  pillLabel,
  resolveThreadPill,
  selectionFocusPrompt,
  shouldStartNewDraft,
  threadTitleFromMessages,
  type AgentThread,
  type AgentThreadMessage,
} from '../../src/shared/agent-thread'
import { buildThreadPrompt } from '../../src/main/agent-thread/thread-prompt'
import {
  loadThreads,
  parseThread,
  writeThread,
  writeThreadIndex,
} from '../../src/main/agent-thread/thread-store'

function message(partial: Partial<AgentThreadMessage> & Pick<AgentThreadMessage, 'id' | 'role' | 'text'>): AgentThreadMessage {
  return { createdAt: '2026-09-02T00:00:00.000Z', ...partial }
}

function thread(partial: Partial<AgentThread> = {}): AgentThread {
  return {
    id: 'thread_1',
    tabId: 'tab_1',
    title: 'New thread',
    status: 'draft',
    createdAt: '2026-09-02T00:00:00.000Z',
    updatedAt: '2026-09-02T00:00:00.000Z',
    annotationIds: [],
    messages: [],
    ...partial,
  }
}

describe('resolveThreadPill', () => {
  const inspectNode = { name: 'Header', tagName: 'header', origin: 'http://localhost:4321' }
  const focusedAnnotation = {
    id: 'ann_1',
    text: 'the nav is too tight',
    elementName: 'Nav',
    anchorType: 'element',
  }
  const canvasSelection = { count: 1, label: 'page', entityIds: ['page_1'] }

  it('prefers a DOM node over a focused comment and canvas selection', () => {
    expect(resolveThreadPill({ inspectNode, focusedAnnotation, canvasSelection })).toEqual({
      kind: 'dom',
      label: 'Header',
      origin: 'http://localhost:4321',
      pageId: null,
    })
  })

  it('prefers a focused comment over canvas selection', () => {
    expect(resolveThreadPill({ focusedAnnotation, canvasSelection })).toEqual({
      kind: 'annotation',
      label: 'Nav',
      annotationId: 'ann_1',
    })
  })

  it('falls back to canvas selection, then empty specular', () => {
    expect(resolveThreadPill({ canvasSelection })).toEqual({
      kind: 'selection',
      label: 'page',
      entityIds: ['page_1'],
    })
    expect(resolveThreadPill({})).toEqual({ kind: 'empty' })
    expect(pillLabel({ kind: 'empty' })).toBe('specular')
  })

  it('uses comment text when the pin has no element name', () => {
    expect(
      resolveThreadPill({
        focusedAnnotation: { ...focusedAnnotation, elementName: undefined },
      }),
    ).toMatchObject({ kind: 'annotation', label: 'the nav is too tight' })
  })
})

describe('shouldStartNewDraft', () => {
  it('starts a new draft unless the active thread is still unsent', () => {
    expect(shouldStartNewDraft(null)).toBe(true)
    expect(shouldStartNewDraft(thread({ status: 'open' }))).toBe(true)
    expect(shouldStartNewDraft(thread({ status: 'draft' }))).toBe(false)
  })
})

describe('threadTitleFromMessages', () => {
  it('takes the first user message and stays on New thread when empty', () => {
    expect(threadTitleFromMessages([])).toBe('New thread')
    expect(
      threadTitleFromMessages([
        message({ id: 'm1', role: 'agent', text: 'ignored' }),
        message({ id: 'm2', role: 'user', text: '  make the header sticky  ' }),
      ]),
    ).toBe('make the header sticky')
  })
})

describe('selectionFocusPrompt', () => {
  it('names selected entity ids as likely focus, not a fence', () => {
    expect(
      selectionFocusPrompt({
        kind: 'selection',
        label: '2 items',
        entityIds: ['text_a', 'text_b'],
      }),
    ).toBe('The user has selected text_a, text_b and likely wants to focus on those.')
  })

  it('is silent when nothing is selected', () => {
    expect(selectionFocusPrompt({ kind: 'empty' })).toBeNull()
  })
})

describe('buildThreadPrompt', () => {
  it('puts selected ids in the prompt as focus, not a fence', () => {
    const prompt = buildThreadPrompt({
      thread: thread({
        messages: [message({ id: 'm1', role: 'user', text: 'make them blue' })],
      }),
      pill: { kind: 'selection', label: '2 items', entityIds: ['text_a', 'text_b'] },
      writeTarget: { kind: 'space' },
      spacePath: '/tmp/space',
    })
    expect(prompt).toContain(
      'The user has selected text_a, text_b and likely wants to focus on those.',
    )
    expect(prompt).toContain('Prefer changing those.')
    expect(prompt).not.toContain('only update these ids')
  })
})

describe('thread-store', () => {
  let dir: string | null = null
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true })
    dir = null
  })

  it('parseThread drops garbage and round-trips a draft with queued comments', () => {
    expect(parseThread(null, 'tab_1')).toBeNull()
    expect(parseThread({ id: 'x' }, 'tab_1')).toBeNull()
    expect(parseThread({ id: 'x', status: 'nope', messages: [] }, 'tab_1')).toBeNull()

    const saved = thread({
      title: 'make the header sticky',
      annotationIds: ['ann_1'],
      claudeSessionId: 'sess_1',
      messages: [
        message({
          id: 'm1',
          role: 'user',
          text: 'make the header sticky',
          queued: true,
          annotationId: 'ann_1',
        }),
      ],
    })
    const parsed = parseThread(JSON.parse(JSON.stringify(saved)), 'tab_other')
    expect(parsed).toEqual(saved)
  })

  it('legacy closed threads parse as open so they stay reachable', () => {
    const legacy = { ...thread({ id: 'thread_legacy' }), status: 'closed' }
    expect(parseThread(JSON.parse(JSON.stringify(legacy)), 'tab_1')?.status).toBe('open')
  })

  it('writeThread / loadThreads gather every tab, sorted by last use', () => {
    dir = mkdtempSync(join(tmpdir(), 'specular-threads-'))
    const open = thread({ id: 'thread_open', status: 'open', title: 'open one' })
    const otherTab = thread({
      id: 'thread_other',
      tabId: 'tab_2',
      status: 'draft',
      title: 'draft on another tab',
      updatedAt: '2026-09-02T01:00:00.000Z',
    })
    writeThread(dir, open)
    writeThread(dir, otherTab)
    writeThreadIndex(dir, 'thread_other')

    const loaded = loadThreads(dir)
    expect(loaded.activeThreadId).toBe('thread_other')
    expect(loaded.threads.map((item) => item.id)).toEqual(['thread_other', 'thread_open'])
  })

  it('no index means no active thread — the panel shows the list', () => {
    dir = mkdtempSync(join(tmpdir(), 'specular-threads-'))
    writeThread(dir, thread({ id: 'thread_only' }))
    expect(loadThreads(dir).activeThreadId).toBeNull()
  })
})
