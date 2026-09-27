/**
 * A dependency-free mirror of the comment draft (`comment-draft.ts`), read by
 * low-level runtime modules — the binding dispatcher, the focus reconciler,
 * tool-mode — without those modules importing `comment-draft.ts` itself.
 * `comment-draft.ts` pulls in annotation creation (`workspace-annotations.ts`,
 * which imports `mutate-workspace.ts`); those low-level modules sit inside
 * `mutate-workspace.ts`'s own import chain (via `viewport-control.ts` →
 * `layout-engine.ts` → `focus-reconciler-runtime.ts` → `binding-dispatcher.ts`
 * → `binding-handlers.ts` → `tool-mode.ts` → `inspect-session.ts`), so an
 * import back to `comment-draft.ts` from any of them closes a cycle through
 * `mutate-workspace.ts` before its own `let gestureSessionActive` has run —
 * the same TDZ hazard that probe documents on itself. This file has no
 * imports, so it can never be part of that cycle.
 *
 * `comment-draft.ts` is the only writer.
 */

import type { CommentDraft } from '../../shared/comment-draft'

let snapshot: CommentDraft | null = null
let clearOnToolChange: () => void = () => {}
let clear: () => void = () => {}

export function setCommentDraftSnapshot(next: CommentDraft | null): void {
  snapshot = next
}

export function commentDraftSnapshot(): CommentDraft | null {
  return snapshot
}

/** Registered once by `comment-draft.ts` at module load. */
export function setCommentDraftActions(actions: {
  clear: () => void
  clearOnToolChange: () => void
}): void {
  clear = actions.clear
  clearOnToolChange = actions.clearOnToolChange
}

export function clearCommentDraftViaSignal(): void {
  clear()
}

export function clearCommentDraftOnToolChangeViaSignal(): void {
  clearOnToolChange()
}
