import { Bot, FolderGit2, Settings2, Sparkles } from 'lucide-react'
import type { ComponentType } from 'react'

export type SettingsSection = 'general' | 'skills' | 'models' | 'repos'

const ITEMS: { id: SettingsSection; label: string; icon: ComponentType<{ size?: number }> }[] = [
  { id: 'general', label: 'General', icon: Settings2 },
  { id: 'skills', label: 'Skills', icon: Sparkles },
  { id: 'models', label: 'Models', icon: Bot },
  { id: 'repos', label: 'Repos', icon: FolderGit2 },
]

export function Sidebar({
  active,
  onChange,
}: {
  active: SettingsSection
  onChange: (next: SettingsSection) => void
}) {
  return (
    <nav className="flex w-[180px] shrink-0 flex-col border-r border-[var(--surface-popover-border)] bg-[var(--surface-panel)]">
      <div className="titlebar-drag h-[34px] w-full shrink-0" />
      <ul className="flex flex-col gap-[2px] px-2 pb-3 pt-2">
        {ITEMS.map((item) => {
          const Icon = item.icon
          const isActive = active === item.id
          return (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => onChange(item.id)}
                className={`flex w-full items-center gap-2 rounded-[6px] px-2 py-[6px] text-left text-[13px] ${
                  isActive
                    ? 'bg-[var(--surface-popover-border)] text-[var(--surface-toolbar-foreground)]'
                    : 'text-[var(--surface-toolbar-foreground)] opacity-70 hover:opacity-100'
                }`}
              >
                <Icon size={14} />
                <span>{item.label}</span>
              </button>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
