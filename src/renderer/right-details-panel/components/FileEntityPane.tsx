import { File } from 'lucide-react'
import type { PanelFileEntityDetail } from '../../../shared/types'
import { fileEntityLabel, mutedClass } from '../rightDetailsPanelHelpers'
import { usePaneTheme } from '../PaneContext'
import { PaneField } from './PaneSection'
import { ImageFilePane } from './ImageFilePane'
import { ComponentFilePane } from './ComponentFilePane'
import { MarkdownFilePane } from './MarkdownFilePane'
import { FileDeviceSection } from './FileDeviceSection'
import { FileEntityShell } from './FileEntityShell'

function GenericFilePane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  const isDark = usePaneTheme()
  const muted = mutedClass

  return (
    <FileEntityShell
      icon={<File size={14} className="shrink-0 text-[var(--surface-foreground-muted)]" />}
      label={fileEntityLabel(fileEntity.file)}
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

      {fileEntity.subpath ? (
        <PaneField label="Subpath">
          <div className={`text-[11px] ${muted}`}>{fileEntity.subpath}</div>
        </PaneField>
      ) : null}
    </FileEntityShell>
  )
}

export function FileEntityPane({ fileEntity }: { fileEntity: PanelFileEntityDetail }) {
  switch (fileEntity.fileType) {
    case 'image':
      return <ImageFilePane fileEntity={fileEntity} />
    case 'markdown':
      return <MarkdownFilePane fileEntity={fileEntity} />
    case 'component':
      return <ComponentFilePane fileEntity={fileEntity} />
    case 'video':
      return <ImageFilePane fileEntity={fileEntity} />
    default:
      return <GenericFilePane fileEntity={fileEntity} />
  }
}
