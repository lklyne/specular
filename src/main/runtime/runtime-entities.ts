import type { WebContentsView } from 'electron'
import type {
  ComponentTreeNode,
  InspectNodeDetail,
  PageColorScheme,
  WorkspacePageSource,
} from '../../shared/types'
import type { DeviceOrientation } from '../../shared/device-catalog'

export interface Page {
  id: string
  name?: string
  title?: string
  url: string
  faviconUrl?: string | null
  pageView: WebContentsView
  devtoolsHostView?: WebContentsView
  devtoolsHostAttached?: boolean
  presetIndex: number
  canvasX: number
  /**
   * Top-left of the page's snap rect in canvas coords. With a device frame
   * this is the bezel top; without, it is the body top.
   */
  canvasY: number
  syncId: string | null
  source: WorkspacePageSource
  parentGroupId?: string
  groupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: PageColorScheme
  componentTree?: ComponentTreeNode[]
  inspectDetailsByNodeId?: Record<string, InspectNodeDetail>
  syncState: {
    suppressNavigationBroadcastUntil: number
    suppressNextScrollBroadcastUntil: number
  }
  peekWidth?: number
  peekHeight?: number
  /** Page's absolute scroll offset in raw CSS pixels, broadcast from the
   *  page preload (ephemeral view state — never persisted, never in the
   *  Y.Doc). Absent until the first offset broadcast arrives. */
  scrollX?: number
  scrollY?: number
  /** scrollHeight of the same scroll container the offset comes from, in CSS
   *  px. Lets main map a page anchor's `offsetY` fraction to a document
   *  position for scroll-to-comment (ADR 0029). Ephemeral;
   *  absent until the first offset broadcast arrives. */
  scrollHeight?: number
  /** True between Electron's did-start-loading/did-stop-loading events.
   * Document-bound items keep their previous visibility during this interval
   * so a provisional route does not strip the canvas before it settles. */
  isLoading?: boolean
  /** Live document positions of the DOM selectors anchored items reference,
   *  keyed by selector (ADR 0030 element attachment). The page's reflow tracker
   *  broadcasts these on real reflow events; scene builders read them as a
   *  render-time correction. Ephemeral — never persisted, never in the Y.Doc;
   *  a selector is absent until its first resolution and keeps its last-known
   *  position thereafter. */
  elementPositions?: Map<
    string,
    { docX: number; docY: number; viewportPositioned?: boolean }
  >
  lastPageBoundsKey?: string
  lastDevtoolsHostBoundsKey?: string
  /** Last colorScheme applied via CDP (see page-color-scheme.ts). Undefined
   *  means either "no override applied yet" or "no override needed" —
   *  both collapse to the same no-op when colorScheme is also absent. */
  lastColorSchemeKey?: PageColorScheme
  lastSafeAreaCssKey?: string
  lastSafeAreaCssId?: string
  crashedAt?: number
  crashReason?: Electron.RenderProcessGoneDetails['reason']
  /**
   * Bumped on every `did-navigate` / `dom-ready` of the page's webContents.
   * Not persisted — HMR partial updates don't navigate, so this can't be an
   * authoritative "has the DOM changed" signal, only a warn-only staleness
   * heuristic for ref-based agent mutations (see D8, issue #318).
   */
  navGeneration: number
  /**
   * navGeneration as of the last agent snapshot, recorded via
   * POST /pages/:id/snapshot-seen. Lives here (not in the CLI process)
   * because every `specular` CLI invocation is a fresh short-lived process —
   * only the main app outlives the snapshot→mutate loop the D8 comparison
   * spans. Same ephemeral/not-persisted semantics as navGeneration.
   * Per-page, not per-client: two agents driving the same page share the
   * baseline — acceptable for a warn-only heuristic.
   */
  lastAgentSnapshotGeneration?: number
}

// ---------------------------------------------------------------------------
// Custom size metadata (canvas sizing — renamed from "responsive")
// ---------------------------------------------------------------------------

type PageCustomSizeMetadata = {
  pageSizeMode?: 'custom' | 'responsive' // accept legacy 'responsive' on read
  customSize?: { width?: unknown; height?: unknown }
  responsiveSize?: { width?: unknown; height?: unknown } // legacy field
}

export function pageOverridesFromMetadata(
  metadata: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  if (!metadata) return undefined
  const candidate = metadata.overrides
  if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) {
    return undefined
  }
  return candidate as Record<string, unknown>
}

