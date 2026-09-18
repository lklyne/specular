import { Menu } from '@base-ui/react/menu'
import { Check, ChevronDown } from 'lucide-react'
import type { FixConfig, FixModel } from '../../../shared/types'
import { usePaneTheme } from '../PaneContext'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { composerChipClass } from './ContextChip'

const MODEL_LABELS: Record<FixModel, string> = {
  opus: 'Opus',
  sonnet: 'Sonnet',
  haiku: 'Haiku',
}

/** Pill dropdown picking the model the thread's agent runs on. */
export function ModelChip({ fixConfig }: { fixConfig: FixConfig }) {
  const isDark = usePaneTheme()
  return (
    <Menu.Root>
      <Menu.Trigger className={`${composerChipClass(isDark)} shrink-0`}>
        {MODEL_LABELS[fixConfig.model]}
        <ChevronDown size={11} className="shrink-0" />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner sideOffset={6}>
          <Menu.Popup
            className={`z-50 min-w-28 rounded-[10px] border p-1 shadow-xl outline-none ${
              isDark
                ? 'border-zinc-700 bg-zinc-900 text-[var(--surface-foreground)]'
                : 'border-zinc-200 bg-white text-[var(--surface-foreground)]'
            }`}
          >
            {(Object.keys(MODEL_LABELS) as FixModel[]).map((model) => (
              <Menu.Item
                key={model}
                className={`flex cursor-default items-center gap-2 rounded-[7px] px-2.5 py-1.5 text-xs outline-none ${
                  isDark ? 'data-[highlighted]:bg-zinc-800' : 'data-[highlighted]:bg-zinc-100'
                }`}
                onClick={() =>
                  rightDetailsPanelApi.setFixConfig({ model, permissions: fixConfig.permissions })
                }
              >
                <span>{MODEL_LABELS[model]}</span>
                {model === fixConfig.model ? <Check size={12} className="ml-auto shrink-0" /> : null}
              </Menu.Item>
            ))}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}
