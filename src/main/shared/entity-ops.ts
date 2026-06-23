import type {
  ApplyDirectiveResult,
  BatchLayoutMode,
  LayoutDirective,
} from '../../shared/types'
import {
  CUSTOM_SHELL_INSETS,
  LAPTOP_PRESET_INDEX,
  VIEWPORT_PRESETS,
  defaultOrientationForDevice,
  deviceForPresetIndex,
  shellInsetsForDevice,
  sizeForOrientation,
} from '../../shared/device-catalog'
import { callApp } from './app-client'

// ---------------------------------------------------------------------------
// Upsert entities
// ---------------------------------------------------------------------------

export interface UpsertOptions {
  /** Legacy: simple layout flag for auto-placed creates (no canvasX/Y). */
  layout?: BatchLayoutMode
  gap?: number
  /**
   * Declarative layout directive. When present, computed positions override
   * any per-item canvasX/Y for ALL items, including items with an `id` (which
   * get re-laid-out). Mixed creates + re-layouts in one call are supported.
   */
  directive?: LayoutDirective
}

interface ItemFootprint {
  /** Outer width (visible footprint, including device-shell bezels). */
  width: number
  /** Outer height (visible footprint, including device-shell bezels). */
  height: number
  /** Distance from outer top-left to entity data origin (canvasX). */
  insetX: number
  /** Distance from outer top-left to entity data origin (canvasY). */
  insetY: number
}

function sizeForItem(item: Record<string, unknown>): ItemFootprint {
  if (item.kind === 'page') {
    const presetIndex = (item.presetIndex as number | undefined) ?? LAPTOP_PRESET_INDEX
    const preset = VIEWPORT_PRESETS[presetIndex]
    const w = preset?.width ?? 1280
    const h = preset?.height ?? 800
    const device = deviceForPresetIndex(presetIndex)
    const orientation =
      (item.orientation as 'portrait' | 'landscape' | undefined) ??
      defaultOrientationForDevice(device)
    const inner = sizeForOrientation(w, h, orientation)
    const showPage = item.showDeviceFrame !== false
    const insets = showPage
      ? (device ? shellInsetsForDevice(device.id, orientation) : CUSTOM_SHELL_INSETS)
      : { top: 0, right: 0, bottom: 0, left: 0 }
    // Footprint includes device-shell bezels only. The hover-only chrome
    // action header is reserved separately via occupied-region inflation so
    // it doesn't widen user-facing layout gaps.
    return {
      width: inner.width + insets.left + insets.right,
      height: inner.height + insets.top + insets.bottom,
      insetX: insets.left,
      insetY: insets.top,
    }
  }
  return {
    width: Number(item.width) || 200,
    height: Number(item.height) || 200,
    insetX: 0,
    insetY: 0,
  }
}

