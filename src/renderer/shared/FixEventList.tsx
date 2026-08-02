import { Fragment, useEffect, useRef } from 'react'
import type { FixProgressEntry, FixProgressEvent } from '../../shared/types'

export function fixStatusLabel(status: FixProgressEntry['status']): string {
  if (status === 'running') return 'Running'
  if (status === 'failed') return 'Failed'
  return 'Completed'
}

const kindColor: Record<FixProgressEvent['kind'], string> = {
  system: 'text-[var(--surface-panel-foreground-muted)]',
  text: 'text-[var(--surface-panel-foreground)]',
  tool_use: 'text-blue-700 dark:text-blue-300',
  tool_result: 'text-emerald-700 dark:text-emerald-300',
  result: 'text-[var(--surface-panel-foreground)]',
  stderr: 'text-amber-700 dark:text-amber-300',
  error: 'text-red-700 dark:text-red-300',
}

export function FixEventList({
  events,
  className,
}: {
  events: FixProgressEvent[]
  className?: string
}) {
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const eventCount = events.length

  useEffect(() => {
    const node = scrollRef.current
    if (!node) return
    node.scrollTop = node.scrollHeight
  }, [eventCount])

  return (
    <div
      ref={scrollRef}
      className={`grid auto-rows-min grid-cols-[auto_minmax(0,1fr)] gap-x-2 gap-y-0 overflow-auto font-mono text-[11px] leading-relaxed ${className ?? ''}`}
    >
      {events.map((event, i) => (
        <Fragment key={`${event.timestamp}-${i}`}>
          <span className="text-[var(--surface-panel-foreground-muted)]">
            {event.kind.replace('_', ' ')}
          </span>
          <span className={`break-words ${kindColor[event.kind]}`}>
            {event.text}
          </span>
        </Fragment>
      ))}
    </div>
  )
}
