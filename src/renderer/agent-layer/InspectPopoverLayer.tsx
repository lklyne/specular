import { useLayoutEffect, useMemo, useRef, useState } from 'react'
import type {
  CanvasScenePageEntity,
  InspectNodeDetail,
  LayoutUpdateData,
} from '../../shared/types'
import {
  inspectTargetScreenRect,
  placeInspectPopover,
  type InspectPopoverSize,
} from '../../shared/inspect-popover-position'

const FALLBACK_SIZE: InspectPopoverSize = { width: 260, height: 84 }

function styleValue(detail: InspectNodeDetail, key: string): string | null {
  const prefix = `${key}=`
  const line = detail.computedStyles.find((entry) => entry.startsWith(prefix))
  return line ? line.slice(prefix.length) : null
}

function attrValue(detail: InspectNodeDetail, name: string): string | null {
  return detail.attributes.find((attr) => attr.name === name)?.value ?? null
}

function selectorRemainder(detail: InspectNodeDetail): string {
  const id = attrValue(detail, 'id')
  const idPart = id ? `#${id}` : ''
  const classPart = detail.cssClasses.slice(0, 3).map((cls) => `.${cls}`).join('')
  return `${idPart}${classPart}`
}

function firstFontFamily(fontFamily: string | null): string | null {
  if (!fontFamily) return null
  const first = fontFamily.split(',')[0]?.trim() ?? ''
  return first.replace(/^['"]|['"]$/g, '') || null
}

export function InspectPopoverLayer({
  layoutData,
}: {
  layoutData: LayoutUpdateData
}) {
  const detail = useMemo((): InspectNodeDetail | null => {
    const inspect = layoutData.inspect
    if (!inspect?.enabled) return null
    const nodeId = inspect.hoveredNodeId ?? inspect.selectedNodeId
    return nodeId ? inspect.detailById[nodeId] ?? null : null
  }, [layoutData.inspect])

  const page = useMemo((): CanvasScenePageEntity | null => {
    if (!detail) return null
    return layoutData.entities.find(
      (entity): entity is CanvasScenePageEntity =>
        entity.kind === 'page' && entity.id === detail.pageId,
    ) ?? null
  }, [detail, layoutData.entities])

  const [size, setSize] = useState<InspectPopoverSize>(FALLBACK_SIZE)
  const ref = useRef<HTMLDivElement | null>(null)
  useLayoutEffect(() => {
    const rect = ref.current?.getBoundingClientRect()
    if (!rect) return
    const next = {
      width: Math.ceil(rect.width),
      height: Math.ceil(rect.height),
    }
    if (next.width !== size.width || next.height !== size.height) {
      setSize(next)
    }
  }, [detail, size.height, size.width])

  if (!detail || !page) return null
  const screenTarget = inspectTargetScreenRect(detail, page)
  if (!screenTarget) return null
  const target = {
    ...screenTarget,
    top: screenTarget.top - layoutData.canvasOrigin.y,
    bottom: screenTarget.bottom - layoutData.canvasOrigin.y,
  }
  const position = placeInspectPopover(target, size, {
    width: window.innerWidth,
    height: window.innerHeight,
  })

  const tagName = detail.tagName || 'element'
  const remainder = selectorRemainder(detail)
  const fontFamily = firstFontFamily(styleValue(detail, 'font-family'))
  const fontSize = styleValue(detail, 'font-size')
  const fontWeight = styleValue(detail, 'font-weight')
  const color = styleValue(detail, 'color')
  const background = styleValue(detail, 'background')

  return (
    <div
      ref={ref}
      className="pointer-events-none absolute z-[2147483647] max-w-[360px] rounded-md bg-slate-800/95 px-1.5 py-1.5 font-mono text-[11px] leading-tight text-white shadow-lg"
      style={{
        left: position.left,
        top: position.top,
        boxShadow: '0 2px 8px rgba(0,0,0,0.25)',
      }}
    >
      <div className="flex min-w-0 items-center gap-1.5">
        <span className="shrink-0 rounded bg-blue-500 px-1.5 py-0.5 text-white">
          {tagName}
        </span>
        {remainder ? (
          <span className="min-w-0 truncate opacity-90">{remainder}</span>
        ) : null}
      </div>
      {fontFamily ? (
        <div className="mt-1.5 flex min-w-0 items-baseline gap-1.5 whitespace-nowrap">
          <span
            className="min-w-0 truncate text-white"
            style={{
              fontFamily: styleValue(detail, 'font-family') ?? undefined,
              fontWeight: fontWeight ?? undefined,
            }}
          >
            {fontFamily}
          </span>
          <span className="shrink-0 opacity-70">
            · {[fontSize, fontWeight].filter(Boolean).join(' · ')}
          </span>
        </div>
      ) : null}
      <div className="mt-1.5 flex items-center gap-2.5 whitespace-nowrap opacity-85">
        {color ? <ColorSwatch label="text" value={color} /> : null}
        {background && background !== 'rgba(0, 0, 0, 0)' ? (
          <ColorSwatch label="bg" value={background} />
        ) : null}
      </div>
    </div>
  )
}

function ColorSwatch({ label, value }: { label: string; value: string }) {
  return (
    <span className="inline-flex items-center gap-1">
      <span
        className="inline-block h-2.5 w-2.5 shrink-0 rounded-[2px]"
        style={{
          background: value,
          boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.35)',
        }}
      />
      <span>{label} {value}</span>
    </span>
  )
}
