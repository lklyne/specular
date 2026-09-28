import type { Annotation } from '../../../shared/types'
import { usePaneTheme } from '../PaneContext'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'

/**
 * A count of the thread's comments that were sent to the agent but are still
 * open on the canvas, with one action to close them all once the agent's work
 * has addressed them. The comments themselves live on the canvas and in the
 * transcript, so they aren't repeated here.
 */
export function OpenComments({ annotations }: { annotations: Annotation[] }) {
  const isDark = usePaneTheme()
  if (annotations.length === 0) return null
  const resolveAll = () => {
    for (const annotation of annotations) rightDetailsPanelApi.resolveAnnotation(annotation.id)
  }
  return (
    <div className="flex items-center justify-between gap-2 px-0.5 pb-1">
      <span className="text-[11px] font-medium text-[var(--surface-foreground-muted)]">
        Thread comments <span className="tabular-nums">{annotations.length}</span>
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
  )
}
