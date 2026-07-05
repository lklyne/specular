import type { CanvasSceneFileEntity } from '../../../shared/types'
import { filePathToSrcVersioned } from './filePathToSrc'

export function ImageInlineRenderer({ entity }: { entity: CanvasSceneFileEntity }) {
  const fileName = entity.file.split('/').pop() ?? entity.file
  return (
    <img
      key={entity.fileReloadVersion ?? 0}
      src={filePathToSrcVersioned(entity.file, entity.fileReloadVersion)}
      alt={fileName}
      draggable={false}
      style={{
        width: '100%',
        height: '100%',
        objectFit: entity.objectFit ?? 'contain',
        pointerEvents: 'none',
        // Defer selection to Specular's own selection system; the native
        // browser selection can otherwise leave images in a stuck highlighted state.
        userSelect: 'none',
        WebkitUserSelect: 'none',
      }}
    />
  )
}
