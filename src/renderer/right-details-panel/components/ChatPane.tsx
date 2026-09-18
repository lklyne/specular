import { useEffect, useMemo, useRef, useState } from 'react'
import { FolderOpen, Loader2, Plus, X } from 'lucide-react'
import type { AgentThread } from '../../../shared/agent-thread'
import type { DevtoolsPanelData, FixProgressEntry } from '../../../shared/types'
import { CommentBubble, CommentInput } from '../../shared/CommentPrimitives'
import { FixEventList } from '../../shared/FixEventList'
import { usePaneTheme } from '../PaneContext'
import { ContextChip } from './ContextChip'
import { ModelChip } from './ModelChip'
import { PaneHeader } from './PaneHeader'
import { threadPillFromPanelData, threadWriteTargetFromPanel } from '../panelThreadPill'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'

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
  const queued = active?.messages.filter((message) => message.queued && message.text.trim()) ?? []
  const hasQueued = Boolean(
    active?.messages.some((message) => message.role === 'user' && message.text.trim()),
  )
  const isNew =
    !active || active.status === 'draft' || !active.messages.some((message) => message.role === 'agent')

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PaneHeader
        label={active?.title ?? 'Threads'}
        actions={<ThreadActions hasActive={Boolean(active)} isDark={isDark} />}
      />
      {active ? (
        <ThreadTranscript thread={active} progress={progress} isDark={isDark} muted={muted} />
      ) : (
        <ThreadList threads={threads} isDark={isDark} muted={muted} />
      )}
      <div className={`border-t px-2 py-2 ${divider}`}>
        <Composer
          running={running}
          isDraft={active?.status === 'draft'}
          hasQueued={hasQueued}
          isNew={isNew}
          chip={
            <>
              <ContextChip pill={pill} data={data} queuedCount={queued.length} />
              {data.fixConfig ? <ModelChip fixConfig={data.fixConfig} /> : null}
            </>
          }
          repoPath={writeTarget.kind === 'repo' ? writeTarget.repoPath : null}
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
  return (
    <div className="flex items-center gap-0.5">
      <button
        type="button"
        className={iconBtn}
        title="New thread"
        aria-label="New thread"
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
  isDark,
  muted,
}: {
  thread: AgentThread | null
  progress?: FixProgressEntry
  isDark: boolean
  muted: string
}) {
  const transcriptRef = useRef<HTMLDivElement | null>(null)
  const messageCount = thread?.messages.length ?? 0
  useEffect(() => {
    const el = transcriptRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [messageCount, progress?.events.length, progress?.status])

  return (
    <div ref={transcriptRef} className="min-h-0 flex-1 space-y-2.5 overflow-y-auto px-3 py-2.5">
      {!thread || thread.messages.length === 0 ? (
        <div className={`text-[12px] ${muted}`}>
          Comment on the canvas to queue a draft, or type below and send.
        </div>
      ) : (
        thread.messages
          .filter((message) => !message.queued)
          .map((message) => (
            <CommentBubble key={message.id} author={message.role} text={message.text} />
          ))
      )}
      {progress?.status === 'running' ? (
        <div className={`rounded-md border px-2 py-1.5 ${isDark ? 'border-zinc-700' : 'border-zinc-200'}`}>
          <div className="mb-1 flex items-center gap-1.5 text-[11px]">
            <Loader2 size={11} className="animate-spin" />
            Running
          </div>
          <FixEventList events={progress.events} className="max-h-40" />
        </div>
      ) : null}
      {progress?.status === 'failed' && progress.error ? (
        <div className="text-[12px] text-red-600 dark:text-red-400">{progress.error}</div>
      ) : null}
    </div>
  )
}

function Composer({
  running,
  isDraft,
  hasQueued,
  isNew,
  chip,
  repoPath,
  isDark,
  muted,
}: {
  running: boolean
  isDraft: boolean
  hasQueued: boolean
  isNew: boolean
  chip: React.ReactNode
  repoPath: string | null
  isDark: boolean
  muted: string
}) {
  const [text, setText] = useState('')
  const canSend = !running && (Boolean(text.trim()) || (isDraft && hasQueued))
  const submit = () => {
    if (!canSend) return
    rightDetailsPanelApi.sendAgentThread(text.trim())
    setText('')
  }
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex min-w-0 items-center gap-1.5 px-0.5">
        {chip}
        {repoPath ? (
          <span className={`inline-flex min-w-0 items-center gap-1 ${muted}`} title={repoPath}>
            <FolderOpen size={11} className="shrink-0" />
            <span className="truncate">{folderName(repoPath)}</span>
          </span>
        ) : null}
      </div>
      <div
        className={`relative rounded-[16px] border py-1.5 pl-2.5 pr-1.5 ${
          isDark ? 'border-zinc-600 bg-zinc-900/40' : 'border-zinc-300 bg-zinc-50'
        }`}
      >
        <CommentInput
          value={text}
          onChange={setText}
          onSubmit={submit}
          placeholder={isNew ? 'Message…' : 'Follow up…'}
          submitLabel="Send"
          disabled={running}
          canSubmit={canSend}
        />
      </div>
    </div>
  )
}

function folderName(path: string): string {
  const parts = path.split('/').filter(Boolean)
  return parts[parts.length - 1] ?? path
}
