import { useMemo } from 'react'
import { Popover } from '@base-ui/react/popover'
import { Info, Loader2, MessageCircle, RotateCw } from 'lucide-react'
import type {
  Annotation,
  DevtoolsPanelPageSummary,
  FixProgressEntry,
} from '../../../shared/types'
import { PRIMARY_BUTTON_BORDERED_CLASS } from '../../shared/primaryButton'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import {
  authorLabel,
  formatCommentTime,
} from '../rightDetailsPanelHelpers'
import {
  groupAnnotationsByPage,
} from '../rightDetailsPanelSelectors'
import { useFocusedAnnotationScroll } from '../useFocusedAnnotationScroll'
import {
  CircleCheckIcon,
  MoreVerticalIcon,
  TrashIcon,
} from '../../shared/PanelIcons'
import { FixEventList, fixStatusLabel } from '../../shared/FixEventList'

export function CommentRow({
  annotation,
  isDark,
  mutedClass,
  rowHoverClass,
  focusRowClass,
  focusedAnnotationId,
  registerAnnotationElement,
  progress,
}: {
  annotation: Annotation
  isDark: boolean
  mutedClass: string
  rowHoverClass: string
  focusRowClass: string
  focusedAnnotationId?: string | null
  registerAnnotationElement: (id: string, element: HTMLElement | null) => void
  progress?: FixProgressEntry
}) {
  const hasScreenshot = !!annotation.metadata?.regionScreenshot
  const progressButton = progress ? (
    <FixProgressButton progress={progress} isDark={isDark} mutedClass={mutedClass} />
  ) : null
  const moreMenu = (
    <Popover.Root>
      <Popover.Trigger
        render={
          <button
            type="button"
            className={`inline-flex size-6 shrink-0 items-center justify-center rounded-md ${
              isDark
                ? 'text-[var(--surface-foreground-muted)] hover:bg-zinc-700'
                : 'text-[var(--surface-foreground-muted)] hover:bg-zinc-200'
            }`}
            aria-label="More actions"
            title="More actions"
          />
        }
      >
        <MoreVerticalIcon className="size-4" />
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner sideOffset={4} align="end">
          <Popover.Popup
            className={`z-30 min-w-[140px] overflow-hidden rounded-md border py-1 shadow-xl ${
              isDark
                ? 'border-zinc-600 bg-zinc-800 text-[var(--surface-foreground)]'
                : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
            }`}
          >
            <button
              type="button"
              className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs ${
                isDark ? 'text-[var(--surface-foreground-muted)] hover:bg-white/10 hover:text-[var(--surface-foreground)]' : 'text-[var(--surface-foreground-muted)] hover:bg-zinc-100 hover:text-[var(--surface-foreground)]'
              }`}
              onClick={() => {
                rightDetailsPanelApi.resolveAnnotation(annotation.id)
              }}
            >
              <CircleCheckIcon className="size-4" />
              <span>Close</span>
            </button>
            <button
              type="button"
              className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs ${
                isDark ? 'text-[var(--surface-foreground-muted)] hover:bg-white/10 hover:text-[var(--surface-foreground)]' : 'text-[var(--surface-foreground-muted)] hover:bg-zinc-100 hover:text-[var(--surface-foreground)]'
              }`}
              onClick={() => {
                rightDetailsPanelApi.deleteAnnotation(annotation.id)
              }}
            >
              <TrashIcon className="size-4" />
              <span>Delete</span>
            </button>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )

  return (
    <div
      ref={(element) => registerAnnotationElement(annotation.id, element)}
      className={`group/card relative flex flex-col gap-0.5 overflow-hidden rounded-xl border p-1.5 transition-colors ${
        focusedAnnotationId === annotation.id ? focusRowClass : rowHoverClass
      } ${isDark ? 'border-zinc-700/50 bg-zinc-800/60' : 'border-zinc-200 bg-white'}`}
    >
      {hasScreenshot ? (
        <button
          type="button"
          className="relative w-full cursor-pointer"
          onClick={() => rightDetailsPanelApi.openAnnotationThread(annotation.id)}
        >
          <img
            src={`data:image/png;base64,${annotation.metadata!.regionScreenshot}`}
            alt="Region screenshot"
            className="w-full rounded-lg border border-zinc-200/50 bg-zinc-100 dark:border-zinc-700/50 dark:bg-zinc-900"
          />
        </button>
      ) : null}
      <div className="absolute top-2 right-2 opacity-0 transition-opacity group-hover/card:opacity-70 group-hover/card:hover:opacity-100">
        {moreMenu}
      </div>
      <button
        type="button"
        className="w-full cursor-pointer px-1 text-left"
        onClick={() => rightDetailsPanelApi.openAnnotationThread(annotation.id)}
      >
        <div className="inline-flex items-center gap-1.5 text-xs">
          <span className="font-medium">{authorLabel(annotation.author)}</span>
          <span className={mutedClass}>{formatCommentTime(annotation.createdAt)}</span>
        </div>
        {annotation.anchor.type === 'element' && annotation.elementName ? (
          <div className="mt-0.5 truncate text-xs font-medium text-[var(--surface-foreground)]">
            {annotation.elementName}
          </div>
        ) : null}
        <div className="mt-0.5 line-clamp-3 whitespace-pre-wrap text-xs">
          {annotation.text || '(No text)'}
        </div>
        {annotation.replies.length > 0 ? (
          <div className="mt-1 text-xs text-blue-600 dark:text-blue-400">
            {annotation.replies.length} repl{annotation.replies.length === 1 ? 'y' : 'ies'}
          </div>
        ) : null}
      </button>
      {progressButton ? (
        <div className="px-1 pb-0.5">{progressButton}</div>
      ) : null}
    </div>
  )
}

