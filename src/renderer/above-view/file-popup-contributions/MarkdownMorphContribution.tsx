// ADR 0013 §3 — markdown file popup contribution. Surfaces the leading
// short/long toggle on `.md` selections; clicking `short` morphs the file
// entity back into a plain-text entity at the same canvas rect.

import type { CanvasSceneFileEntity } from '../../../shared/types'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'
import { TextKindToggle } from '../TextKindToggle'

export function MarkdownMorphContribution({
  api,
  isDark,
  entity,
}: {
  api: Pick<CanvasBgElectronAPI, 'morphTextFile'>
  isDark: boolean
  entity: CanvasSceneFileEntity
}) {
  return (
    <TextKindToggle
      isDark={isDark}
      active="long"
      onPick={(kind) => {
        if (kind === 'short') {
          void api.morphTextFile(entity.id, 'file-to-text')
        }
      }}
    />
  )
}
