import { useDeferredValue, useEffect, useRef, ViewTransition } from 'react'
import { Collapsible } from '@base-ui/react/collapsible'
import { ChevronDown } from 'lucide-react'
import type { FixProgressEvent } from '../../../shared/types'
import { GrainGradient } from '../../shared/GrainGradient'
import { Shimmer } from '../../shared/Shimmer'
import './AgentRunBar.css'

/**
 * A running agent turn: a gradient strip whose one line says what the agent is
 * doing now. Opens to the full run log.
 */
export function AgentRunBar({ events }: { events: FixProgressEvent[] }) {
  // Deferred so the label change renders as a transition, which is what
  // triggers the ViewTransition below.
  const current = useDeferredValue(currentLabel(events))

  return (
    <Collapsible.Root className="overflow-hidden rounded-lg border border-[var(--surface-input-border)] bg-[var(--surface-input)]">
      <Collapsible.Trigger className="group relative flex h-9 w-full items-center gap-2 px-3 text-left">
        <GrainGradient />
        <span aria-hidden className="absolute inset-0 bg-white/45 dark:bg-black/45" />
        <ViewTransition key={current} enter="run-label-enter" exit="run-label-exit" default="none">
          <Shimmer
            aria-live="polite"
            className="relative min-w-0 flex-1 text-[12px] font-medium [--shimmer-base:rgb(0_0_0/0.55)] [--shimmer-highlight:rgb(0_0_0)] dark:[--shimmer-base:rgb(255_255_255/0.6)] dark:[--shimmer-highlight:rgb(255_255_255)]"
          >
            {current}
          </Shimmer>
        </ViewTransition>
        <ChevronDown
          size={12}
          className="relative shrink-0 text-black/60 transition-transform group-data-[panel-open]:rotate-180 dark:text-white/70"
        />
      </Collapsible.Trigger>
      <Collapsible.Panel>
        <RunLog events={events} />
      </Collapsible.Panel>
    </Collapsible.Root>
  )
}

function RunLog({ events }: { events: FixProgressEvent[] }) {
  const logRef = useRef<HTMLOListElement | null>(null)
  useEffect(() => {
    const el = logRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [events.length])

  return (
    <ol
      ref={logRef}
      className="thin-scrollbar max-h-40 space-y-1 overflow-y-auto px-3 py-2 font-mono text-[11px] leading-relaxed break-words text-[var(--surface-foreground-muted)]"
    >
      {events.map((event, i) => (
        <li key={`${event.timestamp}-${i}`}>{event.text}</li>
      ))}
    </ol>
  )
}

/** Tool results carry no label, so this is the step still in flight. */
function currentLabel(events: FixProgressEvent[]): string {
  for (let i = events.length - 1; i >= 0; i--) {
    const label = events[i].label
    if (label) return label
  }
  return 'Starting'
}
