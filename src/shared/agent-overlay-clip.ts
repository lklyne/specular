export interface AgentOverlayClipInput {
  leftChromeWidth: number
}

export interface AgentOverlayClipInsets {
  top: number
  right: number
  bottom: number
  left: number
}

/**
 * The agent overlay BrowserWindow is natively positioned below top chrome and
 * sized to exclude the right details panel. Only left chrome remains inside
 * its local coordinate space and needs CSS clipping.
 */
export function agentOverlayClipInsets(
  input: AgentOverlayClipInput,
): AgentOverlayClipInsets {
  return {
    top: 0,
    right: 0,
    bottom: 0,
    left: input.leftChromeWidth,
  }
}

export function agentOverlayClipPath(input: AgentOverlayClipInput): string {
  const insets = agentOverlayClipInsets(input)
  return `inset(${insets.top}px ${insets.right}px ${insets.bottom}px ${insets.left}px)`
}
