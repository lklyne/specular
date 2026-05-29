/**
 * JSON Canvas Serializer/Deserializer
 *
 * Converts between our internal WorkspaceSnapshot format and the
 * JSON Canvas specification v1.0 (https://jsoncanvas.org/).
 */

import type {
  Annotation,
  BrowserTabMode,
  DevtoolsPanelTab,
  PersistedCanvasEntity,
  PersistedDrawingEntity,
  PersistedFileEntity,
  PersistedPageEntity,
  PersistedGroupEntity,
  PersistedShapeEntity,
  PersistedTextEntity,
  WorkspaceEdge,
  WorkspaceGroup,
  WorkspaceSnapshot,
} from '../../shared/types'
import type {
  JsonCanvasDocument,
  JsonCanvasDrawingNode,
  JsonCanvasEdge,
  JsonCanvasFileNode,
  JsonCanvasGroupNode,
  JsonCanvasLinkNode,
  JsonCanvasNode,
  JsonCanvasShapeNode,
  JsonCanvasTextNode,
  JsonCanvasAppState,
} from '../../shared/json-canvas-types'
import { VIEWPORT_PRESETS } from '../../shared/constants'
import { NEUTRAL_STORAGE } from '../../shared/canvas-colors'
import { pageCustomSizeFromMetadata } from './runtime-entities'

// --- Serialize ---

export function serializeToJsonCanvas(
  snapshot: WorkspaceSnapshot,
  annotations?: Annotation[],
): JsonCanvasDocument {
  const nodes: JsonCanvasNode[] = []
  const edges: JsonCanvasEdge[] = []

  // Build ordered entity list
  const entities = { ...(snapshot.entities ?? {}) }
  const edgeIds = new Set((snapshot.edges ?? []).map((edge) => edge.id))
  const knownIds = new Set([...Object.keys(entities), ...edgeIds])
  const orderedIds = uniqueKnownIds(snapshot.entityOrder ?? Object.keys(entities), knownIds)

  // Also include page entities from legacy pages array
  for (const page of snapshot.pages) {
    if (page.id && !entities[page.id]) {
      const entity: PersistedPageEntity = {
        kind: 'page',
        id: page.id,
        name: page.name,
        url: page.url,
        presetIndex: page.presetIndex,
        canvasX: page.canvasX,
        canvasY: page.canvasY,
        linked: page.linked,
        source: page.source,
        parentGroupId: page.parentGroupId ?? page.groupId,
        groupId: page.parentGroupId ?? page.groupId,
        metadata: page.metadata,
      }
      entities[page.id] = entity
      if (!knownIds.has(page.id)) {
        knownIds.add(page.id)
        orderedIds.push(page.id)
      }
    }
  }
  for (const edgeId of edgeIds) {
    if (orderedIds.includes(edgeId)) continue
    orderedIds.push(edgeId)
  }

  // Convert entities to nodes (array order = z-order per spec)
  for (const id of orderedIds) {
    const entity = entities[id]
    if (!entity) continue

    if (entity.kind === 'page') {
      nodes.push(serializePageToLinkNode(entity))
    } else if (entity.kind === 'text') {
      nodes.push(serializeTextToTextNode(entity))
    } else if (entity.kind === 'file') {
      nodes.push(serializeFileToFileNode(entity))
    } else if (entity.kind === 'group') {
      nodes.push(serializeGroupEntityToGroupNode(entity))
    } else if (entity.kind === 'drawing') {
      nodes.push(serializeDrawingToDrawingNode(entity))
    } else if (entity.kind === 'shape') {
      nodes.push(serializeShapeToShapeNode(entity))
    }
  }

  // Convert edges
  if (snapshot.edges) {
    for (const edge of snapshot.edges) {
      edges.push(serializeEdge(edge))
    }
  }

  const doc: JsonCanvasDocument = { nodes, edges }
  if (orderedIds.length) {
    doc.specular = { entityOrder: orderedIds }
  }

  // Add annotations as extension
  if (annotations?.length) {
    doc.annotations = annotations
  }

  // Add app state as extension
  doc.appState = serializeAppState(snapshot)

  return doc
}

