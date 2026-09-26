import { Switch } from '@base-ui/react/switch'
import type { CursorVisibilityPrefs } from '../../shared/types'

const ROWS: { key: keyof CursorVisibilityPrefs; title: string; description: string }[] = [
  {
    key: 'hideAgentCursors',
    title: 'Agent cursors',
    description: 'Show a cursor where an agent is working, on the canvas and inside pages.',
  },
  {
    key: 'hideSyncedCursors',
    title: 'Synced cursors',
    description: 'Show your cursor mirrored onto the other pages of a sync set while you browse one.',
  },
]

export function CursorVisibilitySection({
  cursorVisibility,
  onChange,
}: {
  cursorVisibility: CursorVisibilityPrefs
  onChange: (next: Partial<CursorVisibilityPrefs>) => void
}) {
  return (
    <div className="mt-7">
      <div className="mb-2">
        <h3 className="text-[13px] font-medium">Cursors</h3>
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          Hiding a cursor only stops drawing it. Agents keep working and synced pages keep
          following.
        </p>
      </div>

      <div className="flex flex-col gap-2">
        {ROWS.map((row) => (
          <label
            key={row.key}
            className="flex cursor-pointer select-none items-start gap-3 rounded-[8px] border border-[var(--surface-card-border)] bg-[var(--surface-card)] px-4 py-3"
          >
            <div className="min-w-0 flex-1">
              <span className="text-[13px] font-medium">{row.title}</span>
              <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
                {row.description}
              </p>
            </div>
            <div className="pt-[2px]">
              <Switch.Root
                checked={!cursorVisibility[row.key]}
                onCheckedChange={(shown) => onChange({ [row.key]: !shown })}
                className="relative inline-flex h-[18px] w-[32px] shrink-0 cursor-pointer items-center rounded-full border border-[var(--surface-switch-track-border)] bg-[var(--surface-switch-track)] transition-colors data-[checked]:border-[var(--surface-switch-track-checked-border)] data-[checked]:bg-[var(--surface-switch-track-checked)]"
              >
                <Switch.Thumb className="block h-[14px] w-[14px] translate-x-[1px] rounded-full bg-white shadow-sm transition-transform data-[checked]:translate-x-[15px]" />
              </Switch.Root>
            </div>
          </label>
        ))}
      </div>
    </div>
  )
}
