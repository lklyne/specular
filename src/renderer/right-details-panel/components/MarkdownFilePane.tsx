import { FileText } from 'lucide-react'
import type { PanelFileEntityDetail } from '../../../shared/types'
import { fileEntityLabel, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { FileDeviceSection } from './FileDeviceSection'
import { FileEntityShell } from './FileEntityShell'

export function MarkdownFilePane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass(isDark)
  const label = fileEntityLabel(fileEntity.file).replace(/\.md$/i, '') || 'Note'

  return (
    <FileEntityShell
      icon={<FileText size={14} className="shrink-0 text-zinc-500" />}
      label={label}
      entityId={fileEntity.id}
    >
      <FileDeviceSection fileEntity={fileEntity} />

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