export function pageCustomSizeFromMetadata(
  metadata: Record<string, unknown> | undefined,
): { width: number; height: number } | null {
  if (!metadata) return null
  const candidate = metadata as PageCustomSizeMetadata
  // Accept both new 'custom' and legacy 'responsive'
  if (candidate.pageSizeMode !== 'custom' && candidate.pageSizeMode !== 'responsive') return null
  // Try new field first, fall back to legacy
  const sizeObj = candidate.customSize ?? candidate.responsiveSize
  const width = sizeObj?.width
  const height = sizeObj?.height
  if (typeof width !== 'number' || typeof height !== 'number') return null
  if (!Number.isFinite(width) || !Number.isFinite(height)) return null
  return { width, height }
}

export function pageUsesCustomSize(
  metadata: Record<string, unknown> | undefined,
): boolean {
  return pageCustomSizeFromMetadata(metadata) !== null
}

export function setCustomPageSizeMetadata(
  metadata: Record<string, unknown> | undefined,
  size: { width: number; height: number },
): Record<string, unknown> {
  const next = { ...(metadata ?? {}) }
  next.pageSizeMode = 'custom'
  next.customSize = { width: size.width, height: size.height }
  // Clean up legacy fields
  delete next.responsiveSize
  return next
}

export function clearCustomPageSizeMetadata(
  metadata: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  if (!metadata) return undefined
  const next = { ...metadata }
  delete next.pageSizeMode
  delete next.customSize
  delete next.responsiveSize
  return Object.keys(next).length ? next : undefined
}

// ---------------------------------------------------------------------------
// Legacy browser-size metadata. Browser mode no longer exists; these helpers
// remain so old files containing `browserSizeMode` can be read or cleaned up.
// ---------------------------------------------------------------------------

export type BrowserSizeMode = 'fill' | 'device'

export function pageBrowserSizeModeFromMetadata(
  metadata: Record<string, unknown> | undefined,
): BrowserSizeMode {
  if (!metadata) return 'device'
  const mode = metadata.browserSizeMode
  return mode === 'fill' ? 'fill' : 'device'
}

export function setPageBrowserSizeMode(
  metadata: Record<string, unknown> | undefined,
  mode: BrowserSizeMode,
): Record<string, unknown> {
  return {
    ...(metadata ?? {}),
    browserSizeMode: mode,
  }
}

// ---------------------------------------------------------------------------
// Device page metadata (device shell presentation)
// ---------------------------------------------------------------------------

export function deviceIdFromMetadata(
  metadata: Record<string, unknown> | undefined,
): string | null {
  if (!metadata) return null
  const id = metadata.deviceId
  return typeof id === 'string' ? id : null
}

export function setDeviceIdMetadata(
  metadata: Record<string, unknown> | undefined,
  deviceId: string | null,
): Record<string, unknown> {
  const next = { ...(metadata ?? {}) }
  if (deviceId === null) {
    delete next.deviceId
  } else {
    next.deviceId = deviceId
  }
  return next
}

export function deviceOrientationFromMetadata(
  metadata: Record<string, unknown> | undefined,
): DeviceOrientation {
  if (!metadata) return 'portrait'
  const o = metadata.deviceOrientation
  return o === 'landscape' ? 'landscape' : 'portrait'
}

export function setDeviceOrientationMetadata(
  metadata: Record<string, unknown> | undefined,
  orientation: DeviceOrientation,
): Record<string, unknown> {
  return {
    ...(metadata ?? {}),
    deviceOrientation: orientation,
  }
}

export function showDeviceFrameFromMetadata(
  metadata: Record<string, unknown> | undefined,
): boolean {
  if (!metadata) return false
  return metadata.showDeviceFrame === true
}

export function setShowDeviceFrameMetadata(
  metadata: Record<string, unknown> | undefined,
  show: boolean,
): Record<string, unknown> {
  return {
    ...(metadata ?? {}),
    showDeviceFrame: show,
  }
}

// ---------------------------------------------------------------------------
// SVG device shell rendering mode (A/B toggle)
// ---------------------------------------------------------------------------

export function useSvgDeviceShellFromMetadata(
  metadata: Record<string, unknown> | undefined,
): boolean {
  if (!metadata) return false
  return metadata.useSvgDeviceShell === true
}

export function setUseSvgDeviceShellMetadata(
  metadata: Record<string, unknown> | undefined,
  use: boolean,
): Record<string, unknown> {
  return {
    ...(metadata ?? {}),
    useSvgDeviceShell: use,
  }
}
