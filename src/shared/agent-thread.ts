/**
 * Canvas agent threads: a conversation on a canvas, persisted beside the
 * document (not in it). Comments queue into a draft; Send runs the agent.
 * Follow-up stays on the open thread; a new comment starts a new draft.
 */

export type AgentThreadStatus = 'draft' | 'open'
export type AgentThreadMessageRole = 'user' | 'agent'

/** Media types the model accepts as image input. */
const THREAD_IMAGE_MEDIA_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'] as const
export type ThreadImageMediaType = (typeof THREAD_IMAGE_MEDIA_TYPES)[number]

export function isThreadImageMediaType(value: unknown): value is ThreadImageMediaType {
  return (THREAD_IMAGE_MEDIA_TYPES as readonly unknown[]).includes(value)
}

/** A pasted image on its way from the composer to main, base64 without the data-URL prefix. */
export interface ThreadImageUpload {
  mediaType: ThreadImageMediaType
  data: string
}

/** A pasted image saved beside the thread, relative to the space folder. */
export interface AgentThreadImage {
  path: string
  mediaType: ThreadImageMediaType
}

export interface AgentThreadMessage {
  id: string
  role: AgentThreadMessageRole
  text: string
  createdAt: string
  /** True until the user hits Send on a draft. */
  queued?: boolean
  annotationId?: string
  images?: AgentThreadImage[]
}

/** A user message worth sending: words, pasted images, or both. */
export function messageHasContent(message: AgentThreadMessage): boolean {
  return Boolean(message.text.trim()) || Boolean(message.images?.length)
}

export interface AgentThread {
  id: string
  tabId: string
  title: string
  status: AgentThreadStatus
  createdAt: string
  updatedAt: string
  claudeSessionId?: string
  annotationIds: string[]
  messages: AgentThreadMessage[]
}

export type ThreadWriteTarget =
  | { kind: 'space' }
  | { kind: 'repo'; origin: string; repoPath: string }

export type ThreadPill =
  | { kind: 'dom'; label: string; origin: string | null; pageId?: string | null }
  | { kind: 'annotation'; label: string; annotationId: string }
  | { kind: 'selection'; label: string; entityIds: string[] }
  | { kind: 'empty' }

export interface ThreadPillInput {
  inspectNode?: { name: string; tagName: string; origin: string | null; pageId?: string | null } | null
  focusedAnnotation?: {
    id: string
    text: string
    elementName?: string
    anchorType: string
  } | null
  canvasSelection?: { count: number; label: string; entityIds: string[] } | null
}

function isDraftThread(thread: AgentThread | null | undefined): boolean {
  return thread?.status === 'draft'
}

/** A new comment starts a fresh draft unless the active thread is still unsent. */
export function shouldStartNewDraft(thread: AgentThread | null | undefined): boolean {
  return !isDraftThread(thread)
}

export function resolveThreadPill(input: ThreadPillInput): ThreadPill {
  const node = input.inspectNode
  if (node) {
    const label = node.name.trim() || node.tagName.trim() || 'element'
    return { kind: 'dom', label, origin: node.origin, pageId: node.pageId ?? null }
  }
  const annotation = input.focusedAnnotation
  if (annotation) {
    const snippet = annotation.text.replace(/\s+/g, ' ').trim()
    const label =
      annotation.elementName?.trim() ||
      (snippet ? truncate(snippet, 48) : annotationLabel(annotation.anchorType))
    return { kind: 'annotation', label, annotationId: annotation.id }
  }
  const selection = input.canvasSelection
  if (selection && selection.count > 0) {
    return { kind: 'selection', label: selection.label, entityIds: selection.entityIds ?? [] }
  }
  return { kind: 'empty' }
}

export function threadTitleFromMessages(messages: AgentThreadMessage[]): string {
  const first = messages.find((m) => m.role === 'user' && m.text.trim())
  if (!first) return messages.some((m) => m.images?.length) ? 'Image' : 'New thread'
  return truncate(first.text.replace(/\s+/g, ' ').trim(), 48)
}

export function pillLabel(pill: ThreadPill): string {
  switch (pill.kind) {
    case 'dom':
      return pill.label
    case 'annotation':
      return pill.label
    case 'selection':
      return pill.label
    case 'empty':
      return 'specular'
    default: {
      const exhaustive: never = pill
      return exhaustive
    }
  }
}

/** Soft focus for the agent prompt. Not a write fence. */
export function selectionFocusPrompt(pill: ThreadPill): string | null {
  if (pill.kind === 'selection' && pill.entityIds.length > 0) {
    return `The user has selected ${pill.entityIds.join(', ')} and likely wants to focus on those.`
  }
  if (pill.kind === 'annotation') {
    return `The user has selected comment ${pill.annotationId} and likely wants to focus on that.`
  }
  if (pill.kind === 'dom') {
    const where = pill.pageId ? ` on page ${pill.pageId}` : ''
    return `The user has selected DOM node "${pill.label}"${where} and likely wants to focus on that.`
  }
  return null
}

function annotationLabel(anchorType: string): string {
  if (anchorType === 'element') return 'Element'
  if (anchorType === 'region') return 'Region'
  if (anchorType === 'page') return 'Page'
  return 'Comment'
}

function truncate(value: string, max: number): string {
  if (value.length <= max) return value
  return value.slice(0, max - 1) + '…'
}
