import { Image } from 'lucide-react'
import type { PanelFileEntityDetail } from '../../../shared/types'
import { fileEntityLabel, mutedClass } from '../rightDetailsPanelHelpers'
import { rightDetailsPanelApi } from '../rightDetailsPanelApi'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { FileDeviceSection } from './FileDeviceSection'
import { FileEntityShell } from './FileEntityShell'

const FIT_OPTIONS: Array<{ value: 'contain' | 'cover' | 'fill'; label: string }> = [
  { value: 'contain', label: 'Contain' },
  { value: 'cover', label: 'Cover' },
  { value: 'fill', label: 'Fill' },
]

export function ImageFilePane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass
  const activeFit = fileEntity.objectFit ?? 'contain'

  return (
    <FileEntityShell
      icon={<Image size={14} className="shrink-0 text-[var(--surface-foreground-muted)]" />}
      label={fileEntityLabel(fileEntity.file)}
      entityId={fileEntity.id}
    >
      <FileDeviceSection fileEntity={fileEntity} />

      <PaneField label="Object Fit">
        <div className="flex gap-1">
          {FIT_OPTIONS.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => rightDetailsPanelApi.updateEntity('file', fileEntity.id, { objectFit: opt.value })}
              className={`rounded px-2 py-1 text-[11px] font-medium transition-colors ${
                activeFit === opt.value
                  ? isDark
                    ? 'bg-zinc-700 text-[var(--surface-foreground)]'
                    : 'bg-zinc-200 text-[var(--surface-foreground)]'
                  : isDark
                    ? 'text-[var(--surface-foreground-muted)] hover:bg-zinc-800 hover:text-[var(--surface-foreground)]'
                    : 'text-[var(--surface-foreground-muted)] hover:bg-zinc-100'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </PaneField>

      <PaneField label="Dimensions">
        <div className={`text-[11px] ${muted}`}>
          {fileEntity.width} × {fileEntity.height}
        </div>
      </PaneField>

      <PaneField label="Path">
        <div
          className={`break-all rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
          title={fileEntity.file}
        >
          {fileEntity.file}
        </div>
      </PaneField>
    </FileEntityShell>
  )
}
