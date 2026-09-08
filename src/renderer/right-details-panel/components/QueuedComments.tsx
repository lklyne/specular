import { MessageSquare } from 'lucide-react'
import type { AgentThreadMessage } from '../../../shared/agent-thread'
import { usePaneTheme } from '../PaneContext'

/**
 * Messages waiting to be handed to the agent — comments taken from the canvas,
 * and follow-ups typed while a run is still in flight — stacked above the
 * message field. They move into the transcript the moment their turn starts.
 */
export function QueuedComments({ messages }: { messages: AgentThreadMessage[] }) {
  const isDark = usePaneTheme()
  if (messages.length === 0) return null
  return (
    <div className="flex flex-col gap-1 pb-1">
      {messages.map((message) => (
        <div
          key={message.id}
          className={`flex items-start gap-1.5 rounded-lg px-1.5 py-1 text-[12px] leading-5 ${
            isDark ? 'bg-zinc-800' : 'bg-zinc-200/60'
          }`}
        >
          <MessageSquare
            size={11}
            className="mt-[5px] shrink-0 text-[var(--surface-foreground-muted)]"
          />
          <span className="line-clamp-3 min-w-0 flex-1 whitespace-pre-wrap">{message.text}</span>
        </div>
      ))}
    </div>
  )
}
