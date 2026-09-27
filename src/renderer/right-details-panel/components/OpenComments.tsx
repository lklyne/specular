import { MessageSquare } from 'lucide-react'
import type { Annotation } from '../../../shared/types'
import { usePaneTheme } from '../PaneContext'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'

/**
 * The thread's comments that were sent to the agent but are still open on the
 * canvas, stacked above the message field with one action to close them all
 * once the agent's work has addressed them.
 */
export function OpenComments({ annotations }: { annotations: Annotation[] }) {
  const isDark = usePaneTheme()
  if (annotations.length === 0) return null
  const resolveAll = () => {
    for (const annotation of annotations) rightDetailsPanelApi.resolveAnnotation(annotation.id)
  }
  return (
    <div className="flex flex-col gap-1 pb-1">
      <div className="flex items-center justify-between gap-2 px-0.5">
        <span className="text-[11px] text-[var(--surface-foreground-muted)]">
          {annotations.length} open comment{annotations.length === 1 ? '' : 's'}
        </span>
        <button
          type="button"
          className={`rounded-md px-1.5 py-0.5 text-[11px] font-medium text-[var(--surface-foreground)] transition-colors ${
            isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-200'
          }`}
          onClick={() => resolveAll()}
        >
          Resolve comments
        </button>
      </div>
      {annotations.map((annotation) => (
        <div
          key={annotation.id}
          data-annotation-id={annotation.id}
          className={`flex items-start gap-1.5 rounded-lg px-1.5 py-1 text-[12px] leading-5 ${
            isDark ? 'bg-zinc-800' : 'bg-zinc-200/60'
          }`}
        >
          <MessageSquare size={11} className="mt-[5px] shrink-0 text-[var(--surface-foreground-muted)]" />
          <span className="line-clamp-2 min-w-0 flex-1 whitespace-pre-wrap">{annotation.text}</span>
        </div>
      ))}
    </div>
  )
}
