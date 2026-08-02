import { useState } from 'react'
import { Folder } from 'lucide-react'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'

export function GeneralPane({
  api,
  space,
  onSpaceChange,
}: {
  api: SettingsElectronAPI
  space: { path: string; isDefault: boolean }
  onSpaceChange: (next: { path: string; isDefault: boolean }) => void
}) {
  const [changing, setChanging] = useState(false)

  const handleChange = async () => {
    setChanging(true)
    try {
      const nextPath = await api.spaceChangeViaPicker()
      if (nextPath) onSpaceChange({ path: nextPath, isDefault: false })
    } finally {
      setChanging(false)
    }
  }

  const segments = space.path.split('/').filter(Boolean)
  const name = segments[segments.length - 1] ?? space.path
  const parent = segments.slice(0, -1).join(' › ')

  return (
    <section>
      <header className="mb-4 mt-2">
        <h2 className="text-[15px] font-semibold">General</h2>
      </header>

      <div className="mb-2">
        <h3 className="text-[13px] font-medium">Space</h3>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          The folder holding your canvases, images, and notes.
        </p>
      </div>

      <div className="flex items-center justify-between gap-3 rounded-[8px] border border-[var(--surface-popover-border)] px-3 py-[10px]">
        <div className="flex min-w-0 items-center gap-2">
          <Folder size={16} className="shrink-0 opacity-70" />
          <div className="min-w-0">
            <div className="truncate text-[13px] font-medium">{name}</div>
            {parent ? (
              <div className="truncate text-[11px] text-[var(--surface-toolbar-foreground)] opacity-60">
                {parent}
              </div>
            ) : null}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={() => api.spaceRevealInFinder()}
            className="rounded-[6px] border border-[var(--surface-popover-border)] px-[10px] py-[5px] text-[12px] text-[var(--surface-toolbar-foreground)] hover:bg-[var(--surface-popover-border)]"
          >
            Reveal in Finder
          </button>
          <button
            type="button"
            onClick={handleChange}
            disabled={changing}
            className="rounded-[6px] border border-[var(--surface-popover-border)] px-[10px] py-[5px] text-[12px] text-[var(--surface-toolbar-foreground)] hover:bg-[var(--surface-popover-border)] disabled:pointer-events-none disabled:opacity-50"
          >
            Change…
          </button>
        </div>
      </div>
    </section>
  )
}
