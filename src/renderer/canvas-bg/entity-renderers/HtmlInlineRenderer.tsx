import { useEffect, useRef, useState } from 'react'
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
  const reloadVersion = entity.fileReloadVersion ?? 0
  const flash = useReloadFlash(reloadVersion)
  return (
    <div style={{ position: 'relative', width: '100%', height: '100%' }}>
      <iframe
        key={reloadVersion}
        src={filePathToSrcVersioned(entity.file, reloadVersion)}
        title={fileName}
        style={{
          width: '100%',
          height: '100%',
          border: 'none',
          pointerEvents: isInteractive ? 'auto' : 'none',
          background: 'white',
        }}
      />
      {flash !== null ? (
        <div
          key={`flash-${flash}`}
          aria-hidden
          className="file-refresh-flash pointer-events-none absolute inset-0"
          style={{ boxShadow: 'inset 0 0 0 2px var(--color-blue-500, #3b82f6)' }}
        />
      ) : null}
    </div>
  )
}

/** Fires a one-shot flash key whenever the reload version advances, skipping
 *  the initial mount (no flash for a freshly-placed entity). */
function useReloadFlash(reloadVersion: number): number | null {
  const [flashKey, setFlashKey] = useState<number | null>(null)
  const previousRef = useRef(reloadVersion)
  useEffect(() => {
    if (reloadVersion === previousRef.current) return
    previousRef.current = reloadVersion
    setFlashKey(reloadVersion)
  }, [reloadVersion])
  return flashKey
}
