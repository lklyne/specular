import { useEffect, useState } from 'react'
import { Select } from '@base-ui/react/select'
import { Check, ChevronDown } from 'lucide-react'
import type { FixConfig, FixModel, FixPermissions } from '../../shared/types'
import type { SettingsElectronAPI } from '../../shared/electron-api/settings'

const MODEL_OPTIONS: { value: FixModel; label: string }[] = [
  { value: 'opus', label: 'Opus' },
  { value: 'sonnet', label: 'Sonnet' },
  { value: 'haiku', label: 'Haiku' },
]

const PERMISSION_OPTIONS: { value: FixPermissions; label: string; hint: string }[] = [
  {
    value: 'auto',
    label: 'Auto',
    hint: 'Claude approves routine actions itself and declines anything risky.',
  },
  {
    value: 'acceptEdits',
    label: 'Edit and verify',
    hint: 'Claude can read and edit files and run typecheck, tests, and read-only git. Anything else is skipped.',
  },
  {
    value: 'dangerously',
    label: 'Bypass permissions',
    hint: 'Claude runs every tool without asking, including arbitrary shell commands. Only use this on repos you trust.',
  },
]

export function FixConfigPane({
  api,
  fixConfig,
}: {
  api: SettingsElectronAPI
  fixConfig: FixConfig
}) {
  const [model, setModel] = useState<FixModel>(fixConfig.model)
  const [permissions, setPermissions] = useState<FixPermissions>(fixConfig.permissions)

  useEffect(() => {
    setModel(fixConfig.model)
    setPermissions(fixConfig.permissions)
  }, [fixConfig.model, fixConfig.permissions])

  const dirty = model !== fixConfig.model || permissions !== fixConfig.permissions

  return (
    <section>
      <header className="mb-4 mt-2">
        <p className="mt-1 text-[12px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-70">
          Fix uses Claude Code to read your comments and make changes in linked repositories. Pick a model and permission level.
        </p>
      </header>

      <div className="flex flex-col gap-4">
        <SettingsSelect
          label="Model"
          value={model}
          onValueChange={setModel}
          options={MODEL_OPTIONS}
        />

        <SettingsSelect
          label="Permissions"
          value={permissions}
          onValueChange={setPermissions}
          options={PERMISSION_OPTIONS}
          hint={PERMISSION_OPTIONS.find((o) => o.value === permissions)?.hint}
        />

        <div className="flex justify-end">
          <button
            type="button"
            disabled={!dirty}
            onClick={() => api.setFixConfig({ model, permissions })}
            className="rounded-[6px] bg-[var(--surface-toolbar-foreground)] px-4 py-[6px] text-[12px] font-medium text-[var(--surface-panel)] shadow-sm enabled:hover:opacity-90 disabled:opacity-50"
          >
            Save
          </button>
        </div>
      </div>
    </section>
  )
}

function SettingsSelect<T extends string>({
  label,
  value,
  onValueChange,
  options,
  hint,
}: {
  label: string
  value: T
  onValueChange: (next: T) => void
  options: { value: T; label: string }[]
  hint?: string
}) {
  return (
    <div>
      <label className="mb-1 block text-[11px] font-medium">{label}</label>
      <Select.Root value={value} onValueChange={(v) => v !== null && onValueChange(v as T)}>
        <Select.Trigger className="flex w-full items-center justify-between gap-2 rounded-[6px] border border-[var(--surface-popover-border)] bg-[var(--surface-input)] px-3 py-[6px] text-[12px] text-left">
          <Select.Value />
          <Select.Icon className="text-[var(--surface-toolbar-foreground)] opacity-60">
            <ChevronDown size={12} />
          </Select.Icon>
        </Select.Trigger>
        <Select.Portal>
          <Select.Positioner sideOffset={4} alignItemWithTrigger={false} className="z-[1000]">
            <Select.Popup className="min-w-[var(--anchor-width)] rounded-[6px] border border-[var(--surface-popover-border)] bg-[var(--surface-popover)] p-1 text-[12px] shadow-md outline-none">
              {options.map((opt) => (
                <Select.Item
                  key={opt.value}
                  value={opt.value}
                  className="flex cursor-default items-center justify-between gap-2 rounded-[4px] px-2 py-[5px] text-[var(--surface-toolbar-foreground)] outline-none data-[highlighted]:bg-[var(--surface-popover-subtle)]"
                >
                  <Select.ItemText>{opt.label}</Select.ItemText>
                  <Select.ItemIndicator>
                    <Check size={12} />
                  </Select.ItemIndicator>
                </Select.Item>
              ))}
            </Select.Popup>
          </Select.Positioner>
        </Select.Portal>
      </Select.Root>
      {hint ? (
        <p className="mt-1 text-[10px] leading-snug text-[var(--surface-toolbar-foreground)] opacity-60">
          {hint}
        </p>
      ) : null}
    </div>
  )
}
