import { useEffect, useRef, useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { diagramThemeVars } from '../../shared/mermaid/diagram-theme'
import { svgElementFromString } from '../../shared/mermaid/mermaid-svg-dom'
import { useMermaidRender } from '../../shared/mermaid/useMermaidRender'
import { getFileApi } from './filePathToSrc'

const LOAD_ERROR = 'This diagram’s file could not be read.'

/** A `.mmd` file rendered as a diagram, scaled to fit the card. Display-only:
 *  the file on disk is the source and a change there re-renders. */
export function MermaidInlineRenderer({
  entity,
  isDark,
}: {
  entity: CanvasSceneFileEntity
  isDark: boolean
}) {
  const { source, loadError } = useDiagramFile(entity.file, entity.fileReloadVersion)
  const render = useMermaidRender(source)
  const muted = isDark ? '#a8a29e' : '#78716c'

  if (loadError || (render && !render.ok)) {
    return (
      <div style={{ ...messageStyle, color: muted }}>
        <span>{loadError ?? 'This diagram could not be rendered.'}</span>
        {render && !render.ok ? <code style={{ fontSize: 11 }}>{render.error}</code> : null}
      </div>
    )
  }
  if (!render) return null
  return <DiagramSvg svg={render.svg} isDark={isDark} />
}

function DiagramSvg({ svg, isDark }: { svg: string; isDark: boolean }) {
  const hostRef = useRef<HTMLDivElement | null>(null)
  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const el = svgElementFromString(svg)
    host.replaceChildren(...(el ? [el] : []))
    if (el) {
      el.style.width = '100%'
      el.style.height = '100%'
    }
  }, [svg])
  return (
    <div
      ref={hostRef}
      style={{
        ...diagramThemeVars(isDark),
        width: '100%',
        height: '100%',
        // Defer selection and pointer handling to the canvas, like an image.
        pointerEvents: 'none',
        userSelect: 'none',
        WebkitUserSelect: 'none',
      }}
    />
  )
}

const messageStyle: React.CSSProperties = {
  width: '100%',
  height: '100%',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 6,
  padding: 16,
  fontSize: 12,
  fontFamily: 'system-ui, sans-serif',
  textAlign: 'center',
  wordBreak: 'break-word',
}

/** The file's text, re-read whenever the disk watcher bumps the version. */
function useDiagramFile(file: string, reloadVersion: number | undefined) {
  const [source, setSource] = useState<string | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    getFileApi()
      .readNoteFile(file)
      .then((text) => {
        if (cancelled) return
        setLoadError(text === null ? LOAD_ERROR : null)
        setSource(text)
      })
      .catch(() => {
        if (!cancelled) setLoadError(LOAD_ERROR)
      })
    return () => {
      cancelled = true
    }
  }, [file, reloadVersion])
  return { source, loadError }
}
