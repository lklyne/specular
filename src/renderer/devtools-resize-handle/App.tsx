import { useEffect, useRef, useState } from 'react'
import type { ThemeData } from '../../shared/types'
import type { DevtoolsResizeHandleElectronAPI } from '../../shared/electron-api/devtools-resize-handle'

export default function App({
  api,
  initialTheme,
}: {
  api: DevtoolsResizeHandleElectronAPI
  initialTheme: ThemeData
}) {
  const [isDark, setIsDark] = useState(initialTheme.isDark)
  const pointerIdRef = useRef<number | null>(null)
  const handleRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => api.onThemeChanged((data) => setIsDark(data.isDark)), [api])

  const endResize = (pointerId: number | null) => {
    if (pointerId === null) return
    const handle = handleRef.current
    if (handle?.hasPointerCapture(pointerId)) {
      handle.releasePointerCapture(pointerId)
    }
    pointerIdRef.current = null
    api.devtoolsResizeEnd()
  }

  return (
    <div
      ref={handleRef}
      onPointerDown={(event) => {
        pointerIdRef.current = event.pointerId
        event.currentTarget.setPointerCapture(event.pointerId)
        api.devtoolsResizeStart(event.screenX)
        event.preventDefault()
      }}
      onPointerMove={(event) => {
        if (pointerIdRef.current !== event.pointerId) return
        api.devtoolsResizeMove(event.screenX)
      }}
      onPointerUp={(event) => {
        if (pointerIdRef.current !== event.pointerId) return
        endResize(event.pointerId)
      }}
      onPointerCancel={(event) => {
        if (pointerIdRef.current !== event.pointerId) return
        endResize(event.pointerId)
      }}
      onLostPointerCapture={(event) => {
        if (pointerIdRef.current !== event.pointerId) return
        endResize(event.pointerId)
      }}
      style={{
        position: 'relative',
        width: '100%',
        height: '100%',
        cursor: 'ew-resize',
        touchAction: 'none',
        background: 'transparent',
      }}
    >
      <div
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          bottom: 0,
          width: 1,
          background: isDark ? '#3f3f46' : '#d4d4d8',
          pointerEvents: 'none',
        }}
      />
    </div>
  )
}
