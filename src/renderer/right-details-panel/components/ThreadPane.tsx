import { useEffect, useRef, useState } from 'react'
import { ChevronDown, ChevronLeft, ChevronRight, Info, Loader2, Play } from 'lucide-react'
import type { Annotation, FixProgressEntry } from '../../../shared/types'
import { CommentBubble, CommentInput } from '../../shared/CommentPrimitives'
import { FixEventList, fixStatusLabel } from '../../shared/FixEventList'
import { CircleCheckIcon, MoreVerticalIcon, TrashIcon } from '../../shared/PanelIcons'
import { usePaneTheme } from '../PaneContext'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { formatCommentTime } from '../rightDetailsPanelHelpers'

/**
 * Drill-in view for one comment thread: the whole conversation, full panel
 * height, with the composer pinned at the bottom. The canvas keeps only a
 * highlight ring on the thread's anchor, so the page under discussion stays
 * visible while the agent works on it.
 */
export function ThreadPane({
  annotation,
  progress,
}: {
  annotation: Annotation
  progress?: FixProgressEntry
}) {
  const isDark = usePaneTheme()
  const [replyText, setReplyText] = useState('')
  const [menuOpen, setMenuOpen] = useState(false)
  const transcriptRef = useRef<HTMLDivElement | null>(null)

  const muted = 'text-[var(--surface-foreground-muted)]'
  const divider = isDark ? 'border-zinc-700' : 'border-zinc-200'
  const iconBtn = `flex h-6 w-6 items-center justify-center rounded transition-colors ${
    isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-100'
  }`
  const running = progress?.status === 'running'

  const messageCount = annotation.replies.length
  useEffect(() => {
    const el = transcriptRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [messageCount, running])

  const submitReply = () => {
    const text = replyText.trim()
    if (!text) return
    rightDetailsPanelApi.replyToAnnotation(annotation.id, text)
    setReplyText('')
  }

  const anchorLabel =
    annotation.anchor.type === 'element'
      ? annotation.elementName || 'Element'
      : annotation.anchor.type === 'region'
        ? 'Region'
        : annotation.anchor.type === 'page'
          ? 'Page'
          : 'Canvas note'

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className={`flex items-center gap-1 border-b px-2 py-1.5 ${divider}`}>
        <button
          type="button"
          className={`flex items-center gap-0.5 rounded px-1 py-0.5 text-[12px] font-medium transition-colors ${
            isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-100'
          }`}
          onClick={() => rightDetailsPanelApi.closeAnnotationThread()}
        >
          <ChevronLeft size={13} />
          Comments
        </button>
        <div className="ml-auto flex items-center gap-0.5">
          <button
            type="button"
            className={`${iconBtn} ${muted} disabled:opacity-40`}
            aria-label="Fix with agent"
            title="Fix with agent"
            disabled={running}
            onClick={() => rightDetailsPanelApi.fixSingleAnnotation(annotation.id)}
          >
            {running ? <Loader2 size={13} className="animate-spin" /> : <Play size={13} />}
          </button>
          <div className="relative">
            <button
              type="button"
              className={`${iconBtn} ${muted}`}
              aria-label="More actions"
              title="More actions"
              onClick={() => setMenuOpen((current) => !current)}
            >
              <MoreVerticalIcon className="size-3.5" />
            </button>
            {menuOpen ? (
              <div
                className={`absolute right-0 top-7 z-30 min-w-32 rounded-md border py-1 shadow-xl ${
                  isDark
                    ? 'border-zinc-600 bg-zinc-800 text-[var(--surface-foreground)]'
                    : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
                }`}
              >
                <button
                  type="button"
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs ${
                    isDark ? 'hover:bg-white/10' : 'hover:bg-zinc-100'
                  }`}
                  onClick={() => {
                    setMenuOpen(false)
                    rightDetailsPanelApi.resolveAnnotation(annotation.id)
                    rightDetailsPanelApi.closeAnnotationThread()
                  }}
                >
                  <CircleCheckIcon className="size-3.5" />
                  <span>Close</span>
                </button>
                <button
                  type="button"
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs ${
                    isDark ? 'hover:bg-white/10' : 'hover:bg-zinc-100'
                  }`}
                  onClick={() => {
                    setMenuOpen(false)
                    rightDetailsPanelApi.deleteAnnotation(annotation.id)
                    rightDetailsPanelApi.closeAnnotationThread()
                  }}
                >
                  <TrashIcon className="size-3.5" />
                  <span>Delete</span>
                </button>
              </div>
            ) : null}
          </div>
        </div>
      </div>

      <div className={`flex items-baseline gap-1.5 border-b px-3 py-1.5 text-xs ${divider}`}>
        <span className="truncate font-medium">{anchorLabel}</span>
        <span className={`shrink-0 ${muted}`}>{formatCommentTime(annotation.createdAt)}</span>
      </div>

      <div ref={transcriptRef} className="min-h-0 flex-1 space-y-2.5 overflow-y-auto px-3 py-2.5">
        <CommentBubble
          author={annotation.author}
          text={annotation.text}
          fallback="Drawing feedback"
        />
        {annotation.replies.map((reply, idx) => (
          <CommentBubble
            key={`${annotation.id}:reply:${idx}`}
            author={reply.author}
            text={reply.text}
          />
        ))}
        {progress ? <FixRunRow progress={progress} isDark={isDark} muted={muted} /> : null}
      </div>

      <div className={`border-t px-2 py-2 ${divider}`}>
        <div
          className={`relative rounded-[16px] border py-1.5 pl-2.5 pr-1.5 ${
            isDark ? 'border-zinc-600 bg-zinc-900/40' : 'border-zinc-300 bg-zinc-50'
          }`}
        >
          <CommentInput
            value={replyText}
            onChange={setReplyText}
            onSubmit={submitReply}
            placeholder="Reply…"
            submitLabel="Send reply"
          />
        </div>
      </div>
    </div>
  )
}

