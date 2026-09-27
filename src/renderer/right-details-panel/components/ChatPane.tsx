import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { FolderOpen, Plus, X, Zap } from 'lucide-react'
import { messageHasContent, type AgentThread, type AgentThreadMessage } from '../../../shared/agent-thread'
import type { CommentDraft } from '../../../shared/comment-draft'
import type { Annotation, DevtoolsPanelData, FixProgressEntry } from '../../../shared/types'
import { isUnresolved } from '../../../shared/annotation-utils'
import { CommentBubble, CommentSendButton, CommentTextarea } from '../../shared/CommentPrimitives'
import { Tooltip } from '../../shared/Tooltip'
import { usePaneTheme } from '../PaneContext'
import { AgentRunBar } from './AgentRunBar'
import { CommentDraftChip } from './CommentDraftChip'
import { ContextChip, composerChipClass } from './ContextChip'
import { OpenComments } from './OpenComments'
import { PastedImages } from './PastedImages'
import { QueuedComments } from './QueuedComments'
import { ModelChip } from './ModelChip'
import { PaneHeader } from './PaneHeader'
import { threadPillFromPanelData, threadWriteTargetFromPanel } from '../panelThreadPill'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { useCommentFlash } from '../useCommentFlash'
import { usePastedImages } from '../usePastedImages'

export function ChatPane({ data }: { data: DevtoolsPanelData }) {
  const isDark = usePaneTheme()
  const threads = data.agentThreads ?? []
  const activeId = data.activeThreadId ?? null
  const active = threads.find((thread) => thread.id === activeId) ?? null
  const pill = useMemo(() => threadPillFromPanelData(data), [data])
  const writeTarget = useMemo(() => threadWriteTargetFromPanel(data, pill), [data, pill])
  const progress = active ? data.fixProgress?.[active.id] : undefined
  const running = progress?.status === 'running'
  const divider = isDark ? 'border-zinc-700' : 'border-zinc-200'
  const muted = 'text-[var(--surface-foreground-muted)]'
  const queued = active?.messages.filter((message) => message.queued && messageHasContent(message)) ?? []
  const queuedIds = new Set(queued.map((message) => message.annotationId))
  const openComments = (data.annotations ?? []).filter(
    (annotation) =>
      active?.annotationIds.includes(annotation.id) &&
      isUnresolved(annotation.status) &&
      !queuedIds.has(annotation.id),
  )
  const isNew =
    !active || active.status === 'draft' || !active.messages.some((message) => message.role === 'agent')
  const rootRef = useRef<HTMLDivElement | null>(null)
  useCommentFlash(rootRef, data)

  return (
    <div ref={rootRef} className="flex h-full min-h-0 flex-col">
      <PaneHeader
        label={active?.title ?? 'Threads'}
        actions={<ThreadActions hasActive={Boolean(active)} isDark={isDark} />}
      />
      {active ? (
        <ThreadTranscript
          thread={active}
          progress={progress}
          spacePath={data.spacePath ?? null}
          muted={muted}
        />
      ) : (
        <ThreadList threads={threads} isDark={isDark} muted={muted} />
      )}
      <div className={`border-t px-2 py-2 ${divider}`}>
        <Composer
          running={running}
          isNew={isNew}
          queued={queued}
          openComments={openComments}
          commentDraft={data.commentDraft ?? null}
          context={<ContextChip pill={pill} data={data} />}
          model={data.fixConfig ? <ModelChip fixConfig={data.fixConfig} /> : null}
          folderPath={writeTarget.kind === 'repo' ? writeTarget.repoPath : (data.spacePath ?? null)}
          autoFix={
            writeTarget.kind === 'repo'
              ? { origin: writeTarget.origin, on: Boolean(data.originBindings?.[writeTarget.origin]?.autoFix) }
              : null
          }
          isDark={isDark}
          muted={muted}
        />
      </div>
    </div>
  )
}

function ThreadActions({ hasActive, isDark }: { hasActive: boolean; isDark: boolean }) {
  const iconBtn = `flex h-6 w-6 items-center justify-center rounded transition-colors ${
    isDark ? 'hover:bg-zinc-700' : 'hover:bg-zinc-100'
  }`
  // Switching threads leaves the composer mounted, so the buttons keep focus
  // in the field — an open comment draft stays ready to type into.
  const keepFocus = (event: React.PointerEvent) => event.preventDefault()
  return (
    <div className="flex items-center gap-0.5">
      <button
        type="button"
        className={iconBtn}
        title="New thread"
        aria-label="New thread"
        onPointerDown={keepFocus}
        onClick={() => rightDetailsPanelApi.newAgentThread()}
      >
        <Plus size={13} />
      </button>
      {hasActive ? (
        <button
          type="button"
          className={iconBtn}
          title="Back to threads"
          aria-label="Back to threads"
          onPointerDown={keepFocus}
          onClick={() => rightDetailsPanelApi.deselectAgentThread()}
        >
          <X size={13} />
        </button>
      ) : null}
    </div>
  )
}

