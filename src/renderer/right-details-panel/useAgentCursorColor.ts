import { useEffect, useState } from 'react'
import { rightDetailsPanelApi } from './rightDetailsPanelApi'

/** Colour of the agent cursor on the canvas, or null while none shows. */
export function useAgentCursorColor(): string | null {
  const [color, setColor] = useState<string | null>(null)
  useEffect(() => rightDetailsPanelApi.onAgentCursorColor(setColor), [])
  return color
}
