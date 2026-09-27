/**
 * The in-progress comment draft: opened by a canvas gesture (element/point
 * click, region drag, the selection popup's Annotate button), finished by the
 * sidebar composer. Ephemeral runtime state, not Y.Doc — a draft that never
 * gets submitted leaves nothing behind. One draft at a time; opening a new
 * one replaces whatever was open.
 */

import type { ThreadImageUpload } from '../../shared/agent-thread'
import type { CommentDraft } from '../../shared/comment-draft'
import type { Annotation, AnnotationCreateRequest } from '../../shared/types'
import { ipcChannels } from '../../shared/ipc-contract'
import { createAnnotation } from '../workspace-annotations'
import { executeRegionSelect } from './region-select'
import { annotateSelectionRegion } from './annotate-selection'
import { queueImagesOnThread } from '../agent-thread/thread-runtime'
import { focusAnnotation, openCommentsPanel } from './devtools-panel'
import { aboveView } from './view-refs'
import { requestLayout } from './viewport-control'
import { setCommentDraftActions, setCommentDraftSnapshot } from './comment-draft-signal'

let currentDraft: CommentDraft | null = null

export function makeCommentDraftId(): string {
  return `draft:${Math.random().toString(36).slice(2, 10)}:${Date.now().toString(36)}`
}

export function getCommentDraft(): CommentDraft | null {
  return currentDraft
}

function broadcastDraft(): void {
  setCommentDraftSnapshot(currentDraft)
  if (aboveView && !aboveView.webContents.isDestroyed()) {
    aboveView.webContents.send(ipcChannels.commentDraftChanged, currentDraft)
  }
  requestLayout()
}

/**
 * Open a draft, replacing any other. Element/point drafts also drop the
 * canvas's focused-thread ring — the two composers are mutually exclusive UI
 * (mirrors the pre-sidebar behavior for those two anchor kinds; a region or
 * selection draft leaves an existing focused thread alone).
 */
export function setCommentDraft(draft: CommentDraft): void {
  currentDraft = draft
  if (draft.kind === 'element' || draft.kind === 'point') {
    focusAnnotation(undefined)
    if (aboveView && !aboveView.webContents.isDestroyed()) {
      aboveView.webContents.send(ipcChannels.annotationThreadOpen, { annotationId: null })
    }
  }
  openCommentsPanel()
  broadcastDraft()
}

export function clearCommentDraft(): void {
  if (!currentDraft) return
  currentDraft = null
  broadcastDraft()
}

/**
 * Leaving the comment tool drops a point/element/region draft — it was
 * anchored to a gesture that tool no longer owns. A selection draft came from
 * a popup that can mount under any tool, so it survives the switch; only an
 * explicit cancel or submit clears it.
 */
export function clearCommentDraftOnToolChange(): void {
  if (currentDraft && currentDraft.kind !== 'selection') clearCommentDraft()
}

function attachImages(annotation: Annotation, images: ThreadImageUpload[]): void {
  if (!images.length) return
  const threadId = annotation.metadata?.threadId
  if (typeof threadId === 'string') queueImagesOnThread(threadId, images)
}

/**
 * Save the draft: text may be empty only when images carry the comment.
 * Region and selection creation capture a screenshot first (async) — the
 * draft closes immediately either way, matching the pre-sidebar composer's
 * fire-and-forget submit.
 */
export function submitCommentDraft(text: string, images: ThreadImageUpload[] = []): boolean {
  const draft = currentDraft
  if (!draft) return false
  const trimmed = text.trim()
  if (!trimmed && images.length === 0) return false

  switch (draft.kind) {
    case 'element': {
      const request: AnnotationCreateRequest = { ...draft.request, text: trimmed }
      attachImages(createAnnotation(request), images)
      break
    }
    case 'point': {
      const request: AnnotationCreateRequest = {
        anchor: { type: 'canvas', canvasX: draft.canvasX, canvasY: draft.canvasY },
        text: trimmed,
      }
      attachImages(createAnnotation(request), images)
      break
    }
    case 'region':
      executeRegionSelect(draft.canvasRect, trimmed)
        .then((annotation) => attachImages(annotation, images))
        .catch((err) => console.error('[comment-draft] region submit failed:', err))
      break
    case 'selection':
      annotateSelectionRegion({ entityIds: draft.entityIds, text: trimmed })
        .then((annotation) => attachImages(annotation, images))
        .catch((err) => console.error('[comment-draft] selection submit failed:', err))
      break
  }
  clearCommentDraft()
  return true
}

setCommentDraftActions({
  clear: clearCommentDraft,
  clearOnToolChange: clearCommentDraftOnToolChange,
})
