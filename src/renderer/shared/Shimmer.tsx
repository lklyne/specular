import type { CSSProperties } from 'react'
import { mergeProps } from '@base-ui/react/merge-props'
import { useRender } from '@base-ui/react/use-render'
import './Shimmer.css'

/**
 * Single-line text with a highlight band sweeping across it. The band's width
 * scales with the text so short and long labels sweep at the same pace.
 * Colours come from `--shimmer-base` / `--shimmer-highlight`.
 */
export function Shimmer({
  children,
  duration = 2,
  spread = 2,
  render,
  ...props
}: useRender.ComponentProps<'span'> & {
  children: string
  /** Seconds per sweep. */
  duration?: number
  /** Band half-width in px per character. */
  spread?: number
}) {
  const style = {
    '--shimmer-spread': `${Math.max(children.length * spread, 24)}px`,
    '--shimmer-duration': `${duration}s`,
  } as CSSProperties
  return useRender({
    defaultTagName: 'span',
    render,
    props: mergeProps<'span'>(
      { className: 'shimmer-text block truncate', style, children },
      props,
    ),
  })
}
