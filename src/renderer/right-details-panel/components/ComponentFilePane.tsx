import { Code2 } from 'lucide-react'
import type { PanelFileEntityDetail } from '../../../shared/types'
import { fileEntityLabel, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { FileEntityShell } from './FileEntityShell'

export function ComponentFilePane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass

  return (
    <FileEntityShell
      icon={<Code2 size={14} className="shrink-0 text-[var(--surface-panel-foreground-muted)]" />}
      label={fileEntityLabel(fileEntity.file)}
      entityId={fileEntity.id}
    >
      <PaneField label="Renderer">
        <div className={`text-[11px] ${muted}`}>Component (live · live preview ships next)</div>
      </PaneField>

      <PaneField label="Path">
        <div
          className={`break-all rounded px-2 py-1.5 text-[11px] leading-5 ${isDark ? 'bg-zinc-800' : 'bg-zinc-100'}`}
        >
          {fileEntity.file}
        </div>
      </PaneField>
    </FileEntityShell>
  )
}
