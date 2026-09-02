/**
 * In-memory agent threads for the whole space — every canvas tab's threads
 * share one list. Persist under `.specular/threads/` — not the Y.Doc.
 */

import { randomUUID } from 'crypto'
import type { Annotation, FixProgressEvent } from '../../shared/types'
import {
  resolveThreadPill,
  shouldStartNewDraft,
  threadTitleFromMessages,
  type AgentThread,
  type ThreadPill,
  type ThreadPillInput,
  type ThreadWriteTarget,
} from '../../shared/agent-thread'
import { annotationOrigin } from '../../shared/annotation-utils'
import { runFixAgent } from '../agent-fix/agent-backend'
import {
  appendFixEvent,
  finalizeFixProgress,
  startFixProgress,
} from '../agent-fix/fix-progress'
import {
  isAnnotationInFlight,
  markFixFinished,
  markFixStarted,
} from '../agent-fix/fix-tracker'
import { inferRepoPathForOrigin } from '../runtime/dev-server-manager'
import {
  findPageById,
  inspectSelectedTarget,
  pages,
} from '../runtime/runtime-context'
import { spaceDir } from '../runtime/space-dir'
import { activeSpaceTabId, workspaceAnnotations } from '../runtime/space-model'
import {
  focusedAnnotationId as uiFocusedAnnotationId,
  getUiState,
} from '../ui-state'
import { buildThreadFollowUpPrompt, buildThreadPrompt } from './thread-prompt'
import {
  deleteThreadFile,
  loadThreads,
  writeThread,
  writeThreadIndex,
} from './thread-store'

function makeId(prefix: string): string {
  return `${prefix}_${randomUUID()}`
}

let threads: AgentThread[] = []
let activeThreadId: string | null = null
let loaded = false

type ChangeListener = () => void
const listeners = new Set<ChangeListener>()

export function onThreadChange(fn: ChangeListener): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

function notify(): void {
  for (const fn of listeners) {
    try {
      fn()
    } catch (err) {
      console.error('thread-runtime listener error:', err)
    }
  }
}

function persist(thread: AgentThread): void {
  writeThread(spaceDir(), thread)
  writeThreadIndex(spaceDir(), activeThreadId)
}

export function loadThreadsFromDisk(): void {
  const result = loadThreads(spaceDir())
  threads = result.threads
  activeThreadId = result.activeThreadId
  loaded = true
  notify()
}

function ensureThreadsLoaded(): void {
  if (!loaded) loadThreadsFromDisk()
}

export function getAgentThreads(): AgentThread[] {
  return [...threads].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
}

export function getActiveThreadId(): string | null {
  return activeThreadId
}

export function getActiveThread(): AgentThread | null {
  return threads.find((thread) => thread.id === activeThreadId) ?? null
}

export function newAgentThread(): AgentThread | null {
  const tabId = activeSpaceTabId
  if (!tabId) return null
  ensureThreadsLoaded()
  const now = new Date().toISOString()
  const thread: AgentThread = {
    id: makeId('thread'),
    tabId,
    title: 'New thread',
    status: 'draft',
    createdAt: now,
    updatedAt: now,
    annotationIds: [],
    messages: [],
  }
  threads = [thread, ...threads]
  activeThreadId = thread.id
  persist(thread)
  notify()
  return thread
}

/** Back out to the thread list without archiving anything. */
export function deselectAgentThread(): void {
  ensureThreadsLoaded()
  if (activeThreadId === null) return
  activeThreadId = null
  writeThreadIndex(spaceDir(), null)
  notify()
}

export function deleteAgentThread(threadId: string): void {
  ensureThreadsLoaded()
  const thread = threads.find((candidate) => candidate.id === threadId)
  if (!thread) return
  threads = threads.filter((candidate) => candidate.id !== threadId)
  deleteThreadFile(spaceDir(), thread.tabId, threadId)
  if (activeThreadId === threadId) activeThreadId = null
  writeThreadIndex(spaceDir(), activeThreadId)
  notify()
}

export function selectAgentThread(threadId: string): void {
  ensureThreadsLoaded()
  const thread = threads.find((candidate) => candidate.id === threadId)
  if (!thread) return
  activeThreadId = thread.id
  writeThreadIndex(spaceDir(), activeThreadId)
  notify()
}

