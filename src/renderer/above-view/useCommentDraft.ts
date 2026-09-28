import { useEffect, useState } from 'react'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import type { CommentDraft } from '../../shared/comment-draft'

/**
 * The in-progress comment draft, mirrored from main. Main owns opening,
 * clearing, and submitting it (a canvas gesture opens one, the sidebar
 * composer finishes it); the canvas only draws a passive marker from it.
 */
export function useCommentDraft(api: CanvasBgElectronAPI): CommentDraft | null {
  const [draft, setDraft] = useState<CommentDraft | null>(null)
  useEffect(() => api.onCommentDraftChanged(setDraft), [api])
  return draft
}
