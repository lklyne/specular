import { useEffect, useState } from 'react'
import {
  Code,
  File,
  Folder,
  MessageSquare,
  PenLine,
  Spline,
  SquareDashedMousePointer,
  StickyNote,
} from 'lucide-react'
import type { ThreadPill } from '../../../shared/agent-thread'
import { pillLabel } from '../../../shared/agent-thread'
import type { DevtoolsPanelData } from '../../../shared/types'
import { iconForFilePath } from '../../shared/fileIcon'
import { usePaneTheme } from '../PaneContext'
import { viewportIcon } from '../../shared/pageListItem'
import { ShapeGlyph } from '../../shared/ShapeGlyph'

/** Shared pill styling for the composer's context and model chips. */
export function composerChipClass(isDark: boolean): string {
  return `inline-flex min-w-0 max-w-full items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium text-[var(--surface-foreground-muted)] ${
    isDark ? 'border-zinc-600 bg-zinc-800' : 'border-zinc-300 bg-zinc-100'
  }`
}

/** The composer's context pill: what the thread is anchored to. */
export function ContextChip({ pill, data }: { pill: ThreadPill; data: DevtoolsPanelData }) {
  const isDark = usePaneTheme()
  return (
    <span className={composerChipClass(isDark)}>
      <ChipIcon pill={pill} data={data} />
      <span className="truncate">{chipLabel(pill, data)}</span>
    </span>
  )
}

function chipLabel(pill: ThreadPill, data: DevtoolsPanelData): string {
  if (pill.kind === 'empty') {
    const canvasName = data.canvasName?.trim()
    if (canvasName) return canvasName
    return 'specular'
  }
  if (pill.kind === 'selection' && pill.label === 'page') {
    return data.selection?.pageTitle || pill.label
  }
  return pillLabel(pill)
}

function ChipIcon({ pill, data }: { pill: ThreadPill; data: DevtoolsPanelData }) {
  const mode = data.panelMode
  if (pill.kind === 'dom') return <Code size={11} className="shrink-0" />
  if (pill.kind === 'annotation') return <MessageSquare size={11} className="shrink-0" />
  if (pill.kind === 'selection') {
    switch (mode.kind) {
      case 'multi':
        return <SquareDashedMousePointer size={11} className="shrink-0" />
      case 'page':
        return <PageChipIcon data={data} />
      case 'text':
        return <StickyNote size={11} className="shrink-0" />
      case 'drawing':
        return <PenLine size={11} className="shrink-0" />
      case 'shape':
        return (
          <span className="shrink-0">
            <ShapeGlyph kind={data.shapeEntity?.shapeKind ?? 'rectangle'} size={11} />
          </span>
        )
      case 'edge':
        return <Spline size={11} className="shrink-0" />
      case 'group':
        return <Folder size={11} className="shrink-0" />
      case 'file': {
        const Icon = data.fileEntity ? iconForFilePath(data.fileEntity.file) : File
        return <Icon size={11} className="shrink-0" />
      }
    }
  }
  return <File size={11} className="shrink-0" />
}

function PageChipIcon({ data }: { data: DevtoolsPanelData }) {
  const faviconUrl = data.pages?.find((page) => page.id === data.selection?.pageId)?.faviconUrl
  const [imageFailed, setImageFailed] = useState(false)
  useEffect(() => {
    setImageFailed(false)
  }, [faviconUrl])
  if (faviconUrl && !imageFailed) {
    return (
      <img
        alt=""
        aria-hidden="true"
        src={faviconUrl}
        className="h-[11px] w-[11px] shrink-0 rounded-[2px]"
        onError={() => setImageFailed(true)}
      />
    )
  }
  const Icon = viewportIcon(data.selection?.viewportLabel ?? '', data.selection?.width)
  return <Icon size={11} className="shrink-0" />
}
