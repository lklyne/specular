import { useEffect, useRef } from 'react'
import type { BindingId } from '../../../shared/bindings'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'

const api = (window as unknown as { electronAPI: CanvasBgElectronAPI }).electronAPI

export function useRendererBindingHandlers(
  handlers: Partial<Record<BindingId, () => void>>,
): void {
  const handlersRef = useRef(handlers)
  useEffect(() => {
    handlersRef.current = handlers
  })

  useEffect(() => {
    return api.onBindingFire((id) => {
      const handler = handlersRef.current[id]
      if (handler) handler()
    })
  }, [])
}
