/**
 * TypeScript types for the JSON Canvas specification v1.0
 * https://jsoncanvas.org/
 * https://github.com/obsidianmd/jsoncanvas/blob/main/spec/1.0.md
 */

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
  linked?: boolean
  label?: string
  source?: string
  groupId?: string
  parentGroupId?: string
  metadata?: Record<string, unknown>
}

export interface JsonCanvasFileNode extends JsonCanvasNodeBase {
  type: 'file'
  file: string
  subpath?: string
  // App-specific extensions (other tools ignore per spec extensibility model)
  objectFit?: 'contain' | 'cover' | 'fill'
  presetIndex?: number
  metadata?: Record<string, unknown>
}

export interface JsonCanvasGroupNode extends JsonCanvasNodeBase {
  type: 'group'
  label?: string
  background?: string
  backgroundStyle?: 'cover' | 'ratio' | 'repeat'
  // App-specific extensions
  layoutMode?: string
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
}

/**
 * Shape node (Specular extension). Other JSON Canvas tools ignore
 * unknown `type` values per the spec's extensibility model.
 */
export interface JsonCanvasShapeNode extends JsonCanvasNodeBase {
  type: 'shape'
  shapeKind: 'rectangle' | 'ellipse' | 'diamond'
  text?: string
  strokeWidth?: number
  theme?: string
  label?: string
  parentGroupId?: string
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

import type { AnnotationDrawingStroke, EdgeSide, EdgeEnd } from './types'
export type { EdgeSide, EdgeEnd }

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
}
