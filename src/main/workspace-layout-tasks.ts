import type {
  ApplyTaskLayoutRequest,
  ApplyTaskLayoutResponse,
  LayoutComponentStatesRequest,
  LayoutComponentStatesResponse,
  PageConfig,
} from '../shared/types'
import {
  CLUSTER_HORIZONTAL_GUTTER,
  CLUSTER_VERTICAL_GUTTER,
  DEFAULT_BREAKPOINT_PRESET_LABELS,
  USER_GROUP_PADDING,
  VIEWPORT_PRESETS,
} from '../shared/constants'
import {
  focusCanvasBounds,
  setSelectedGroupId,
} from './runtime/ui-actions'
import { scheduleWorkspaceAutosave } from './runtime/workspace-autosave'
import { workspaceGroups } from './runtime/workspace-model'
import { getManifest } from './design-system-store'
import { normalizeUserUrl } from '../shared/url'
import { makeId, createGroup } from './workspace-utils'
import { createEdges } from './workspace-edges'
import { createPages } from './workspace-pages'
import { groupBounds } from './workspace-entities'
import { findPlacement } from './workspace-placement'

// --- Layout task helpers ---

function presetIndexByLabel(label: string): number {
  const idx = VIEWPORT_PRESETS.findIndex((preset) => preset.label === label)
  if (idx === -1) throw new Error(`Unknown preset label: ${label}`)
  return idx
}

function buildBreakpointLabel(url: string, customLabel?: string): string {
  if (customLabel?.trim()) return customLabel.trim()
  const parsed = new URL(url)
  const path = parsed.pathname === '/' ? '' : parsed.pathname
  return `Breakpoints: ${parsed.hostname}${path}`
}

function clusterBoundsForPresets(presetIndexes: number[]): { width: number; height: number } {
  const widths = presetIndexes.map((presetIndex) => VIEWPORT_PRESETS[presetIndex].width)
  const heights = presetIndexes.map((presetIndex) => VIEWPORT_PRESETS[presetIndex].height)
  const totalWidth =
    widths.reduce((sum, width) => sum + width, 0) +
    Math.max(0, widths.length - 1) * CLUSTER_HORIZONTAL_GUTTER
  return {
    width: totalWidth,
    height: Math.max(...heights),
  }
}

function duplicateClusterWarning(url: string, presetLabels: string[]): string[] {
  const duplicates = workspaceGroups.filter((group) => {
    if (group.metadata?.taskKind !== 'breakpoint_map') return false
    return (
      group.metadata?.url === url &&
      JSON.stringify(group.metadata?.presets ?? []) === JSON.stringify(presetLabels)
    )
  })
  return duplicates.length ? ['A breakpoint cluster for this URL already exists'] : []
}

function componentStatesLabel(component: string, customLabel?: string): string {
  if (customLabel?.trim()) return customLabel.trim()
  return `States: ${component}`
}

function cartesianProduct(values: Array<Array<[string, unknown]>>): Array<Record<string, unknown>> {
  if (!values.length) return [{}]
  let result: Array<Record<string, unknown>> = [{}]
  for (const axis of values) {
    const next: Array<Record<string, unknown>> = []
    for (const seed of result) {
      for (const [key, value] of axis) {
        next.push({ ...seed, [key]: value })
      }
    }
    result = next
  }
  return result
}

function stateDisplayLabel(props: Record<string, unknown>, state?: string): string {
  const entries = Object.entries(props).map(([key, value]) => `${key}=${String(value)}`)
  if (state) entries.push(state)
  return entries.length ? entries.join(' / ') : state ?? 'default'
}

function valuesFromManifestProp(
  componentName: string,
  propName: string,
): unknown[] | undefined {
  const manifest = getManifest()
  const def = manifest?.components[componentName]
  const prop = def?.propSignature.find((candidate) => candidate.name === propName)
  if (!prop) return undefined
  if (prop.type === 'enum') return prop.values?.length ? [...prop.values] : undefined
  if (prop.type === 'boolean') return [true, false]
  if (prop.defaultValue !== undefined) return [prop.defaultValue]
  return undefined
}