function serializePageToLinkNode(entity: PersistedPageEntity): JsonCanvasLinkNode {
  const preset = VIEWPORT_PRESETS[entity.presetIndex] ?? VIEWPORT_PRESETS[0]
  const customSize = pageCustomSizeFromMetadata(entity.metadata)
  return {
    id: entity.id,
    type: 'link',
    x: entity.canvasX,
    y: entity.canvasY,
    width: customSize?.width ?? preset?.width ?? 375,
    height: customSize?.height ?? preset?.height ?? 667,
    url: entity.url,
    // App-specific extensions
    presetIndex: entity.presetIndex,
    linked: entity.linked,
    label: entity.name,
    source: entity.source,
    groupId: entity.parentGroupId ?? entity.groupId,
    parentGroupId: entity.parentGroupId ?? entity.groupId,
    metadata: entity.metadata,
  }
}

function serializeTextToTextNode(entity: PersistedTextEntity): JsonCanvasTextNode {
  const isNeutral = entity.color === NEUTRAL_STORAGE
  const node: JsonCanvasTextNode = {
    id: entity.id,
    type: 'text',
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
    text: entity.text,
    color: isNeutral ? '1' : entity.color,
  }
  const specular = buildSpecularExtensions(
    entity.textStyle,
    entity.widthMode,
    isNeutral,
    entity.textSize,
  )
  if (specular) node.specular = specular
  return node
}

function buildSpecularExtensions(
  textStyle: PersistedTextEntity['textStyle'] | undefined,
  widthMode: PersistedTextEntity['widthMode'] | undefined,
  isNeutral: boolean,
  textSize: number | undefined,
): JsonCanvasTextNode['specular'] {
  if (
    textStyle === undefined &&
    widthMode === undefined &&
    !isNeutral &&
    textSize === undefined
  ) {
    return undefined
  }
  const ext: NonNullable<JsonCanvasTextNode['specular']> = {}
  if (textStyle !== undefined) ext.textStyle = textStyle
  if (widthMode !== undefined) ext.widthMode = widthMode
  if (isNeutral) ext.colorRole = 'neutral'
  if (textSize !== undefined) ext.textSize = textSize
  return ext
}

function serializeFileToFileNode(entity: PersistedFileEntity): JsonCanvasFileNode {
  return {
    id: entity.id,
    type: 'file',
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
    file: entity.file,
    subpath: entity.subpath,
    objectFit: entity.objectFit,
    presetIndex: entity.presetIndex,
    metadata: entity.metadata,
  }
}

function serializeShapeToShapeNode(entity: PersistedShapeEntity): JsonCanvasShapeNode {
  const isNeutral = entity.color === NEUTRAL_STORAGE
  const node: JsonCanvasShapeNode = {
    id: entity.id,
    type: 'shape',
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
    shapeKind: entity.shapeKind,
    text: entity.text,
    color: isNeutral ? '1' : entity.color,
    strokeWidth: entity.strokeWidth,
    theme: entity.theme,
    label: entity.label,
    parentGroupId: entity.parentGroupId,
  }
  if (isNeutral || entity.textSize !== undefined) {
    node.specular = {}
    if (isNeutral) node.specular.colorRole = 'neutral'
    if (entity.textSize !== undefined) node.specular.textSize = entity.textSize
  }
  return node
}

function serializeDrawingToDrawingNode(entity: PersistedDrawingEntity): JsonCanvasDrawingNode {
  return {
    id: entity.id,
    type: 'drawing',
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
    strokes: entity.strokes,
    label: entity.label,
    parentGroupId: entity.parentGroupId,
  }
}

function serializeGroupEntityToGroupNode(entity: PersistedGroupEntity): JsonCanvasGroupNode {
  return {
    id: entity.id,
    type: 'group',
    x: entity.canvasX,
    y: entity.canvasY,
    width: entity.width,
    height: entity.height,
    label: entity.label,
    color: entity.color,
    // App-specific extensions
    layoutMode: entity.layoutMode,
    parentGroupId: entity.parentGroupId,
    managedLayout: entity.managedLayout,
    groupColor: entity.color,
    sourceTaskId: entity.sourceTaskId,
    groupMetadata: entity.metadata,
  }
}

