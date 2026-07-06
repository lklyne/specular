import type { CanvasSceneFileEntity } from '../../../shared/types'
import { filePathToSrcVersioned } from './filePathToSrc'

export function HtmlInlineRenderer({
  entity,
  isInteractive,
}: {
  entity: CanvasSceneFileEntity
  /** Select-first / interact-second: the iframe only captures the pointer
   *  once the user has entered it (second click / double-click). A merely-
   *  selected file stays click-through so the first click selects/drags. */
  isInteractive: boolean
}) {
  const fileName = entity.file.split('/').pop() ?? entity.file
  return (
    <iframe
      key={entity.fileReloadVersion ?? 0}
      src={filePathToSrcVersioned(entity.file, entity.fileReloadVersion)}
      title={fileName}
      style={{
        width: '100%',
        height: '100%',
        border: 'none',
        pointerEvents: isInteractive ? 'auto' : 'none',
        background: 'white',
      }}
    />
  )
}