export function selectThreadForAnnotation(annotationId: string): void {
  ensureThreadsLoaded()
  const annotation = workspaceAnnotations.find((item) => item.id === annotationId)
  const threadId = annotation?.metadata?.threadId
  if (typeof threadId !== 'string') return
  const existing = threads.find((thread) => thread.id === threadId)
  if (existing) selectAgentThread(existing.id)
}

export function queueCommentOnAnnotation(annotation: Annotation): string | null {
  const tabId = activeSpaceTabId
  if (!tabId) return null
  ensureThreadsLoaded()
  let thread = getActiveThread()
  if (shouldStartNewDraft(thread)) {
    thread = newAgentThread()
  }
  if (!thread) return null
  appendQueuedUserMessage(thread, annotation.text.trim() || '(comment)', annotation.id)
  return thread.id
}

/** Pin reply or HTTP follow-up: stay on the annotation's thread, do not send. */
export function queueReplyOnAnnotation(annotation: Annotation, text: string): string | null {
  const trimmed = text.trim()
  if (!trimmed) return null
  const tabId = activeSpaceTabId
  if (!tabId) return null
  ensureThreadsLoaded()
  const existingId =
    typeof annotation.metadata?.threadId === 'string' ? annotation.metadata.threadId : null
  const existing = existingId
    ? threads.find((thread) => thread.id === existingId)
    : undefined
  if (!existing) return queueCommentOnAnnotation({ ...annotation, text: trimmed })
  appendQueuedUserMessage(existing, trimmed, annotation.id)
  if (activeThreadId !== existing.id) selectAgentThread(existing.id)
  return existing.id
}

function appendQueuedUserMessage(thread: AgentThread, text: string, annotationId: string): void {
  const now = new Date().toISOString()
  thread.messages.push({
    id: makeId('tmsg'),
    role: 'user',
    text,
    createdAt: now,
    queued: true,
    annotationId,
  })
  if (!thread.annotationIds.includes(annotationId)) {
    thread.annotationIds.push(annotationId)
  }
  thread.title = threadTitleFromMessages(thread.messages)
  thread.updatedAt = now
  persist(thread)
  notify()
}

export function sendActiveThread(composerText?: string): boolean {
  const tabId = activeSpaceTabId
  if (!tabId) return false
  ensureThreadsLoaded()
  let thread = getActiveThread()
  const extra = composerText?.trim() ?? ''
  if (!thread) {
    if (!extra) return false
    thread = newAgentThread()
  }
  if (!thread) return false
  if (isAnnotationInFlight(thread.id)) return false

  if (extra) {
    const now = new Date().toISOString()
    thread.messages.push({
      id: makeId('tmsg'),
      role: 'user',
      text: extra,
      createdAt: now,
      queued: thread.status === 'draft',
    })
    thread.title = threadTitleFromMessages(thread.messages)
    thread.updatedAt = now
    persist(thread)
    notify()
  }

  if (!thread.messages.some((message) => message.role === 'user' && message.text.trim())) {
    return false
  }

  const pill = captureThreadPill()
  const writeTarget = resolveWriteTarget(pill)
  const progressKey = writeTarget.kind === 'repo' ? writeTarget.origin : 'space'
  const resumeSessionId = thread.status === 'open' ? thread.claudeSessionId : undefined
  const fullPrompt = buildThreadPrompt({
    thread,
    pill,
    writeTarget,
    spacePath: spaceDir(),
  })
  const lastUser = [...thread.messages].reverse().find((message) => message.role === 'user')
  const prompt = resumeSessionId && lastUser
    ? buildThreadFollowUpPrompt(lastUser.text, pill)
    : fullPrompt

  startFixProgress(thread.id, progressKey)
  markFixStarted(thread.id, progressKey)
  void runThreadAgent(thread.id, {
    prompt,
    fullPrompt,
    resumeSessionId,
    cwd: spaceDir(),
    progressKey,
  })
  return true
}

