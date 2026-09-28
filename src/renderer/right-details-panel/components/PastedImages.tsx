import { X } from 'lucide-react'
import type { PastedImage } from '../usePastedImages'

/** Thumbnails of images pasted into the composer; hover one to remove it. */
export function PastedImages({
  images,
  onRemove,
}: {
  images: PastedImage[]
  onRemove: (id: string) => void
}) {
  if (images.length === 0) return null
  return (
    <div className="flex flex-wrap gap-1.5 pb-1.5">
      {images.map((image) => (
        <div
          key={image.id}
          className="group relative h-12 w-12 shrink-0 overflow-hidden rounded-md border border-zinc-300 dark:border-zinc-600"
        >
          <img src={image.url} alt="Pasted image" className="h-full w-full object-cover" />
          <button
            type="button"
            aria-label="Remove image"
            title="Remove image"
            className="absolute right-0.5 top-0.5 flex h-4 w-4 items-center justify-center rounded-full bg-black/70 text-white opacity-0 transition-opacity hover:bg-black focus-visible:opacity-100 group-hover:opacity-100"
            onClick={() => onRemove(image.id)}
          >
            <X size={10} strokeWidth={2.5} />
          </button>
        </div>
      ))}
    </div>
  )
}
