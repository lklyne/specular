/**
 * Disk home for canvas agent threads: one JSON file per thread under
 * `.specular/threads/<tab-id>/`, with a space-wide `index.json` holding the
 * active thread. Not Y.Doc — chat is not undoable canvas state.
 */

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'fs'
import { join } from 'path'
import type { AgentThread, AgentThreadMessage, AgentThreadStatus } from '../../shared/agent-thread'

const INDEX_FILE = 'index.json'

export type ThreadIndex = {
  activeThreadId: string | null
}

function threadsRoot(spacePath: string): string {
  return join(spacePath, '.specular', 'threads')
}

export function threadsDir(spacePath: string, tabId: string): string {
  return join(threadsRoot(spacePath), tabId)
}

export function loadThreads(spacePath: string): {
  threads: AgentThread[]
  activeThreadId: string | null
} {
  const root = threadsRoot(spacePath)
  if (!existsSync(root)) return { threads: [], activeThreadId: null }

  const threads: AgentThread[] = []
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    const dir = join(root, entry.name)
    for (const name of readdirSync(dir)) {
      if (!name.endsWith('.json') || name === INDEX_FILE) continue
      const parsed = readThreadFile(join(dir, name), entry.name)
      if (parsed) threads.push(parsed)
    }
  }
  threads.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))

  const index = readIndex(root)
  const activeThreadId =
    index.activeThreadId && threads.some((t) => t.id === index.activeThreadId)
      ? index.activeThreadId
      : null
  return { threads, activeThreadId }
}

export function writeThread(spacePath: string, thread: AgentThread): void {
  const dir = threadsDir(spacePath, thread.tabId)
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, `${thread.id}.json`), `${JSON.stringify(thread, null, 2)}\n`, 'utf8')
}

export function writeThreadIndex(spacePath: string, activeThreadId: string | null): void {
  const root = threadsRoot(spacePath)
  mkdirSync(root, { recursive: true })
  const index: ThreadIndex = { activeThreadId }
  writeFileSync(join(root, INDEX_FILE), `${JSON.stringify(index, null, 2)}\n`, 'utf8')
}

export function deleteThreadFile(spacePath: string, tabId: string, threadId: string): void {
  const path = join(threadsDir(spacePath, tabId), `${threadId}.json`)
  if (existsSync(path)) rmSync(path)
}

function readIndex(root: string): ThreadIndex {
  const path = join(root, INDEX_FILE)
  if (!existsSync(path)) return { activeThreadId: null }
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, 'utf8'))
    if (!parsed || typeof parsed !== 'object') return { activeThreadId: null }
    const active = (parsed as { activeThreadId?: unknown }).activeThreadId
    return { activeThreadId: typeof active === 'string' ? active : null }
  } catch {
    return { activeThreadId: null }
  }
}

function readThreadFile(path: string, tabId: string): AgentThread | null {
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, 'utf8'))
    return parseThread(parsed, tabId)
  } catch {
    return null
  }
}

export function parseThread(value: unknown, tabId: string): AgentThread | null {
  if (!value || typeof value !== 'object') return null
  const raw = value as Record<string, unknown>
  if (typeof raw.id !== 'string' || !raw.id) return null
  const status = parseStatus(raw.status)
  if (!status) return null
  const messages = parseMessages(raw.messages)
  const createdAt = typeof raw.createdAt === 'string' ? raw.createdAt : new Date().toISOString()
  const updatedAt = typeof raw.updatedAt === 'string' ? raw.updatedAt : createdAt
  const annotationIds = Array.isArray(raw.annotationIds)
    ? raw.annotationIds.filter((id): id is string => typeof id === 'string')
    : []
  return {
    id: raw.id,
    tabId: typeof raw.tabId === 'string' ? raw.tabId : tabId,
    title: typeof raw.title === 'string' && raw.title.trim() ? raw.title : 'New thread',
    status,
    createdAt,
    updatedAt,
    ...(typeof raw.claudeSessionId === 'string' ? { claudeSessionId: raw.claudeSessionId } : {}),
    annotationIds,
    messages,
  }
}

function parseStatus(value: unknown): AgentThreadStatus | null {
  if (value === 'draft' || value === 'open') return value
  // Files written before delete replaced archiving carry status 'closed';
  // surface them as open so old conversations stay reachable.
  if (value === 'closed') return 'open'
  return null
}

function parseMessages(value: unknown): AgentThreadMessage[] {
  if (!Array.isArray(value)) return []
  const messages: AgentThreadMessage[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object') continue
    const raw = item as Record<string, unknown>
    if (typeof raw.id !== 'string' || (raw.role !== 'user' && raw.role !== 'agent')) continue
    if (typeof raw.text !== 'string') continue
    messages.push({
      id: raw.id,
      role: raw.role,
      text: raw.text,
      createdAt: typeof raw.createdAt === 'string' ? raw.createdAt : new Date().toISOString(),
      ...(raw.queued === true ? { queued: true } : {}),
      ...(typeof raw.annotationId === 'string' ? { annotationId: raw.annotationId } : {}),
    })
  }
  return messages
}
