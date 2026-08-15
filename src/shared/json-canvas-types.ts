/**
 * TypeScript types for the JSON Canvas specification v1.0
 * https://jsoncanvas.org/
 * https://github.com/obsidianmd/jsoncanvas/blob/main/spec/1.0.md
 */

import type { ShapeKind } from './shapes'

// --- Colors ---

/** Preset color "1"–"6" (red, orange, yellow, green, cyan, purple) or hex "#RRGGBB" */
export type CanvasColor = '1' | '2' | '3' | '4' | '5' | '6' | (string & {})

// --- Nodes ---

export interface JsonCanvasNodeBase {
  id: string
  type: 'text' | 'link' | 'file' | 'group' | 'drawing' | 'shape'
  x: number
  y: number
  width: number
  height: number
  color?: CanvasColor
}

/**
 * Specular-only fields on a JSON Canvas node, namespaced so they don't
 * collide with other tools' extensions. See ADR 0004 and ADR 0013 §1/§2.
 */
export interface SpecularNodeExtensions {
  /** 'plain' = unbacked text; 'sticky' = colored card. Missing → 'sticky'. */
  textStyle?: 'plain' | 'sticky'
  /** Text auto-resize mode. Missing → plain defaults to 'auto', sticky to 'fixed'. */
  widthMode?: 'auto' | 'fixed'
  /**
   * Theme/role-aware neutral marker. When set, the resolved RGB depends on
   * the active theme and the entity's color role; the spec `color` field
   * carries `"1"` (red preset) only as a cross-tool fallback. See ADR 0013 §1.
   */
  colorRole?: 'neutral'
  /**
   * Per-entity text size in pixels. Used by text (plain + sticky) and shape
   * (inner label). Missing → renderer defaults to 14 ("Small"). ADR 0013 §2.
   */
  textSize?: number
  /** Shape fill visibility. Missing → 'solid'. */
  fillStyle?: 'solid' | 'none'
  /** Shape inner-label alignment. Missing → center/middle. */
  textAlign?: 'left' | 'center' | 'right'
  textVerticalAlign?: 'top' | 'middle' | 'bottom'
  /**
   * Hooks the node to a page entity and the URL that page showed when the
   * node was placed. See shared/page-anchor.ts.
   */
  pageAnchor?: { pageId: string; pageUrl?: string }
  /**
   * Group membership by reference. JSON Canvas has no such field — the spec
   * derives membership spatially, from a node's rectangle sitting inside a
   * group's — so this is a Specular extension, namespaced per ADR 0004 §2
   * because `text` and `file` are spec node types. (`shape` and `drawing`
   * are Specular node types outright, so they carry it at the top level.)
   * A tool that ignores `specular` still lands close to right: grouped items
   * sit inside their group's bounds, so spatial reading agrees.
   */
  parentGroupId?: string
  /** Sidebar/outline name, distinct from the node's rendered content. */
  label?: string
}

export interface JsonCanvasTextNode extends JsonCanvasNodeBase {
  type: 'text'
  text: string
  specular?: SpecularNodeExtensions
}

export interface JsonCanvasLinkNode extends JsonCanvasNodeBase {
  type: 'link'
  url: string
  // App-specific extensions (other tools ignore per spec extensibility model)
  presetIndex?: number
  syncId?: string | null
  label?: string
  source?: string
  groupId?: string
  parentGroupId?: string
  metadata?: Record<string, unknown>
  /** Optional — absent means the page follows the system color scheme. */
  colorScheme?: 'light' | 'dark'
}

export interface JsonCanvasFileNode extends JsonCanvasNodeBase {
  type: 'file'
  file: string
  subpath?: string
  // App-specific extensions (other tools ignore per spec extensibility model)
  objectFit?: 'contain' | 'cover' | 'fill'
  presetIndex?: number
  metadata?: Record<string, unknown>
  specular?: SpecularNodeExtensions
}

export interface JsonCanvasGroupNode extends JsonCanvasNodeBase {
  type: 'group'
  label?: string
  background?: string
  backgroundStyle?: 'cover' | 'ratio' | 'repeat'
  // App-specific extensions
  layoutMode?: string
  layoutGap?: number
  pageIds?: string[]
  entityIds?: string[]
  parentGroupId?: string
  managedLayout?: boolean
  groupColor?: string
  sourceTaskId?: string
  groupMetadata?: Record<string, unknown>
}

/**
 * Drawing node (Specular extension). Other JSON Canvas tools ignore
 * unknown `type` values per the spec's extensibility model.
 */
export interface JsonCanvasDrawingNode extends JsonCanvasNodeBase {
  type: 'drawing'
  strokes: AnnotationDrawingStroke[]
  label?: string
  parentGroupId?: string
  /** Hooks the drawing to a page entity + URL. See shared/page-anchor.ts. */
  pageAnchor?: { pageId: string; pageUrl?: string }
}