export async function upsertEntities(
  items: Array<Record<string, unknown>>,
  options?: UpsertOptions,
): Promise<{ created: string[]; updated: string[] }> {
  const results: { created: string[]; updated: string[] } = { created: [], updated: [] }

  // Apply layout directive first, if present. Computes positions for all
  // items (creates and re-layouts), overrides per-item canvasX/Y, and
  // back-fills `kind` for items with `id` so the bucketer routes correctly.
  if (options?.directive) {
    const warnings: string[] = []
    const directiveItems = items.map((item) => {
      if (item.id) {
        const out: { id: string; width?: number; height?: number } = { id: item.id as string }
        if (item.width !== undefined) out.width = Number(item.width)
        if (item.height !== undefined) out.height = Number(item.height)
        return out
      }
      const footprint = sizeForItem(item)
      return {
        width: footprint.width,
        height: footprint.height,
        insetX: footprint.insetX,
        insetY: footprint.insetY,
      }
    })
    const result = await callApp<ApplyDirectiveResult>('/layout/apply-directive', {
      method: 'POST',
      body: JSON.stringify({ layout: options.directive, items: directiveItems }),
    })
    for (let i = 0; i < items.length; i++) {
      if (items[i].canvasX !== undefined || items[i].canvasY !== undefined) {
        warnings.push(`item[${i}]: canvasX/canvasY ignored under layout directive`)
      }
      items[i].canvasX = result.positions[i].canvasX
      items[i].canvasY = result.positions[i].canvasY
      // Back-fill kind from runtime so bucketing routes the right way for
      // items where the agent only passed an id.
      if (items[i].id && !items[i].kind && result.kinds[i]) {
        items[i].kind = result.kinds[i]
      }
    }
    if (warnings.length > 0) {
      // eslint-disable-next-line no-console
      console.warn('upsert layout directive:', warnings.join('; '))
    }
  }

  // Batch-place creates (items without an `id`) that lack explicit positions.
  // Footprints are sized per item; the apply path resolves kind and the
  // long-text→note route, so no per-kind bucketing happens here anymore.
  const needsPlacement = items.filter(
    (item) => !item.id && (item.canvasX === undefined || item.canvasY === undefined),
  )
  if (needsPlacement.length > 0) {
    const footprints = needsPlacement.map(sizeForItem)
    const placement = await callApp<{ positions: Array<{ canvasX: number; canvasY: number }> }>(
      '/layout/batch-placement',
      {
        method: 'POST',
        body: JSON.stringify({
          items: footprints,
          layout: options?.layout,
          gap: options?.gap,
          anchor: 'selection_or_empty_region',
        }),
      },
    )
    for (let i = 0; i < needsPlacement.length; i++) {
      needsPlacement[i].canvasX = placement.positions[i].canvasX
      needsPlacement[i].canvasY = placement.positions[i].canvasY
    }
  }

  // One declarative patch, one apply path, one Y.Doc transaction. The registry
  // dispatches each item to its kind handler.
  const applied = await callApp<{ created: string[]; updated: string[] }>('/canvas/apply', {
    method: 'POST',
    body: JSON.stringify({ entities: items }),
  })
  results.created.push(...applied.created)
  results.updated.push(...applied.updated)
  return results
}

// ---------------------------------------------------------------------------
// Annotations — slim list
// ---------------------------------------------------------------------------

export async function getAnnotationsSlim(args: {
  status?: string
  url?: string
  page_id?: string
}): Promise<{ annotations: Record<string, unknown>[] }> {
  const params = new URLSearchParams()
  if (typeof args.status === 'string' && args.status) {
    params.set('status', args.status)
  }
  if (typeof args.url === 'string' && args.url.trim().length > 0) {
    params.set('url', args.url.trim())
  }
  if (typeof args.page_id === 'string' && args.page_id.trim().length > 0) {
    params.set('page_id', args.page_id.trim())
  }
  const query = params.toString()
  const result = await callApp<{ annotations: Record<string, unknown>[] }>(
    `/annotations${query ? `?${query}` : ''}`,
  )

  const stubs = result.annotations.map((ann) => {
    const metadata = ann.metadata as Record<string, unknown> | undefined
    const regionElements = metadata?.regionElements as
      | Array<{ pageId: string; pageName: string; elements: Array<Record<string, unknown>> }>
      | undefined
    const regionComponents = metadata?.regionComponents as
      | Array<{ pageId: string; components: Array<Record<string, unknown>> }>
      | undefined
    const inspectContext = metadata?.inspectContext as Record<string, unknown> | undefined

    // Build slim metadata — keep only pageName and pageUrl.
    const slimMeta: Record<string, unknown> = {}
    if (metadata?.pageName) slimMeta.pageName = metadata.pageName
    if (metadata?.pageUrl) slimMeta.pageUrl = metadata.pageUrl

    // Add unified summary with type discriminator. ADR 0006 retired
    // `Annotation.kind` — the anchor type is now the source of truth.
    const anchorType = (ann.anchor as { type?: string } | undefined)?.type
    if (anchorType === 'region') {
      const elementCount = regionElements?.reduce((n, g) => n + g.elements.length, 0) ?? 0
      const componentCount = regionComponents?.reduce(
        (n, g) => n + (g.components?.length ?? 0),
        0,
      ) ?? 0
      slimMeta.summary = {
        type: 'region',
        pageCount: regionElements?.length ?? 0,
        elementCount,
        hasScreenshot: !!metadata?.regionScreenshot,
        componentCount,
      }
    } else if (inspectContext) {
      slimMeta.summary = {
        type: 'element',
        tagName: inspectContext.tagName ?? null,
        name: inspectContext.name ?? null,
        cssClasses: inspectContext.cssClasses ?? [],
        textPreview: inspectContext.textPreview ?? null,
      }
    }

    return {
      id: ann.id,
      anchor: ann.anchor,
      author: ann.author,
      text: ann.text,
      status: ann.status,
      createdAt: ann.createdAt,
      replies: ann.replies,
      ...(Object.keys(slimMeta).length ? { metadata: slimMeta } : {}),
    }
  })

  return { annotations: stubs }
}