function resolveStateCombinations(
  request: LayoutComponentStatesRequest,
  manifestComponentKey: string,
): Array<{
  props: Record<string, unknown>
  pseudoState?: string
  pageUrl: string
  label: string
}> {
  const axes: Array<Array<[string, unknown]>> = []
  for (const propName of request.vary) {
    const explicit = request.values?.[propName]
    const candidates =
      explicit && explicit.length
        ? explicit
        : valuesFromManifestProp(manifestComponentKey, propName) ?? []
    if (!candidates.length) continue
    axes.push(candidates.map((value) => [propName, value]))
  }

  const baseCombos = cartesianProduct(axes)
  const states = request.states?.length ? request.states : [undefined]

  const combos: Array<{
    props: Record<string, unknown>
    pseudoState?: string
    pageUrl: string
    label: string
  }> = []

  for (const props of baseCombos.length ? baseCombos : [{}]) {
    for (const pseudoState of states) {
      const routeValue = props.route
      let pageUrl = normalizeUserUrl(request.url)
      if (typeof routeValue === 'string' && routeValue.trim()) {
        const trimmed = routeValue.trim()
        pageUrl = /^https?:\/\//.test(trimmed)
          ? normalizeUserUrl(trimmed)
          : new URL(trimmed, normalizeUserUrl(request.url)).toString()
      }
      const labelProps = { ...props }
      delete labelProps.route
      combos.push({
        props: labelProps,
        pseudoState,
        pageUrl,
        label: stateDisplayLabel(labelProps, pseudoState),
      })
    }
  }

  return combos
}

// --- Layout task commands ---

export function layoutComponentStates(
  request: LayoutComponentStatesRequest,
): LayoutComponentStatesResponse {
  const manifest = getManifest()
  if (!manifest) {
    throw new Error('No design system manifest is registered')
  }

  const manifestComponentKey = Object.keys(manifest.components).find((name) => {
    const def = manifest.components[name]
    return name === request.component || def.displayName === request.component
  })
  if (!manifestComponentKey) {
    throw new Error(`Unknown design system component: ${request.component}`)
  }

  const combinations = resolveStateCombinations(request, manifestComponentKey)
  if (!combinations.length) {
    throw new Error('No component-state combinations resolved')
  }

  const desktopPresetIndex = VIEWPORT_PRESETS.findIndex((preset) => preset.label === 'Desktop')
  const presetIndex = desktopPresetIndex >= 0 ? desktopPresetIndex : VIEWPORT_PRESETS.length - 1
  const preset = VIEWPORT_PRESETS[presetIndex]

  const firstAxis = request.vary[0]
  const firstAxisCount = request.values?.[firstAxis]?.length ?? 0
  const columnCount = Math.max(1, firstAxisCount || Math.ceil(Math.sqrt(combinations.length)))
  const rowCount = Math.max(1, Math.ceil(combinations.length / columnCount))

  const width = columnCount * preset.width + Math.max(0, columnCount - 1) * CLUSTER_HORIZONTAL_GUTTER
  const height = rowCount * preset.height + Math.max(0, rowCount - 1) * CLUSTER_VERTICAL_GUTTER
  const placement = findPlacement({
    width,
    height,
    anchor: request.anchor ?? 'selection_or_empty_region',
  })

  const taskId = makeId('task')
  const groupId = makeId('group')
  const group = createGroup({
    id: groupId,
    kind: 'group',
    label: componentStatesLabel(request.component, request.label),
    canvasX: placement.canvasX - USER_GROUP_PADDING,
    canvasY: placement.canvasY - USER_GROUP_PADDING,
    width: width + USER_GROUP_PADDING * 2,
    height: height + USER_GROUP_PADDING * 2,
    layoutMode: 'row',
    managedLayout: true,
    pageIds: [],
    sourceTaskId: taskId,
    metadata: {
      taskKind: 'component_states',
      component: request.component,
      url: normalizeUserUrl(request.url),
      vary: [...request.vary],
      generatedAt: new Date().toISOString(),
    },
  })

  const pages: PageConfig[] = combinations.map((combo, index) => {
    const row = Math.floor(index / columnCount)
    const col = index % columnCount
    const canvasX = placement.canvasX + col * (preset.width + CLUSTER_HORIZONTAL_GUTTER)
    const canvasY = placement.canvasY + row * (preset.height + CLUSTER_VERTICAL_GUTTER)
    const propOverrides = Object.fromEntries(
      Object.entries(combo.props).map(([key, value]) => [
        key,
        {
          propPath: [key],
          value,
        },
      ]),
    )

    return {
      id: makeId('page'),
      url: combo.pageUrl,
      presetIndex,
      canvasX,
      canvasY,
      linked: true,
      source: 'generated',
      parentGroupId: groupId,
      groupId,
      metadata: {
        taskKind: 'component_states',
        component: request.component,
        label: combo.label,
        overrides: {
          component: manifestComponentKey,
          selector: request.selector,
          pseudoState: combo.pseudoState,
          props: propOverrides,
          tokens: request.tokens ?? {},
          label: combo.label,
        },
      },
    }
  })

  const { pageIds } = createPages({ pages })
  group.pageIds = [...pageIds]

  if (request.focus ?? true) {
    const bounds = groupBounds(group)
    if (bounds) {
      setSelectedGroupId(group.id)
      focusCanvasBounds(bounds)
    }
  }

  scheduleWorkspaceAutosave()

  return {
    taskId,
    groupId,
    pageIds,
    placement,
    warnings: [],
  }
}

