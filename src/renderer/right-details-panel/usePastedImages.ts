import { useCallback, useState, type ClipboardEvent } from 'react'
import { isThreadImageMediaType, type ThreadImageUpload } from '../../shared/agent-thread'
import { rightDetailsPanelApi } from './rightDetailsPanelApi'

export type PastedImage = ThreadImageUpload & { id: string; url: string }

let nextId = 0

function readImage(file: File): Promise<PastedImage | null> {
  return new Promise((resolve) => {
    const reader = new FileReader()
    reader.onload = () => {
      const url = typeof reader.result === 'string' ? reader.result : ''
      const data = url.slice(url.indexOf(',') + 1)
      resolve(url && isThreadImageMediaType(file.type)
        ? { id: `pasted-${nextId++}`, url, data, mediaType: file.type }
        : null)
    }
    reader.onerror = () => resolve(null)
    reader.readAsDataURL(file)
  })
}

/** Images pasted into the composer, held until the turn is sent. */
export function usePastedImages() {
  const [images, setImages] = useState<PastedImage[]>([])

  const onPaste = useCallback((event: ClipboardEvent<HTMLElement>) => {
    const files = Array.from(event.clipboardData.files).filter((file) =>
      isThreadImageMediaType(file.type),
    )
    if (files.length > 0) {
      event.preventDefault()
      void Promise.all(files.map(readImage)).then((read) => {
        const added = read.filter((image): image is PastedImage => image !== null)
        if (added.length) setImages((prev) => [...prev, ...added])
      })
      return
    }
    // macOS screenshots and images copied from apps like Preview arrive as
    // native pasteboard data (often TIFF) with no DOM file; main reads those.
    // A paste that carries text is a text paste, so it's left alone.
    if (event.clipboardData.getData('text/plain')) return
    void rightDetailsPanelApi.readClipboardImage().then((upload) => {
      if (!upload) return
      const image: PastedImage = {
        ...upload,
        id: `pasted-${nextId++}`,
        url: `data:${upload.mediaType};base64,${upload.data}`,
      }
      setImages((prev) => [...prev, image])
    })
  }, [])

  const remove = useCallback((id: string) => {
    setImages((prev) => prev.filter((image) => image.id !== id))
  }, [])

  const clear = useCallback(() => setImages([]), [])

  return { images, onPaste, remove, clear }
}