function ThreadList({
  threads,
  isDark,
  muted,
}: {
  threads: AgentThread[]
  isDark: boolean
  muted: string
}) {
  const [menu, setMenu] = useState<{ threadId: string; x: number; y: number } | null>(null)
  return (
    <div className="min-h-0 flex-1 overflow-y-auto py-1">
      {threads.length === 0 ? (
        <div className={`px-3 py-2 text-[12px] ${muted}`}>
          Comment on the canvas to queue a draft, or type below and send.
        </div>
      ) : (
        threads.map((thread) => (
          <button
            key={thread.id}
            type="button"
            className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-[12px] ${
              isDark ? 'hover:bg-white/10' : 'hover:bg-zinc-100'
            }`}
            onClick={() => rightDetailsPanelApi.selectAgentThread(thread.id)}
            onContextMenu={(event) => {
              event.preventDefault()
              setMenu({ threadId: thread.id, x: event.clientX, y: event.clientY })
            }}
          >
            <span className="min-w-0 flex-1 truncate">{thread.title}</span>
            {thread.status === 'draft' ? (
              <span className={`text-[10px] uppercase tracking-wide ${muted}`}>draft</span>
            ) : null}
            <span className={`shrink-0 text-[10px] ${muted}`}>{shortDate(thread.updatedAt)}</span>
          </button>
        ))
      )}
      {menu ? (
        <>
          <div
            className="fixed inset-0 z-30"
            onClick={() => setMenu(null)}
            onContextMenu={(event) => {
              event.preventDefault()
              setMenu(null)
            }}
          />
          <div
            className={`fixed z-40 min-w-32 overflow-hidden rounded-md border py-1 shadow-xl ${
              isDark
                ? 'border-zinc-600 bg-zinc-800 text-[var(--surface-foreground)]'
                : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
            }`}
            style={{ left: menu.x, top: menu.y }}
          >
            <button
              type="button"
              className={`block w-full px-3 py-1.5 text-left text-[12px] text-red-600 dark:text-red-400 ${
                isDark ? 'hover:bg-white/10' : 'hover:bg-zinc-100'
              }`}
              onClick={() => {
                rightDetailsPanelApi.deleteAgentThread(menu.threadId)
                setMenu(null)
              }}
            >
              Delete thread
            </button>
          </div>
        </>
      ) : null}
    </div>
  )
}

function shortDate(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return ''
  const sameDay = date.toDateString() === new Date().toDateString()
  return sameDay
    ? date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
    : date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}

function ThreadTranscript({
  thread,
  progress,
  spacePath,
  muted,
}: {
  thread: AgentThread | null
  progress?: FixProgressEntry
  spacePath: string | null
  muted: string
}) {
  const transcriptRef = useRef<HTMLDivElement | null>(null)
  const messageCount = thread?.messages.length ?? 0
  useEffect(() => {
    const el = transcriptRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [messageCount, progress?.events.length, progress?.status])

  return (
    <div ref={transcriptRef} className="thin-scrollbar min-h-0 flex-1 space-y-4 overflow-y-auto px-3 py-2.5">
      {!thread || thread.messages.length === 0 ? (
        <div className={`text-[12px] ${muted}`}>
          Comment on the canvas to queue a draft, or type below and send.
        </div>
      ) : (
        thread.messages
          .filter((message) => !message.queued)
          .map((message) => (
            <CommentBubble
              key={message.id}
              author={message.role}
              text={message.text}
              annotationId={message.annotationId}
              imageSrcs={imageSrcs(message, spacePath)}
            />
          ))
      )}
      {progress?.status === 'running' ? (
        <AgentRunBar events={progress.events} />
      ) : null}
      {progress?.status === 'failed' && progress.error ? (
        <div className="text-[12px] text-red-600 dark:text-red-400">{progress.error}</div>
      ) : null}
    </div>
  )
}

function Composer({
  running,
  isNew,
  queued,
  openComments,
  commentDraft,
  context,
  model,
  folderPath,
  autoFix,
  isDark,
  muted,
}: {
  running: boolean
  isNew: boolean
  /** Comments this turn will carry, shown above the field until they're sent. */
  queued: AgentThreadMessage[]
  /** Sent comments still open on the canvas, resolvable in one click. */
  openComments: Annotation[]
  /** The in-progress comment, if any — shown as a removable chip; while open,
   *  the field saves the comment instead of sending the thread. */
  commentDraft: CommentDraft | null
  /** The thread's anchor chip — where this turn is aimed. */
  context: React.ReactNode
  /** Model picker, or null until the config arrives. */
  model: React.ReactNode
  /** Where this thread writes: the bound repo, or the space folder. */
  folderPath: string | null
  /** Auto-fix for the write target's origin; null when writing to the space. */
  autoFix: { origin: string; on: boolean } | null
  isDark: boolean
  muted: string
}) {
  const [text, setText] = useState('')
  const pasted = usePastedImages()
  const textareaRef = useRef<HTMLTextAreaElement | null>(null)
  // A run in flight doesn't close the composer: sending queues the follow-up,
  // which the thread picks up as soon as the run ends. A draft has no queue
  // of its own to fall back on — text or an image is required.
  const canSend = commentDraft
    ? Boolean(text.trim()) || pasted.images.length > 0
    : Boolean(text.trim()) || pasted.images.length > 0 || (!running && queued.length > 0)
  const submit = () => {
    if (!canSend) return
    const images = pasted.images.map(({ mediaType, data }) => ({ mediaType, data }))
    if (commentDraft) {
      rightDetailsPanelApi.submitCommentDraft(text.trim(), images)
    } else {
      rightDetailsPanelApi.sendAgentThread(text.trim(), images)
    }
    setText('')
    pasted.clear()
  }
  // Escape drops the draft with no comment saved — the typed text is left
  // alone, matching the field's ordinary blur behavior.
  const onTextareaKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== 'Escape' || !commentDraft) return
    event.preventDefault()
    rightDetailsPanelApi.cancelCommentDraft()
  }
  const draftId = commentDraft?.id ?? null
  useEffect(() => {
    if (draftId) textareaRef.current?.focus()
  }, [draftId])
  return (
    <div
      className={`rounded-[16px] border px-2 pb-1.5 pt-1.5 ${
        isDark ? 'border-zinc-600 bg-zinc-900/40' : 'border-zinc-300 bg-zinc-50'
      }`}
      onPaste={pasted.onPaste}
    >
      {commentDraft ? <CommentDraftChip draft={commentDraft} /> : null}
      <OpenComments annotations={openComments} />
      <QueuedComments messages={queued} />
      <PastedImages images={pasted.images} onRemove={pasted.remove} />
      <CommentTextarea
        inputRef={textareaRef}
        value={text}
        onChange={setText}
        onSubmit={submit}
        onKeyDown={onTextareaKeyDown}
        placeholder={
          commentDraft ? 'Add a comment…' : running ? 'Queue a follow-up…' : isNew ? 'Add or edit…' : 'Follow up…'
        }
        rows={2}
        className="min-h-[48px] max-h-[160px] px-0.5 py-0.5"
      />
      <div className="flex min-w-0 items-center gap-1 pt-0.5">
        <div className="flex min-w-0 flex-1 items-center gap-1">
          {context}
          {folderPath ? (
            <Tooltip side="top" label={`Changes are written to ${folderPath}`}>
              <span className={composerChipClass(isDark)}>
                <FolderOpen size={11} className="shrink-0" />
                <span className="truncate">{folderName(folderPath)}</span>
              </span>
            </Tooltip>
          ) : null}
          {autoFix ? <AutoFixChip {...autoFix} isDark={isDark} /> : null}
        </div>
        {model}
        <CommentSendButton
          onSubmit={submit}
          submitReady={canSend}
          label="Send"
          className="shrink-0"
        />
      </div>
    </div>
  )
}

/** Auto-fix toggle for the write target's origin: comments send themselves. */
function AutoFixChip({ origin, on, isDark }: { origin: string; on: boolean; isDark: boolean }) {
  const label = on
    ? `Auto-fix on for ${origin}: each comment is sent as soon as it is placed.`
    : `Auto-fix off for ${origin}: comments queue here until you send.`
  return (
    <Tooltip side="top" label={label}>
      <button
        type="button"
        aria-pressed={on}
        aria-label="Auto-fix"
        className={`${composerChipClass(isDark)} ${on ? 'text-emerald-600 dark:text-emerald-400' : ''}`}
        onClick={() => rightDetailsPanelApi.setAutoFix(origin, !on)}
      >
        <Zap size={11} className="shrink-0" />
        <span>Auto</span>
      </button>
    </Tooltip>
  )
}

/** Sent images live in the space folder, served to the panel over local-file://. */
function imageSrcs(message: AgentThreadMessage, spacePath: string | null): string[] | undefined {
  if (!spacePath || !message.images?.length) return undefined
  return message.images.map((image) => `local-file://${encodeURI(`${spacePath}/${image.path}`)}`)
}

function folderName(path: string): string {
  const parts = path.split('/').filter(Boolean)
  return parts[parts.length - 1] ?? path
}
