/**
 * Diagram colors live on the container, not in the SVG. beautiful-mermaid
 * derives every color from `--bg` / `--fg` custom properties on the `<svg>`,
 * and the rendered svg reads those from `--diagram-bg` / `--diagram-fg` (see
 * `render-mermaid.ts`), so a theme flip restyles a diagram without re-laying
 * it out. Kept free of the renderer import so the diagram chunk stays lazy.
 */

/** Matches the note card surface and text color in each theme. */
export const DIAGRAM_LIGHT = { bg: '#fafaf9', fg: '#1c1917' } as const
export const DIAGRAM_DARK = { bg: '#1c1917', fg: '#e7e5e4' } as const

export function diagramThemeVars(isDark: boolean): Record<string, string> {
  const colors = isDark ? DIAGRAM_DARK : DIAGRAM_LIGHT
  return { '--diagram-bg': colors.bg, '--diagram-fg': colors.fg }
}