export function applyTaskLayout(
  request: ApplyTaskLayoutRequest,
): ApplyTaskLayoutResponse {
  if (request.taskKind !== 'breakpoint_map') {
    throw new Error(`Unsupported task kind: ${request.taskKind}`)
  }

  const url = normalizeUserUrl(request.input.url)
  const presetLabels = request.input.presets?.length
    ? request.input.presets
    : DEFAULT_BREAKPOINT_PRESET_LABELS
  if (new Set(presetLabels).size !== presetLabels.length) {
    throw new Error('Duplicate preset labels are not allowed')
  }
  const presetIndexes = presetLabels.map(presetIndexByLabel)
  const clusterSize = clusterBoundsForPresets(presetIndexes)
  const placement = findPlacement({
    width: clusterSize.width,
    height: clusterSize.height,
    anchor: request.options?.anchor ?? 'selection_or_empty_region',
  })

  const taskId = makeId('task')
  const groupId = makeId('group')
  const warnings = duplicateClusterWarning(url, presetLabels)
  const group = createGroup({
    id: groupId,
    kind: 'group',
    label: buildBreakpointLabel(url, request.input.label),
    canvasX: placement.canvasX - USER_GROUP_PADDING,
    canvasY: placement.canvasY - USER_GROUP_PADDING,
    width: clusterSize.width + USER_GROUP_PADDING * 2,
    height: clusterSize.height + USER_GROUP_PADDING * 2,
    layoutMode: 'row',
    managedLayout: true,
    pageIds: [],
    sourceTaskId: taskId,
    metadata: {
      taskKind: request.taskKind,
      url,
      presets: presetLabels,
      generatedAt: new Date().toISOString(),
    },
  })

  let cursorX = placement.canvasX
  const pagesToCreate = presetIndexes.map((presetIndex) => {
    const preset = VIEWPORT_PRESETS[presetIndex]
    const id = makeId('page')
    const page = {
      id,
      url,
      presetIndex,
      canvasX: cursorX,
      canvasY: placement.canvasY,
      linked: true,
      source: 'generated' as const,
      parentGroupId: groupId,
      groupId,
      metadata: {
        taskKind: request.taskKind,
        url,
        preset: preset.label,
      },
    }
    cursorX += preset.width + CLUSTER_HORIZONTAL_GUTTER
    return page
  })

  const { pageIds } = createPages({ pages: pagesToCreate })
  group.pageIds = [...pageIds]

  const { edgeIds } = createEdges({
    edges: pageIds.slice(0, -1).map((pageId, index) => ({
      fromEntityId: pageId,
      toEntityId: pageIds[index + 1],
      kind: 'breakpoint_variant' as const,
      metadata: {
        taskKind: request.taskKind,
        url,
      },
    })),
  })

  if (request.options?.focus ?? true) {
    const bounds = groupBounds(group)
    if (bounds) {
      setSelectedGroupId(group.id)
      focusCanvasBounds(bounds)
    }
  }

  scheduleWorkspaceAutosave()

  return {
    taskId,
    taskKind: request.taskKind,
    groupId,
    pageIds,
    edgeIds,
    resolvedPresets: presetLabels,
    placement,
    warnings,
  }
}
