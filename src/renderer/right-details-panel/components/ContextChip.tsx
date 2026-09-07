import { useEffect, useState } from 'react'
import {
  Code,
  File,
  Folder,
  PenLine,
  Spline,
  SquareDashedMousePointer,
  StickyNote,
} from 'lucide-react'
import type { ThreadPill } from '../../../shared/agent-thread'
import { pillLabel } from '../../../shared/agent-thread'
import type { DevtoolsPanelData, DevtoolsPanelPageSummary } from '../../../shared/types'
import { iconForFilePath } from '../../shared/fileIcon'
import { usePaneTheme } from '../PaneContext'
import { viewportIcon } from '../../shared/pageListItem'
import { ShapeGlyph } from '../../shared/ShapeGlyph'

/**
 * Shared pill styling for the composer's context and model chips. They read as
 * plain labels at rest and only take on a pill on hover, so the composer row
 * stays quiet next to the message field.
 */
export function composerChipClass(isDark: boolean): string {
  return `inline-flex min-w-0 max-w-full items-center gap-1 rounded-full px-1.5 py-0.5 text-[11px] font-medium text-[var(--surface-foreground-muted)] transition-colors ${
    isDark ? 'hover:bg-zinc-800' : 'hover:bg-zinc-200/70'
  }`
}

/**
 * The composer's context pill: where this turn is aimed. It names the page or
 * entity the thread is anchored to — a queued comment's own text shows in the
 * queue above the message field, so the chip names the comment's page instead.
 */
export function ContextChip({ pill, data }: { pill: ThreadPill; data: DevtoolsPanelData }) {
  const isDark = usePaneTheme()
  return (
    <span className={composerChipClass(isDark)}>
      <ChipIcon pill={pill} data={data} />
      <span className="truncate">{chipLabel(pill, data)}</span>
    </span>
  )
}

/** The page a comment sits on, or null when it is canvas-bound. */
function annotationPage(pill: ThreadPill, data: DevtoolsPanelData): DevtoolsPanelPageSummary | null {
  if (pill.kind !== 'annotation') return null
  const pageId = (data.annotations ?? []).find((item) => item.id === pill.annotationId)?.pageAnchor
    ?.pageId
  if (!pageId) return null
  return data.pages?.find((page) => page.id === pageId) ?? null
}

function canvasLabel(data: DevtoolsPanelData): string {
  return data.canvasName?.trim() || 'specular'
}

function chipLabel(pill: ThreadPill, data: DevtoolsPanelData): string {
  const page = annotationPage(pill, data)
  if (page) return page.label
  if (pill.kind === 'annotation' || pill.kind === 'empty') return canvasLabel(data)
  if (pill.kind === 'selection' && pill.label === 'page') {
    return data.selection?.pageTitle || pill.label
  }
  return pillLabel(pill)
}

function ChipIcon({ pill, data }: { pill: ThreadPill; data: DevtoolsPanelData }) {
  const mode = data.panelMode
  const commentPage = annotationPage(pill, data)
  if (commentPage) {
    return <PageGlyph faviconUrl={commentPage.faviconUrl} width={commentPage.width} />
  }
  if (pill.kind === 'dom') return <Code size={11} className="shrink-0" />
  if (pill.kind === 'selection') {
    switch (mode.kind) {
      case 'multi':
        return <SquareDashedMousePointer size={11} className="shrink-0" />
      case 'page':
        return <SelectedPageGlyph data={data} />
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

function SelectedPageGlyph({ data }: { data: DevtoolsPanelData }) {
  const faviconUrl = data.pages?.find((page) => page.id === data.selection?.pageId)?.faviconUrl
  return (
    <PageGlyph
      faviconUrl={faviconUrl}
      width={data.selection?.width}
      viewportLabel={data.selection?.viewportLabel}
    />
  )
}

/** A page's favicon, falling back to the icon for its viewport size. */
function PageGlyph({
  faviconUrl,
  width,
  viewportLabel,
}: {
  faviconUrl?: string | null
  width?: number
  viewportLabel?: string
}) {
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
  const Icon = viewportIcon(viewportLabel ?? '', width)
  return <Icon size={11} className="shrink-0" />
}
