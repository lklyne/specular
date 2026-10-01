import type { CSSProperties } from 'react'

// Sits below-right of the 24px cursor icon so the arrow tip stays unobscured.
const LABEL_OFFSET = { x: 18, y: 20 }

/**
 * The agent's own words for what it is doing (its task label). Agent-written
 * and therefore untrusted: rendered as a text node only, clipped to one line.
 * The chip is neutral rather than cursor-colored because cursor hues span the
 * full wheel at fixed lightness, where neither white nor black text stays
 * legible on all of them; the accent bar carries the color instead.
 */
export function AgentCursorLabel({ text, color }: { text: string; color: string }) {
  const style: CSSProperties = {
    left: LABEL_OFFSET.x,
    top: LABEL_OFFSET.y,
    maxWidth: 240,
    borderLeft: `3px solid ${color}`,
  }
  return (
    <div
      className="absolute truncate whitespace-nowrap rounded-sm bg-neutral-900/90 px-1.5 py-0.5 text-[11px] leading-4 text-white shadow-sm"
      style={style}
    >
      {text}
    </div>
  )
}
