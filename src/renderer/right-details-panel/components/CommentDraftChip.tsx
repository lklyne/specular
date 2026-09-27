import { MessageSquare, SquareDashed, X } from 'lucide-react'
import type { CommentDraft } from '../../../shared/comment-draft'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'

/**
 * The open comment draft, shown above the message field as a removable chip
 * — styled like a pasted-image thumbnail (hover reveals the X), but a single
 * labeled pill since a draft carries no preview of its own. Region and
 * selection drafts get the dashed-square glyph that matches their canvas
 * marker; element and canvas-point drafts get a plain comment glyph.
 */
export function CommentDraftChip({ draft }: { draft: CommentDraft }) {
  const Icon = draft.kind === 'region' || draft.kind === 'selection' ? SquareDashed : MessageSquare
  return (
    <div className="group relative mb-1.5 inline-flex max-w-full items-center gap-1.5 rounded-full border border-zinc-300 bg-zinc-100 py-1 pl-2 pr-6 text-[12px] font-medium text-[var(--surface-foreground)] dark:border-zinc-600 dark:bg-zinc-800">
      <Icon size={12} className="shrink-0 text-[var(--surface-foreground-muted)]" />
      <span className="truncate">{draft.label}</span>
      <button
        type="button"
        aria-label="Remove comment draft"
        title="Remove comment draft"
        className="absolute right-1 top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-full bg-black/10 text-[var(--surface-foreground-muted)] opacity-0 transition-opacity hover:bg-black/20 focus-visible:opacity-100 group-hover:opacity-100 dark:bg-white/10 dark:hover:bg-white/20"
        onClick={() => rightDetailsPanelApi.cancelCommentDraft()}
      >
        <X size={10} strokeWidth={2.5} />
      </button>
    </div>
  )
}
