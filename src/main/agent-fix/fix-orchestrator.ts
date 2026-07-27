import type { Annotation, FixProgressEvent } from '../../shared/types'
import { annotationOrigin, truncate } from '../../shared/annotation-utils'
import {
  addAnnotationReply,
  getAnnotationById,
  getAnnotations,
  setAnnotationFixSession,
  setOnAnnotationCreated,
  setOnAnnotationReply,
  updateAnnotationStatus,
} from '../workspace-annotations'
import { getOriginBindingView as getOriginBinding } from '../runtime/dev-server-manager'
import { buildFixPrompt, buildFollowUpPrompt } from './prompt-builder'
import { fixTargetKey, resolveFixTarget, type FixTarget } from './fix-target'
import { resolveSelectionContext } from './selection-context'
import { invokeClaude, type FixResult } from './claude-spawner'
import {
  isAnnotationInFlight,
  markFixFinished,
  markFixStarted,
} from './fix-tracker'
import {
  appendFixEvent,
  finalizeFixProgress,
  startFixProgress,
} from './fix-progress'

const MAX_AGENT_REPLIES = 20

/**
 * Auto-fix is an opt-in that lives on an origin→repo binding, so only
 * page-bound comments can fire on their own. A comment targeting a file in the
 * user's space folder has nothing to opt in with and runs only when the user
 * asks (`POST /annotations/fix`).
 */
export function initFixOrchestrator(): void {
  setOnAnnotationCreated((annotation) => {
    if (annotation.author !== 'user') return
    const origin = annotationOrigin(annotation)
    if (!origin) return
    const binding = getOriginBinding(origin)
    if (!binding || !binding.autoFix) return
    fixAnnotation(annotation.id)
  })
  setOnAnnotationReply((annotation, reply) => {
    if (reply.author !== 'user') return
    if (annotation.status === 'dismissed') return
    const origin = annotationOrigin(annotation)
    if (!origin) return
    const binding = getOriginBinding(origin)
    if (!binding || !binding.autoFix) return
    fixAnnotationCore(annotation, { followUpText: reply.text })
  })
}

export function fixAnnotation(annotationId: string): boolean {
  const annotation = getAnnotationById(annotationId)
  if (!annotation) return false
  return fixAnnotationCore(annotation)
}

export function fixPendingAnnotationsForOrigin(origin: string): number {
  const binding = getOriginBinding(origin)
  if (!binding) return 0
  const candidates = getAnnotations().filter((a) => {
    if (a.status === 'resolved' || a.status === 'dismissed') return false
    return annotationOrigin(a) === origin
  })
  let queued = 0
  for (const candidate of candidates) {
    if (fixAnnotationCore(candidate)) queued++
  }
  return queued
}

function fixAnnotationCore(
  annotation: Annotation,
  opts?: { followUpText?: string },
): boolean {
  if (isAnnotationInFlight(annotation.id)) return false

  const target = resolveFixTarget(annotation, getOriginBinding)
  if (!target) {
    const origin = annotationOrigin(annotation)
    addAnnotationReply(
      annotation.id,
      'agent',
      origin
        ? `Cannot fix: no repo linked to ${origin}. Link one in the Comments panel.`
        : 'Cannot fix: annotation has no associated page URL.',
    )
    return false
  }
  const agentReplies = annotation.replies.filter((r) => r.author === 'agent').length
  if (agentReplies >= MAX_AGENT_REPLIES) {
    addAnnotationReply(annotation.id, 'agent', 'Agent reply cap reached. Resolve manually or reopen with a new comment.')
    return false
  }

  // A thread that has been fixed before carries its Claude session id; resume it
  // so the agent keeps its prior context. The first fix (or a stale session)
  // falls back to the full-context prompt.
  const resumeSessionId = annotation.metadata?.fixSessionId
  const fullPrompt = buildFixPrompt(annotation, {
    selection: resolveSelectionContext(annotation),
    target,
  })
  const prompt = resumeSessionId
    ? buildFollowUpPrompt(opts?.followUpText ?? latestUserReplyText(annotation))
    : fullPrompt

  const trackingKey = fixTargetKey(target)
  updateAnnotationStatus(annotation.id, 'acknowledged')
  startFixProgress(annotation.id, trackingKey)
  markFixStarted(annotation.id, trackingKey)
  void runFix(annotation.id, target, { prompt, resumeSessionId, fullPrompt })
  return true
}

function latestUserReplyText(annotation: Annotation): string {
  for (let i = annotation.replies.length - 1; i >= 0; i--) {
    if (annotation.replies[i].author === 'user') return annotation.replies[i].text
  }
  return ''
}

async function runFix(
  annotationId: string,
  target: FixTarget,
  plan: { prompt: string; resumeSessionId?: string; fullPrompt: string },
): Promise<void> {
  let result: FixResult | null = null
  let error: Error | null = null
  const trackingKey = fixTargetKey(target)
  const onEvent = (event: FixProgressEvent) =>
    appendFixEvent(annotationId, event.kind, event.text)
  try {
    result = await invokeClaude(plan.prompt, target.cwd, { resumeSessionId: plan.resumeSessionId, onEvent })
  } catch (err) {
    error = err instanceof Error ? err : new Error(String(err))
    // A stale/missing session can't be resumed (cleaned up, or the .canvas
    // moved to another machine). Retry once from scratch with full context.
    if (plan.resumeSessionId) {
      appendFixEvent(annotationId, 'system', 'Could not resume prior session — starting fresh.')
      error = null
      try {
        result = await invokeClaude(plan.fullPrompt, target.cwd, { onEvent })
      } catch (retryErr) {
        error = retryErr instanceof Error ? retryErr : new Error(String(retryErr))
      }
    }
  } finally {
    markFixFinished(annotationId, trackingKey)
  }
  if (result?.sessionId) setAnnotationFixSession(annotationId, result.sessionId)
  handleCompletion(annotationId, result, error)
}

function handleCompletion(
  annotationId: string,
  result: FixResult | null,
  error: Error | null,
): void {
  if (error || !result) {
    const message = error ? error.message : 'Unknown error from fix runner.'
    const shortMessage = truncate(message, 240)
    appendFixEvent(annotationId, 'error', shortMessage)
    finalizeFixProgress(annotationId, 'failed', { error: shortMessage })
    addAnnotationReply(annotationId, 'agent', `Fix failed: ${shortMessage}`)
    return
  }
  // shouldResolve is the agent's own "I believe this is fixed" signal — kept on
  // the progress entry as a hint, but resolving the thread is the user's call
  // (the comment row's ⋮ → Resolve). We never auto-resolve.
  finalizeFixProgress(annotationId, 'completed', {
    summary: result.summary,
    shouldResolve: result.shouldResolve,
  })
  addAnnotationReply(annotationId, 'agent', result.summary)
}