function FixProgressButton({
  progress,
  isDark,
  mutedClass,
}: {
  progress: FixProgressEntry
  isDark: boolean
  mutedClass: string
}) {
  const lastEvent = progress.events[progress.events.length - 1]
  const label = progress.status === 'running'
    ? (lastEvent ? lastEvent.text : 'Starting…')
    : progress.status === 'failed'
      ? (progress.error ?? 'Fix failed')
      : 'Fix log'
  const Icon = progress.status === 'running' ? Loader2 : Info
  const tone = progress.status === 'failed'
    ? (isDark ? 'text-red-300 hover:bg-zinc-700/70' : 'text-red-700 hover:bg-red-50')
    : progress.status === 'running'
      ? (isDark ? 'text-blue-300 hover:bg-zinc-700/70' : 'text-blue-700 hover:bg-blue-50')
      : (isDark ? 'text-[var(--surface-foreground-muted)] hover:bg-zinc-700/70' : 'text-[var(--surface-foreground-muted)] hover:bg-zinc-100')

  return (
    <Popover.Root>
      <Popover.Trigger
        render={
          <button
            type="button"
            className={`-ml-1.5 inline-flex max-w-full items-center gap-1 rounded-md px-1.5 py-0.5 text-[11px] ${tone}`}
            aria-label="Show fix log"
            title="Show fix log"
          />
        }
      >
        <Icon
          size={11}
          className={progress.status === 'running' ? 'animate-spin shrink-0' : 'shrink-0'}
        />
        <span className="truncate font-mono">{label}</span>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Positioner sideOffset={4} align="start">
          <Popover.Popup
            className={`z-30 flex max-h-[320px] w-[340px] flex-col overflow-hidden rounded-md border shadow-xl ${
              isDark ? 'border-zinc-600 bg-zinc-900 text-[var(--surface-foreground)]' : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
            }`}
          >
            <div className={`flex items-center justify-between border-b px-2 py-1.5 text-[11px] ${
              isDark ? 'border-zinc-700 bg-zinc-800/50' : 'border-zinc-200 bg-zinc-50'
            }`}>
              <span className="font-medium">{fixStatusLabel(progress.status)}</span>
              <span className={mutedClass}>{progress.events.length} events</span>
            </div>
            {progress.events.length === 0 ? (
              <div className={`px-2 py-3 text-[11px] ${mutedClass}`}>Waiting for output…</div>
            ) : (
              <FixEventList events={progress.events} className="flex-1 px-2 py-1.5" />
            )}
            {progress.error ? (
              <div className={`border-t px-2 py-1.5 text-[11px] ${
                isDark ? 'border-zinc-700 bg-red-900/20 text-red-300' : 'border-zinc-200 bg-red-50 text-red-700'
              }`}>
                {progress.error}
              </div>
            ) : null}
            {progress.status !== 'running' ? (
              <div
                className={`flex items-center justify-end border-t px-2 py-1.5 ${
                  isDark ? 'border-zinc-700 bg-zinc-800/50' : 'border-zinc-200 bg-zinc-50'
                }`}
              >
                <button
                  type="button"
                  onClick={() => rightDetailsPanelApi.fixSingleAnnotation(progress.annotationId)}
                  className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-[11px] font-medium ${
                    progress.status === 'failed'
                      ? PRIMARY_BUTTON_BORDERED_CLASS
                      : isDark
                        ? 'border-zinc-600 bg-zinc-800 text-[var(--surface-foreground)] hover:bg-zinc-700'
                        : 'border-zinc-300 bg-white text-[var(--surface-foreground-muted)] hover:bg-zinc-100'
                  }`}
                >
                  <RotateCw size={11} />
                  <span>{progress.status === 'failed' ? 'Retry' : 'Run again'}</span>
                </button>
              </div>
            ) : null}
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}

export function CommentsPane({
  isDark,
  annotations,
  pages,
  focusedAnnotationId,
  annotateEnabled,
  annotateAvailable,
  fixProgress,
}: {
  isDark: boolean
  annotations: Annotation[]
  pages: DevtoolsPanelPageSummary[]
  focusedAnnotationId?: string | null
  annotateEnabled: boolean
  annotateAvailable: boolean
  fixProgress: Record<string, FixProgressEntry>
}) {
  const mutedClass = 'text-[var(--surface-foreground-muted)]'
  const dividerClass = isDark ? 'border-zinc-700/50' : 'border-zinc-200/80'
  const rowHoverClass = isDark ? 'hover:bg-zinc-700/55' : 'hover:border-zinc-300'
  const focusRowClass = isDark ? 'bg-blue-500/20' : 'bg-blue-500/10'
  const groups = useMemo(() => groupAnnotationsByPage(annotations, pages), [annotations, pages])
  const { registerAnnotationElement } = useFocusedAnnotationScroll(
    focusedAnnotationId,
    annotations,
  )

  return (
    <section className="px-2 pt-1 pb-3">
      {!groups.some((g) => g.unresolved.length > 0) ? (
        <div
          className={`flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed px-4 py-6 ${
            isDark ? 'border-zinc-600' : 'border-zinc-300'
          }`}
        >
          <span className={`text-xs ${mutedClass}`}>No open comments</span>
          <button
            type="button"
            className={`inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-medium ${
              annotateEnabled
                ? PRIMARY_BUTTON_BORDERED_CLASS
                : isDark
                  ? 'border-zinc-600 bg-zinc-800 text-[var(--surface-foreground)] hover:bg-zinc-700'
                  : 'border-zinc-300 bg-zinc-50 text-[var(--surface-foreground-muted)] hover:bg-zinc-200'
            } disabled:pointer-events-none disabled:opacity-45`}
            onClick={() => rightDetailsPanelApi.setTool({ kind: annotateEnabled ? 'select' : 'comment' })}
            disabled={!annotateAvailable}
          >
            <MessageCircle size={14} />
            <span>Add comments</span>
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {groups.filter((g) => g.unresolved.length > 0).map((group, index) => (
              <section
                key={group.pageKey}
                className={index === 0 ? '' : `border-t pt-2 ${dividerClass}`}
              >
                <div className="space-y-2">
                  {group.unresolved.map((annotation) => (
                    <CommentRow
                      key={annotation.id}
                      annotation={annotation}
                      isDark={isDark}
                      mutedClass={mutedClass}
                      rowHoverClass={rowHoverClass}
                      focusRowClass={focusRowClass}
                      focusedAnnotationId={focusedAnnotationId}
                      registerAnnotationElement={registerAnnotationElement}
                      progress={fixProgress[annotation.id]}
                    />
                  ))}
                </div>
              </section>
          ))}

        </div>
      )}
    </section>
  )
}