// ---------------------------------------------------------------------------
// Annotations — detail with screenshot
// ---------------------------------------------------------------------------

export async function getAnnotationDetail(args: {
  annotation_id: string
  include_screenshot?: boolean
}): Promise<{
  content: Array<
    | { type: 'text'; text: string }
    | { type: 'image'; data: string; mimeType: string }
  >
}> {
  const ann = await callApp<Record<string, unknown>>(
    `/annotations/${args.annotation_id}`,
  )
  const metadata = ann.metadata as Record<string, unknown> | undefined
  const screenshot = metadata?.regionScreenshot as string | undefined
  const regionElements = metadata?.regionElements as
    | Array<{ pageId: string; pageName: string; elements: Array<Record<string, unknown>> }>
    | undefined
  const regionComponents = metadata?.regionComponents as
    | Array<{ pageId: string; components: Array<Record<string, unknown>> }>
    | undefined
  const inspectContext = metadata?.inspectContext as Record<string, unknown> | undefined
  const includeScreenshot = args.include_screenshot !== false

  const content: Array<
    | { type: 'text'; text: string }
    | { type: 'image'; data: string; mimeType: string }
  > = []

  // Build detail metadata — strip heavy blobs from the text representation.
  const detailMeta: Record<string, unknown> = { ...(metadata ?? {}) }
  delete detailMeta.regionScreenshot
  delete detailMeta.regionElements
  delete detailMeta.regionComponents

  // Strip duplicated anchor fields from inspectContext.
  if (inspectContext) {
    const anchor = ann.anchor as Record<string, unknown> | undefined
    const cleaned = { ...inspectContext }
    // Remove internal-only fields.
    delete cleaned.id
    delete cleaned.nodeId
    delete cleaned.timestamp
    // Remove fields already present in anchor.
    if (anchor?.type === 'element') {
      if (anchor.elementPath) delete cleaned.elementPath
      if (anchor.boundingBox) delete cleaned.boundingBox
    }
    detailMeta.inspectContext = cleaned
  }

  // Include regionComponents only when non-empty.
  const hasComponents = regionComponents?.some((g) => g.components?.length > 0) ?? false
  if (hasComponents) {
    detailMeta.regionComponents = regionComponents
  }

  const annForText = { ...ann, metadata: detailMeta }
  content.push({ type: 'text' as const, text: JSON.stringify(annForText, null, 2) })

  // Format region elements as readable text.
  if (regionElements?.length) {
    let summary = 'Elements in region:'
    for (const group of regionElements) {
      summary += `\n  Page "${group.pageName}":`
      for (const el of group.elements) {
        const tag = el.tagName ?? '?'
        const classes = Array.isArray(el.cssClasses)
          ? (el.cssClasses as string[]).join('.')
          : ''
        const text = el.textPreview ?? ''
        summary += `\n    - <${tag}>${classes ? '.' + classes : ''} "${text}"`
      }
    }
    content.push({ type: 'text' as const, text: summary })
  }

  // Add screenshot as visible image content block.
  if (includeScreenshot && screenshot) {
    content.push({
      type: 'image' as const,
      data: screenshot,
      mimeType: 'image/png',
    })
  }

  return { content }
}