/**
 * Shape node (Specular extension). Other JSON Canvas tools ignore
 * unknown `type` values per the spec's extensibility model.
 */
export interface JsonCanvasShapeNode extends JsonCanvasNodeBase {
  type: 'shape'
  shapeKind: ShapeKind
  text?: string
  strokeWidth?: number
  borderStyle?: 'solid' | 'dashed' | 'none'
  borderColor?: string
  theme?: string
  label?: string
  parentGroupId?: string
  /** Hooks the shape to a page entity + URL; `scrollX/scrollY` record the
   *  page scroll at placement so the shape scroll-follows the document.
   *  See shared/page-anchor.ts. */
  pageAnchor?: { pageId: string; pageUrl?: string; scrollX?: number; scrollY?: number }
  specular?: SpecularNodeExtensions
}

export type JsonCanvasNode =
  | JsonCanvasTextNode
  | JsonCanvasLinkNode
  | JsonCanvasFileNode
  | JsonCanvasGroupNode
  | JsonCanvasDrawingNode
  | JsonCanvasShapeNode

// --- Edges ---

import type {
  AnnotationDrawingStroke,
  EdgeSide,
  EdgeEnd,
  EdgeLineStyle,
  EdgeRouting,
  EdgeSplitAxis,
} from './types'
export type { EdgeSide, EdgeEnd, EdgeLineStyle, EdgeRouting, EdgeSplitAxis }

export interface JsonCanvasEdge {
  id: string
  fromNode: string
  toNode: string
  fromSide?: EdgeSide
  toSide?: EdgeSide
  fromEnd?: EdgeEnd
  toEnd?: EdgeEnd
  color?: CanvasColor
  label?: string
  // App-specific extensions
  strokeWidth?: number
  lineStyle?: EdgeLineStyle
  routing?: EdgeRouting
  elbowSplit?: number
  elbowSplitAxis?: EdgeSplitAxis
  edgeKind?: string
  edgeMetadata?: Record<string, unknown>
}

/**
 * A free-ended edge — one or both ends is a canvas-space point rather than a
 * node. JSON Canvas requires `fromNode`/`toNode` on `edges[]`, so there is no
 * spec-legal way to represent a dangling edge there. These live in
 * `specular.freeEdges` instead — see ADR 0034. Mirrors `JsonCanvasEdge` field
 * for field except `fromNode`/`toNode` are optional and joined by
 * `fromPoint`/`toPoint`.
 */
export interface JsonCanvasFreeEdge {
  id: string
  fromNode?: string
  toNode?: string
  fromPoint?: { x: number; y: number }
  toPoint?: { x: number; y: number }
  fromSide?: EdgeSide
  toSide?: EdgeSide
  fromEnd?: EdgeEnd
  toEnd?: EdgeEnd
  color?: CanvasColor
  label?: string
  strokeWidth?: number
  lineStyle?: EdgeLineStyle
  routing?: EdgeRouting
  elbowSplit?: number
  elbowSplitAxis?: EdgeSplitAxis
  edgeKind?: string
  edgeMetadata?: Record<string, unknown>
}

// --- Document ---

export interface JsonCanvasDocument {
  nodes: JsonCanvasNode[]
  edges: JsonCanvasEdge[]
  // App-specific extensions (other tools ignore per spec)
  specular?: JsonCanvasSpecularExtensions
  annotations?: unknown[]
  appState?: JsonCanvasAppState
}

export interface JsonCanvasSpecularExtensions {
  /**
   * Back-to-front stack order for all Specular canvas participants. JSON
   * Canvas keeps nodes[] and edges[] separate, so this preserves edge
   * interleaving without changing the spec arrays.
   */
  entityOrder?: string[]
  /**
   * Free-ended edges — one or both ends unbound from an entity. Kept outside
   * the spec's `edges[]` so a strict JSON Canvas reader sees a fully valid
   * file and simply doesn't see them. See ADR 0034.
   */
  freeEdges?: JsonCanvasFreeEdge[]
}

export interface JsonCanvasAppState {
  zoom: number
  pan: { x: number; y: number }
  selectedEntityIds?: string[]
  leftSidebarOpen?: boolean
  devtoolsOpen?: boolean
  devtoolsPanelTab?: string
  devtoolsWidth?: number
  browserTabMode?: string
  /** Which canvas answered this read, and what else is open. Present on the
   *  live `GET /canvas` read only — a `.canvas` file describes one tab and
   *  says nothing about its siblings. */
  activeTab?: JsonCanvasTabIdentity
  tabs?: JsonCanvasTabIdentity[]
}

export interface JsonCanvasTabIdentity {
  id: string
  name: string
  entityCount?: number
}