function serializeEdge(edge: WorkspaceEdge): JsonCanvasEdge {
  return {
    id: edge.id,
    fromNode: edge.fromEntityId,
    toNode: edge.toEntityId,
    fromSide: edge.fromSide,
    toSide: edge.toSide,
    fromEnd: edge.fromEnd,
    toEnd: edge.toEnd,
    color: edge.color,
    label: edge.label,
    // App-specific extensions
    edgeKind: edge.kind,
    edgeMetadata: edge.metadata,
  }
}

function serializeAppState(snapshot: WorkspaceSnapshot): JsonCanvasAppState {
  return {
    zoom: snapshot.zoom,
    pan: { ...snapshot.pan },
    selectedEntityIds: snapshot.selectedPageIds ?? [],
    leftSidebarOpen: snapshot.leftSidebarOpen,
    devtoolsOpen: snapshot.devtoolsOpen,
    devtoolsPanelTab: snapshot.devtoolsPanelTab,
    devtoolsWidth: snapshot.devtoolsWidth,
    browserTabMode: snapshot.browserTabMode,
  }
}

// --- Deserialize ---

export function deserializeFromJsonCanvas(doc: JsonCanvasDocument): {
  snapshot: WorkspaceSnapshot
  annotations: Annotation[]
} {
  const entities: Record<string, PersistedCanvasEntity> = {}
  const nodeOrder: string[] = []
  for (const node of doc.nodes) {
    if (node.type === 'link') {
      const entity = deserializeLinkNodeToPage(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    } else if (node.type === 'text') {
      const entity = deserializeTextNodeToText(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    } else if (node.type === 'file') {
      const entity = deserializeFileNodeToFile(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    } else if (node.type === 'group') {
      const entity = deserializeGroupNodeToGroup(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    } else if (node.type === 'drawing') {
      const entity = deserializeDrawingNodeToDrawing(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    } else if (node.type === 'shape') {
      const entity = deserializeShapeNodeToShape(node)
      entities[entity.id] = entity
      nodeOrder.push(entity.id)
    }
  }

  const edges: WorkspaceEdge[] = doc.edges.map(deserializeEdgeToWorkspaceEdge)
  const edgeOrder = edges.map((edge) => edge.id)
  const entityOrder = deserializeEntityOrder(doc.specular?.entityOrder, nodeOrder, edgeOrder)

  const appState = doc.appState ?? { zoom: 1, pan: { x: 0, y: 0 } }

  const snapshot: WorkspaceSnapshot = {
    zoom: appState.zoom ?? 1,
    pan: appState.pan ?? { x: 0, y: 0 },
    pages: [], // Legacy — populated from entities if needed
    entities,
    entityOrder,
    selectedPageIndex: null,
    selectedPageId: null,
    selectedPageIds: appState.selectedEntityIds ?? [],
    leftSidebarOpen: appState.leftSidebarOpen ?? true,
    devtoolsOpen: appState.devtoolsOpen ?? false,
    devtoolsPanelTab: (appState.devtoolsPanelTab as DevtoolsPanelTab) ?? 'elements',
    devtoolsWidth: appState.devtoolsWidth ?? 400,
    browserTabMode: (appState.browserTabMode as BrowserTabMode) ?? 'page',
    edges,
  }

  const annotations = (doc.annotations ?? []) as Annotation[]

  return { snapshot, annotations }
}

function uniqueKnownIds(ids: readonly string[], knownIds: ReadonlySet<string>): string[] {
  const seen = new Set<string>()
  const ordered: string[] = []
  for (const id of ids) {
    if (!knownIds.has(id) || seen.has(id)) continue
    seen.add(id)
    ordered.push(id)
  }
  return ordered
}

function deserializeEntityOrder(
  specularOrder: readonly string[] | undefined,
  nodeOrder: readonly string[],
  edgeOrder: readonly string[],
): string[] {
  const knownIds = new Set([...nodeOrder, ...edgeOrder])
  const ordered = uniqueKnownIds(specularOrder ?? [], knownIds)
  const seen = new Set(ordered)
  for (const id of [...nodeOrder, ...edgeOrder]) {
    if (seen.has(id)) continue
    seen.add(id)
    ordered.push(id)
  }
  return ordered
}

function deserializeLinkNodeToPage(node: JsonCanvasLinkNode): PersistedPageEntity {
  return {
    kind: 'page',
    id: node.id,
    name: node.label,
    url: node.url,
    presetIndex: node.presetIndex ?? 0,
    canvasX: node.x,
    canvasY: node.y,
    linked: node.linked ?? false,
    source: node.source as PersistedPageEntity['source'],
    groupId: node.groupId,
    metadata: node.metadata,
  }
}

function deserializeTextNodeToText(node: JsonCanvasTextNode): PersistedTextEntity {
  // Color is kept raw — a preset number, the 'neutral' sentinel, or a literal
  // hex — so the renderer can resolve the palette against the entity's surface.
  const color =
    node.specular?.colorRole === 'neutral' ? NEUTRAL_STORAGE : node.color ?? '3'
  const textStyle = node.specular?.textStyle ?? 'sticky'
  // Legacy canvases predate widthMode. Default to 'fixed' on read so saved
  // bounds are preserved — the 'auto' default only applies to brand-new
  // plain text created via the text tool. The runtime layer's
  // `defaultWidthMode()` handles the new-entity path.
  const widthMode = node.specular?.widthMode ?? 'fixed'
  return {
    kind: 'text',
    id: node.id,
    text: node.text,
    color,
    textStyle,
    widthMode,
    textSize: node.specular?.textSize,
    canvasX: node.x,
    canvasY: node.y,
    width: node.width,
    height: node.height,
  }
}

function deserializeFileNodeToFile(node: JsonCanvasFileNode): PersistedFileEntity {
  return {
    kind: 'file',
    id: node.id,
    file: node.file,
    subpath: node.subpath,
    canvasX: node.x,
    canvasY: node.y,
    width: node.width,
    height: node.height,
    objectFit: node.objectFit,
    presetIndex: node.presetIndex,
    metadata: node.metadata,
  }
}

function deserializeShapeNodeToShape(node: JsonCanvasShapeNode): PersistedShapeEntity {
  const color =
    node.specular?.colorRole === 'neutral' ? NEUTRAL_STORAGE : node.color
  return {
    kind: 'shape',
    id: node.id,
    shapeKind: node.shapeKind,
    text: node.text ?? '',
    color,
    strokeWidth: node.strokeWidth,
    textSize: node.specular?.textSize,
    theme: node.theme,
    canvasX: node.x,
    canvasY: node.y,
    width: node.width,
    height: node.height,
    label: node.label,
    parentGroupId: node.parentGroupId,
  }
}

function deserializeDrawingNodeToDrawing(node: JsonCanvasDrawingNode): PersistedDrawingEntity {
  return {
    kind: 'drawing',
    id: node.id,
    canvasX: node.x,
    canvasY: node.y,
    width: node.width,
    height: node.height,
    strokes: node.strokes,
    label: node.label,
    parentGroupId: node.parentGroupId,
  }
}

function deserializeGroupNodeToGroup(node: JsonCanvasGroupNode): PersistedGroupEntity {
  return {
    id: node.id,
    kind: 'group',
    label: node.label ?? '',
    canvasX: node.x,
    canvasY: node.y,
    width: node.width,
    height: node.height,
    parentGroupId: node.parentGroupId,
    color: node.groupColor ?? node.color,
    layoutMode: (node.layoutMode as PersistedGroupEntity['layoutMode']) ?? 'freeform',
    managedLayout: node.managedLayout ?? false,
    sourceTaskId: node.sourceTaskId,
    metadata: node.groupMetadata,
  }
}

function deserializeEdgeToWorkspaceEdge(edge: JsonCanvasEdge): WorkspaceEdge {
  return {
    id: edge.id,
    fromEntityId: edge.fromNode,
    toEntityId: edge.toNode,
    fromSide: edge.fromSide,
    toSide: edge.toSide,
    fromEnd: edge.fromEnd,
    toEnd: edge.toEnd,
    color: edge.color,
    label: edge.label,
    kind: (edge.edgeKind as WorkspaceEdge['kind']) ?? 'breakpoint_variant',
    metadata: edge.edgeMetadata,
  }
}
