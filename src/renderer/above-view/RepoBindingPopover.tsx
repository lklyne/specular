import { useState } from 'react'
import { FolderCode } from 'lucide-react'
import { PresetPopover } from '../shared/PresetPopover'
import { CanvasItemPopup } from './CanvasItemPopup'

/**
 * The page popup's repo control: a popdown that shows which local folder the
 * page's origin writes to. The path sits in a file well — one button shaped
 * like a field that opens the folder picker wherever it is clicked.
 */
export function RepoBindingPopover({
  isDark,
  origin,
  boundRepoPath,
  onPick,
  onUnlink,
}: {
  isDark: boolean
  origin: string
  boundRepoPath: string | null
  onPick: () => void
  onUnlink: () => void
}) {
  const [open, setOpen] = useState(false)
  const act = (fn: () => void) => {
    setOpen(false)
    fn()
  }

  return (
    <PresetPopover
      isDark={isDark}
      open={open}
      onOpenChange={setOpen}
      align="end"
      trigger={
        <CanvasItemPopup.IconButton
          isDark={isDark}
          title={boundRepoPath ? `Repo: ${boundRepoPath}` : 'Link a repo'}
          ariaLabel={boundRepoPath ? `Repo ${boundRepoPath}` : 'Link a repo for this site'}
          onClick={() => setOpen((value) => !value)}
        >
          <FolderCode size={14} />
        </CanvasItemPopup.IconButton>
      }
    >
      <div className="flex w-80 flex-col gap-2 p-3">
        <div className="truncate text-xs text-[var(--surface-foreground-muted)]" title={origin}>
          {origin}
        </div>
        <button
          type="button"
          title={boundRepoPath ?? undefined}
          onClick={() => act(onPick)}
          className={`flex w-full items-center gap-2 rounded-[6px] border py-1 pl-2 pr-1 text-left text-xs outline-none transition-colors focus-visible:ring-1 focus-visible:ring-blue-500/40 ${
            isDark
              ? 'border-zinc-700 bg-zinc-950 hover:border-zinc-600'
              : 'border-zinc-300 bg-white hover:border-zinc-400'
          }`}
        >
          <span
            className={`min-w-0 flex-1 truncate ${
              boundRepoPath ? 'text-[var(--surface-foreground)]' : 'text-[var(--surface-foreground-muted)]'
            }`}
          >
            {boundRepoPath ? tildePath(boundRepoPath) : 'No repo linked'}
          </span>
          <span
            className={`shrink-0 rounded-[4px] px-1.5 py-0.5 text-[11px] font-medium ${
              isDark
                ? 'bg-zinc-800 text-[var(--surface-foreground)]'
                : 'bg-[var(--color-stone-200)] text-[var(--surface-foreground)]'
            }`}
          >
            {boundRepoPath ? 'Change…' : 'Choose…'}
          </span>
        </button>
        {boundRepoPath ? (
          <div className="flex justify-end">
            <button
              type="button"
              className="rounded-[6px] px-1.5 py-0.5 text-xs text-[var(--surface-foreground-muted)] transition-colors hover:text-[var(--surface-foreground)]"
              onClick={() => act(onUnlink)}
            >
              Unlink
            </button>
          </div>
        ) : null}
      </div>
    </PresetPopover>
  )
}

function tildePath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, '~')
}