async function runThreadAgent(
  threadId: string,
  plan: {
    prompt: string
    fullPrompt: string
    resumeSessionId?: string
    cwd: string
    progressKey: string
  },
): Promise<void> {
  const onEvent = (event: FixProgressEvent) =>
    appendFixEvent(threadId, event.kind, event.text)
  let result: { summary: string; sessionId?: string } | null = null
  let error: Error | null = null
  try {
    result = await runFixAgent(plan.prompt, plan.cwd, {
      resumeSessionId: plan.resumeSessionId,
      onEvent,
    })
  } catch (err) {
    error = err instanceof Error ? err : new Error(String(err))
    if (plan.resumeSessionId) {
      appendFixEvent(threadId, 'system', 'Could not resume prior session — starting fresh.')
      error = null
      try {
        result = await runFixAgent(plan.fullPrompt, plan.cwd, { onEvent })
      } catch (retryErr) {
        error = retryErr instanceof Error ? retryErr : new Error(String(retryErr))
      }
    }
  } finally {
    markFixFinished(threadId, plan.progressKey)
  }

  const thread = threads.find((candidate) => candidate.id === threadId)
  if (!thread) {
    finalizeFixProgress(threadId, 'failed', { error: 'Thread gone' })
    notify()
    return
  }

  if (error || !result) {
    const message = error ? error.message : 'Unknown error from agent.'
    finalizeFixProgress(threadId, 'failed', { error: message })
    notify()
    return
  }

  const now = new Date().toISOString()
  for (const message of thread.messages) {
    delete message.queued
  }
  thread.messages.push({
    id: makeId('tmsg'),
    role: 'agent',
    text: result.summary,
    createdAt: now,
  })
  thread.status = 'open'
  thread.updatedAt = now
  if (result.sessionId) thread.claudeSessionId = result.sessionId
  persist(thread)
  finalizeFixProgress(threadId, 'completed', { summary: result.summary })
  notify()
}

export function captureThreadPill(): ThreadPill {
  return resolveThreadPill(captureThreadPillInput())
}

export function captureThreadPillInput(): ThreadPillInput {
  const inspect = inspectSelectedTarget
  let origin: string | null = null
  if (inspect) {
    const page = findPageById(inspect.pageId)
    origin = originFromUrl(page?.url)
  }

  const focusedId = uiFocusedAnnotationId()
  const focused = focusedId
    ? workspaceAnnotations.find((item) => item.id === focusedId)
    : undefined
  const selection = getUiState().selection
  let canvasSelection: ThreadPillInput['canvasSelection'] = null
  if (selection.kind === 'single-entity') {
    canvasSelection = {
      count: 1,
      label: selection.entityKind,
      entityIds: [selection.entityId],
    }
  } else if (selection.kind === 'multi-entity') {
    canvasSelection = {
      count: selection.entityIds.length,
      label: `${selection.entityIds.length} items`,
      entityIds: [...selection.entityIds],
    }
  }

  return {
    inspectNode: inspect
      ? { name: inspect.name, tagName: inspect.tagName, origin, pageId: inspect.pageId }
      : null,
    focusedAnnotation: focused
      ? {
          id: focused.id,
          text: focused.text,
          elementName: focused.elementName,
          anchorType: focused.anchor.type,
        }
      : null,
    canvasSelection,
  }
}

export function resolveWriteTarget(pill: ThreadPill): ThreadWriteTarget {
  const origin = originForPill(pill)
  if (!origin) return { kind: 'space' }
  const path = inferRepoPathForOrigin(origin)
  if (path) return { kind: 'repo', origin, repoPath: path }
  return { kind: 'space' }
}

function originForPill(pill: ThreadPill): string | null {
  if (pill.kind === 'dom') return pill.origin
  if (pill.kind === 'annotation') {
    const annotation = workspaceAnnotations.find((item) => item.id === pill.annotationId)
    return annotation ? annotationOrigin(annotation) : null
  }
  if (pill.kind !== 'selection') return null
  const selection = getUiState().selection
  if (selection.kind === 'single-entity' && selection.entityKind === 'page') {
    return originFromUrl(findPageById(selection.entityId)?.url)
  }
  return originFromUrl(pages[0]?.url)
}

function originFromUrl(url: string | undefined): string | null {
  if (!url) return null
  try {
    return new URL(url).origin
  } catch {
    return null
  }
}

/** Test hook: drop in-memory state without touching disk. */
export function _resetThreadsForTests(): void {
  threads = []
  activeThreadId = null
  loaded = false
}