/**
 * The agent's latest run, collapsed to one row in the transcript. Click to
 * expand the event log; it never auto-expands — the row's spinner and latest
 * event line say enough while a run is live.
 */
function FixRunRow({
  progress,
  isDark,
  muted,
}: {
  progress: FixProgressEntry
  isDark: boolean
  muted: string
}) {
  const [expanded, setExpanded] = useState(false)
  const running = progress.status === 'running'
  const lastEvent = progress.events[progress.events.length - 1]
  const eventCount = progress.events.length

  const tone =
    progress.status === 'failed'
      ? isDark ? 'text-red-300' : 'text-red-700'
      : running
        ? isDark ? 'text-blue-300' : 'text-blue-700'
        : muted

  return (
    <div
      className={`rounded-lg border ${
        isDark ? 'border-zinc-700/60 bg-zinc-800/40' : 'border-zinc-200 bg-zinc-50/60'
      }`}
    >
      <button
        type="button"
        className="flex w-full items-center gap-1.5 px-2 py-1.5 text-left text-[11px]"
        onClick={() => setExpanded((current) => !current)}
      >
        {running ? (
          <Loader2 size={11} className="shrink-0 animate-spin" />
        ) : (
          <Info size={11} className="shrink-0" />
        )}
        <span className={`shrink-0 font-medium ${tone}`}>{fixStatusLabel(progress.status)}</span>
        <span className={`min-w-0 flex-1 truncate font-mono ${muted}`}>
          {running && lastEvent ? lastEvent.text : `${eventCount} event${eventCount === 1 ? '' : 's'}`}
        </span>
        {expanded ? (
          <ChevronDown size={11} className={`shrink-0 ${muted}`} />
        ) : (
          <ChevronRight size={11} className={`shrink-0 ${muted}`} />
        )}
      </button>
      {expanded ? (
        <div className={`border-t ${isDark ? 'border-zinc-700/60' : 'border-zinc-200'}`}>
          {eventCount === 0 ? (
            <div className={`px-2 py-2 text-[11px] ${muted}`}>Waiting for output…</div>
          ) : (
            <FixEventList events={progress.events} className="max-h-[240px] px-2 py-1.5" />
          )}
          {progress.error ? (
            <div
              className={`border-t px-2 py-1.5 text-[11px] ${
                isDark ? 'border-zinc-700/60 text-red-300' : 'border-zinc-200 text-red-700'
              }`}
            >
              {progress.error}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
