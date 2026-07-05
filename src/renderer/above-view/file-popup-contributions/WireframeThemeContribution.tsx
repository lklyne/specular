// ADR 0008 §7 — wireframe theme swap. Writes JSON via `api.writeNoteFile`
// and pings the body via `wireframe-file-changed` so it picks up the disk
// change without waiting for a layout broadcast.

import { useEffect, useRef, useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import type { CanvasBgElectronAPI } from '../../../shared/electron-api/canvas-bg'
import { WIREFRAME_THEME_OPTIONS } from '../../canvas-bg/wireframe/WireframeRenderer'
import type { WireframeThemeName } from '../../canvas-bg/wireframe/wireframe-types'
import { filePathToSrc } from '../../canvas-bg/entity-renderers/filePathToSrc'
import { CanvasItemPopup } from '../CanvasItemPopup'

export function WireframeThemeContribution({
  api,
  isDark,
  entity,
}: {
  api: Pick<CanvasBgElectronAPI, 'writeNoteFile'>
  isDark: boolean
  entity: CanvasSceneFileEntity
}) {
  const [theme, setTheme] = useState<WireframeThemeName>('light')
  const wireframeRef = useRef<Record<string, unknown> | null>(null)

  useEffect(() => {
    let cancelled = false
    wireframeRef.current = null
    const read = async () => {
      try {
        const res = await fetch(filePathToSrc(entity.file))
        const wf = JSON.parse(await res.text()) as Record<string, unknown>
        if (cancelled) return
        wireframeRef.current = wf
        setTheme((wf.theme as WireframeThemeName | undefined) ?? 'light')
      } catch {
        /* ignore */
      }
    }
    void read()
    return () => {
      cancelled = true
    }
  }, [entity.file])

  const handleChange = async (next: WireframeThemeName) => {
    try {
      const wf =
        wireframeRef.current ??
        (JSON.parse(await (await fetch(filePathToSrc(entity.file))).text()) as Record<
          string,
          unknown
        >)
      wireframeRef.current = wf
      wf.theme = next
      await api.writeNoteFile(entity.file, JSON.stringify(wf, null, 2))
      setTheme(next)
      // Body renderer owns its own file-content state; disk write alone
      // doesn't propagate. Ping it.
      window.dispatchEvent(
        new CustomEvent('wireframe-file-changed', { detail: { file: entity.file } }),
      )
    } catch {
      /* ignore */
    }
  }

  return (
    <CanvasItemPopup.Section>
      {WIREFRAME_THEME_OPTIONS.map((t) => (
        <button
          key={t.name}
          type="button"
          onClick={() => handleChange(t.name)}
          title={t.name}
          aria-label={`Set wireframe theme to ${t.name}`}
          aria-pressed={theme === t.name}
          className="flex items-center justify-center"
        >
          <span
            style={{
              width: 18,
              height: 18,
              borderRadius: '50%',
              border:
                t.name === theme
                  ? `2px solid ${isDark ? '#60a5fa' : '#3b82f6'}`
                  : `1.5px solid ${isDark ? '#3f3f46' : '#d4d4d8'}`,
              background: t.color,
              display: 'block',
            }}
          />
        </button>
      ))}
    </CanvasItemPopup.Section>
  )
}
