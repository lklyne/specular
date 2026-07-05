import type { CanvasSceneFileEntity } from '../../../shared/types'
import { filePathToSrcVersioned } from './filePathToSrc'

export function HtmlInlineRenderer({
  entity,
  isSelected,
}: {
  entity: CanvasSceneFileEntity
  isSelected: boolean
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
        pointerEvents: isSelected ? 'auto' : 'none',
        background: 'white',
      }}
    />
  )
}
