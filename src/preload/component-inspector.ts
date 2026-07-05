import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import type { ComponentTreeNode, ComponentNodeDetail } from '../shared/types'
import type { DesignSystemManifest } from '../shared/design-system-types'

type Fiber = {
  child?: Fiber | null
  sibling?: Fiber | null
  return?: Fiber | null
  key?: string | null
  type?: unknown
  memoizedProps?: unknown
  stateNode?: unknown
  _debugSource?: { fileName?: string; lineNumber?: number; columnNumber?: number } | null
}

type RendererInterface = {
  overrideProps?: (fiber: unknown, path: string[], value: unknown) => void
}

type ReactDevtoolsHook = {
  renderers?: Map<number, unknown>
  rendererInterfaces?: Map<number, RendererInterface>
  getFiberRoots?: (rendererId: number) => Set<{ current?: Fiber | null }> | undefined
}

interface ComponentRuntimeRef {
  fiber: Fiber
  rendererId: number
  element: Element | null
}

// Framework internals that should be filtered from the hierarchy.
// These are never user-authored and never useful as edit targets.
const FRAMEWORK_INTERNALS = new Set([
  'Suspense',
  'Fragment',
  'StrictMode',
  'Profiler',
  'Provider',
  'Consumer',
  'Context',
  'ContextProvider',
  'ContextConsumer',
  'InnerLayoutRouter',
  'OuterLayoutRouter',
  'AppRouter',
  'HotReload',
  'ReactDevOverlay',
  'ErrorBoundary',
  'RenderFromTemplateContext',
  'ScrollAndFocusHandler',
  'RedirectErrorBoundary',
  'NotFoundErrorBoundary',
  'LoadingBoundary',
  'InnerScrollAndFocusHandler',
])

function isFrameworkInternal(name: string): boolean {
  if (FRAMEWORK_INTERNALS.has(name)) return true
  // Anonymous wrappers and library internals
  if (name.startsWith('_') || name.startsWith('$')) return true
  // ForwardRef/memo wrappers without a real name
  if (name === 'ForwardRef' || name === 'Memo') return true
  return false
}

let manifest: DesignSystemManifest | null = null
let lastSentTreeKey = ''
let publishTimer: NodeJS.Timeout | null = null
let showAllNodes = false
const nodeRefMap = new Map<string, ComponentRuntimeRef>()
const nodeElementMap = new Map<string, Element>()
let elementNodeIdMap: WeakMap<Element, string> = new WeakMap()
const sourceLocationCache: WeakMap<Fiber, { fileName?: string; lineNumber?: number; columnNumber?: number } | null> = new WeakMap()
const pseudoStyleCache = new Set<'hover' | 'focus' | 'disabled'>()

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
  return value as Record<string, unknown>
}

function compactProps(props: unknown): Record<string, unknown> {
  const raw = asRecord(props)
  const output: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(raw)) {
    if (typeof value === 'function') continue
    if (typeof value === 'symbol') continue
    if (value instanceof Element) continue
    if (value === undefined) continue
    try {
      output[key] = JSON.parse(JSON.stringify(value)) as unknown
    } catch {
      output[key] = String(value)
    }
  }
  return output
}

function nodeDisplayName(fiber: Fiber): string | null {
  const type = fiber.type as { displayName?: string; name?: string } | string | undefined
  if (!type) return null
  if (typeof type === 'string') return type
  return type.displayName ?? type.name ?? null
}

function findNearestElement(fiber: Fiber | null | undefined): Element | null {
  if (!fiber) return null
  const stateNode = fiber.stateNode
  if (stateNode instanceof Element) return stateNode
  return findNearestElement(fiber.child ?? null)
}

function matchDesignSystemComponent(
  componentName: string,
  element: Element | null,
): string | undefined {
  if (!manifest) return undefined
  for (const [name, definition] of Object.entries(manifest.components)) {
    if (definition.fiberNames.includes(componentName)) return name
    if (!element) continue
    if (definition.selectors.some((selector) => element.matches(selector))) return name
  }
  return undefined
}

function readTokenValues(
  dsComponentName: string | undefined,
  element: Element | null,
): Record<string, string> {
  if (!manifest || !dsComponentName || !element) return {}
  const definition = manifest.components[dsComponentName]
  if (!definition) return {}
  const computed = window.getComputedStyle(element)
  const values: Record<string, string> = {}
  for (const token of definition.tokens) {
    values[token] = computed.getPropertyValue(token).trim()
  }
  return values
}

function fiberHasSource(fiber: Fiber): boolean {
  if (sourceLocationCache.has(fiber)) return sourceLocationCache.get(fiber) != null
  const src = fiber._debugSource ?? null
  sourceLocationCache.set(fiber, src)
  return src != null
}

function resolveFiberSource(fiber: Fiber): { fileName?: string; lineNumber?: number; columnNumber?: number } | null {
  const cached = sourceLocationCache.get(fiber)
  if (cached !== undefined) return cached
  const src = fiber._debugSource ?? null
  sourceLocationCache.set(fiber, src)
  return src
}

/**
 * Determine whether a component should appear in the hierarchy.
 * DS-matched and source-bearing components always stay, even if their name
 * looks like an internal.
 */
function shouldIncludeInTree(name: string, fiber: Fiber, dsComponentName: string | undefined): boolean {
  if (showAllNodes) return true
  if (dsComponentName) return true
  if (fiberHasSource(fiber)) return true
  return !isFrameworkInternal(name)
}

// --- Tier 1: Skeleton tree (hot path, every 500ms) ---

function walkFiber(
  fiber: Fiber | null | undefined,
  rendererId: number,
  trail: string[],
): ComponentTreeNode[] {
  const output: ComponentTreeNode[] = []
  let cursor = fiber ?? null
  let index = 0
  while (cursor) {
    const name = nodeDisplayName(cursor)
    const nextTrail = [...trail, `${index}:${cursor.key ?? ''}`]
    if (name) {
      const id = `r${rendererId}:${nextTrail.join('/')}`
      const element = findNearestElement(cursor)
      const dsComponentName = matchDesignSystemComponent(name, element)

      if (shouldIncludeInTree(name, cursor, dsComponentName)) {
        const node: ComponentTreeNode = {
          id,
          componentName: name,
          dsComponentName,
          hasSource: fiberHasSource(cursor),
          children: walkFiber(cursor.child ?? null, rendererId, nextTrail),
        }
        nodeRefMap.set(id, { fiber: cursor, rendererId, element })
        if (element) {
          nodeElementMap.set(id, element)
          elementNodeIdMap.set(element, id)
        }
        output.push(node)
      } else {
        // Skip this node but include its children (hoist them up)
        output.push(...walkFiber(cursor.child ?? null, rendererId, nextTrail))
      }
    } else {
      output.push(...walkFiber(cursor.child ?? null, rendererId, nextTrail))
    }
    cursor = cursor.sibling ?? null
    index += 1
  }
  return output
}

function collectReactTree(): ComponentTreeNode[] {
  const hook = (window as unknown as { __REACT_DEVTOOLS_GLOBAL_HOOK__?: ReactDevtoolsHook })
    .__REACT_DEVTOOLS_GLOBAL_HOOK__
  if (!hook?.renderers || !hook.getFiberRoots) return []

  const tree: ComponentTreeNode[] = []
  nodeRefMap.clear()
  nodeElementMap.clear()
  elementNodeIdMap = new WeakMap()
  for (const rendererId of hook.renderers.keys()) {
    const roots = hook.getFiberRoots(rendererId)
    if (!roots) continue
    for (const root of roots) {
      tree.push(...walkFiber(root.current ?? null, rendererId, [`root:${rendererId}`]))
    }
  }
  return tree
}

function domNodeId(element: Element, trail: string): string {
  const id = element.getAttribute('id')
  if (id) return `dom:${trail}#${id}`
  return `dom:${trail}:${element.tagName.toLowerCase()}`
}

function domProps(element: Element): Record<string, unknown> {
  const props: Record<string, unknown> = {}
  for (const attribute of [...element.attributes].slice(0, 10)) {
    props[attribute.name] = attribute.value
  }
  return props
}

function inlineTokenValues(element: Element): Record<string, string> {
  const style = (element as HTMLElement).style
  if (!style) return {}
  const tokens: Record<string, string> = {}
  for (let i = 0; i < style.length; i += 1) {
    const name = style.item(i)
    if (!name.startsWith('--')) continue
    tokens[name] = style.getPropertyValue(name).trim()
  }
  return tokens
}

function walkDom(
  element: Element,
  trail: string,
  depth: number,
  maxDepth: number,
): ComponentTreeNode {
  const id = domNodeId(element, trail)
  nodeElementMap.set(id, element)
  elementNodeIdMap.set(element, id)
  const children =
    depth >= maxDepth
      ? []
      : [...element.children]
          .filter((child) => {
            const tag = child.tagName.toLowerCase()
            return (
              tag !== 'script' &&
              tag !== 'style' &&
              tag !== 'noscript' &&
              tag !== 'meta' &&
              tag !== 'link'
            )
          })
          .slice(0, 80)
          .map((child, index) =>
          walkDom(child, `${trail}.${index}`, depth + 1, maxDepth),
        )

  return {
    id,
    componentName: element.tagName.toLowerCase(),
    hasSource: false,
    children,
  }
}

function collectDomTree(maxDepth = 12): ComponentTreeNode[] {
  const roots = [...document.body.children]
    .filter((element) => {
      const tag = element.tagName.toLowerCase()
      return (
        tag !== 'script' &&
        tag !== 'style' &&
        tag !== 'noscript' &&
        tag !== 'meta' &&
        tag !== 'link'
      )
    })
    .slice(0, 80)
  return roots.map((element, index) => walkDom(element, `${index}`, 0, maxDepth))
}

function publishTreeNow(): void {
  let tree = collectReactTree()
  if (!tree.length) {
    tree = collectDomTree()
  }
  const nextKey = JSON.stringify(tree)
  if (nextKey === lastSentTreeKey) return
  lastSentTreeKey = nextKey
  ipcRenderer.send(ipcChannels.inspectTreeUpdate, tree)
}

function scheduleTreePublish(): void {
  if (publishTimer) return
  publishTimer = setTimeout(() => {
    publishTimer = null
    publishTreeNow()
  }, 500)
}

// --- Tier 2: Rich detail (cold path, on demand) ---

function resolveNodeDetail(nodeId: string): ComponentNodeDetail | null {
  const ref = nodeRefMap.get(nodeId)

  // DOM fallback nodes
  if (!ref) {
    const element = nodeElementMap.get(nodeId)
    if (!element) return null
    return {
      props: domProps(element),
      tokens: inlineTokenValues(element),
    }
  }

  const { fiber, element } = ref
  const dsComponentName = ref.element
    ? matchDesignSystemComponent(nodeDisplayName(fiber) ?? '', element)
    : undefined

  const source = resolveFiberSource(fiber)
  const sourceLocation = source?.fileName
    ? { file: source.fileName, line: source.lineNumber, column: source.columnNumber }
    : undefined

  const detail: ComponentNodeDetail = {
    props: compactProps(fiber.memoizedProps),
    tokens: readTokenValues(dsComponentName, element),
    sourceLocation,
    dsComponentName,
  }

  // Attach DS manifest info for the matched component
  if (dsComponentName && manifest) {
    const definition = manifest.components[dsComponentName]
    if (definition) {
      if (Object.keys(definition.variants).length > 0) {
        detail.dsVariants = definition.variants
      }
      if (definition.propSignature.length > 0) {
        detail.dsPropSignature = definition.propSignature.map((p) => ({
          name: p.name,
          type: p.type,
          ...(p.values?.length ? { values: p.values } : {}),
          ...(p.defaultValue !== undefined ? { defaultValue: p.defaultValue } : {}),
        }))
      }
    }
  }

  return detail
}

// --- Override functions ---

function overrideToken(payload: {
  token?: string
  value?: string
  selector?: string
  componentId?: string
}): void {
  const token = payload.token?.trim()
  if (!token) return
  const value = payload.value ?? ''

  let target: Element | null =
    payload.selector?.trim() ? document.querySelector(payload.selector) : null
  if (!target && payload.componentId) {
    target = nodeRefMap.get(payload.componentId)?.element ?? null
  }
  if (!target) {
    target = document.documentElement
  }
  ;(target as HTMLElement).style.setProperty(token, value)
  scheduleTreePublish()
}

function overrideProps(payload: {
  componentId?: string
  propPath?: string[]
  value?: unknown
}): void {
  if (!payload.componentId || !Array.isArray(payload.propPath)) return
  const nodeRef = nodeRefMap.get(payload.componentId)
  if (!nodeRef) return
  const hook = (window as unknown as { __REACT_DEVTOOLS_GLOBAL_HOOK__?: ReactDevtoolsHook })
    .__REACT_DEVTOOLS_GLOBAL_HOOK__
  const renderer = hook?.rendererInterfaces?.get(nodeRef.rendererId)
  renderer?.overrideProps?.(nodeRef.fiber, payload.propPath, payload.value)
  scheduleTreePublish()
}

function cssRuleTextForPseudoState(
  ruleList: CSSRuleList,
  state: 'hover' | 'focus' | 'disabled',
): string {
  let output = ''
  const pseudo = `:${state}`
  const forceClass = `.__force-${state}`

  for (const rule of [...ruleList]) {
    if (rule instanceof CSSStyleRule) {
      if (!rule.selectorText.includes(pseudo)) continue
      const selectors = rule.selectorText
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean)
      const forcedSelectors = selectors
        .filter((selector) => selector.includes(pseudo))
        .map((selector) => selector.replaceAll(pseudo, forceClass))
      if (!forcedSelectors.length) continue
      output += `${forcedSelectors.join(', ')} { ${rule.style.cssText} }\n`
      continue
    }

    if (rule instanceof CSSMediaRule) {
      const nested = cssRuleTextForPseudoState(rule.cssRules, state)
      if (nested) output += `@media ${rule.conditionText} { ${nested} }\n`
      continue
    }

    if (rule instanceof CSSSupportsRule) {
      const nested = cssRuleTextForPseudoState(rule.cssRules, state)
      if (nested) output += `@supports ${rule.conditionText} { ${nested} }\n`
    }
  }

  return output
}

function ensurePseudoStateStyles(state: 'hover' | 'focus' | 'disabled'): void {
  if (pseudoStyleCache.has(state)) return
  const existing = document.getElementById(`__force-pseudo-${state}`)
  if (existing) {
    pseudoStyleCache.add(state)
    return
  }

  let css = ''
  for (const styleSheet of [...document.styleSheets]) {
    try {
      if (!styleSheet.cssRules) continue
      css += cssRuleTextForPseudoState(styleSheet.cssRules, state)
    } catch {
      // Ignore cross-origin stylesheets that block cssRules access.
    }
  }

  const styleEl = document.createElement('style')
  styleEl.id = `__force-pseudo-${state}`
  styleEl.textContent = css
  document.head.appendChild(styleEl)
  pseudoStyleCache.add(state)
}

function clearForcedPseudoStateClasses(): void {
  for (const state of ['hover', 'focus', 'disabled'] as const) {
    for (const element of document.querySelectorAll(`.__force-${state}`)) {
      element.classList.remove(`.__force-${state}`)
    }
  }
}

function forcePseudoState(
  target: Element,
  state: 'hover' | 'focus' | 'disabled',
): void {
  ensurePseudoStateStyles(state)
  target.classList.add(`__force-${state}`)
  if (state === 'focus' && target instanceof HTMLElement) {
    target.focus({ preventScroll: true })
  }
  if (state === 'disabled' && target instanceof HTMLElement) {
    target.setAttribute('disabled', 'true')
    target.setAttribute('aria-disabled', 'true')
  }
}

function applyPageOverrides(payload: unknown): void {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) return
  const overrides = payload as {
    localStorage?: Record<string, string>
    tokens?: Record<string, string>
    props?: Record<string, { componentId?: string; propPath?: string[]; value?: unknown }>
    pseudoState?: 'hover' | 'focus' | 'disabled'
    selector?: string
  }

  if (overrides.localStorage) {
    for (const [key, value] of Object.entries(overrides.localStorage)) {
      localStorage.setItem(key, String(value))
    }
  }

  const applyAfterMount = () => {
    clearForcedPseudoStateClasses()
    if (overrides.tokens) {
      for (const [token, value] of Object.entries(overrides.tokens)) {
        overrideToken({ token, value, selector: undefined })
      }
    }
    if (overrides.props) {
      for (const entry of Object.values(overrides.props)) {
        overrideProps({
          componentId: entry.componentId,
          propPath: entry.propPath,
          value: entry.value,
        })
      }
    }
    if (overrides.pseudoState) {
      const target = overrides.selector
        ? document.querySelector(overrides.selector)
        : document.body
      if (target) {
        forcePseudoState(target, overrides.pseudoState)
      }
    }
  }

  setTimeout(applyAfterMount, 450)
}

/**
 * Attempt to hook into React's commit cycle via __REACT_DEVTOOLS_GLOBAL_HOOK__.
 * Returns true if the hook was installed, false if React hasn't loaded yet.
 */
function installCommitHook(): boolean {
  const hook = (window as unknown as { __REACT_DEVTOOLS_GLOBAL_HOOK__?: ReactDevtoolsHook & { _webCanvasPatched?: boolean } })
    .__REACT_DEVTOOLS_GLOBAL_HOOK__

  if (!hook || hook._webCanvasPatched) return !!hook?._webCanvasPatched
  if (!hook.renderers) return false

  // Patch onCommitFiberRoot so we're notified on every React commit.
  const original = (hook as unknown as Record<string, unknown>).onCommitFiberRoot as
    | ((...args: unknown[]) => void)
    | undefined
  ;(hook as unknown as Record<string, unknown>).onCommitFiberRoot = (...args: unknown[]) => {
    original?.(...args)
    scheduleTreePublish()
  }
  hook._webCanvasPatched = true
  return true
}

/**
 * Start a MutationObserver as a fallback for pages without React or where
 * the commit hook couldn't be installed. Fires at most once per animation
 * page to avoid overwhelming the publisher.
 */
function installMutationObserver(): MutationObserver {
  let pending = false
  const observer = new MutationObserver(() => {
    if (pending) return
    pending = true
    requestAnimationFrame(() => {
      pending = false
      scheduleTreePublish()
    })
  })
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
    attributes: true,
    attributeFilter: ['class', 'style', 'data-state', 'aria-expanded', 'hidden'],
  })
  return observer
}

export function initComponentInspector(): void {
  ipcRenderer.on(ipcChannels.setDesignSystemManifest, (_event, nextManifest) => {
    manifest = (nextManifest as DesignSystemManifest | null) ?? null
    scheduleTreePublish()
  })

  ipcRenderer.on(ipcChannels.overrideToken, (_event, payload) => {
    overrideToken(payload ?? {})
  })

  ipcRenderer.on(ipcChannels.overrideProps, (_event, payload) => {
    overrideProps(payload ?? {})
  })

  ipcRenderer.on(ipcChannels.applyPageOverrides, (_event, payload) => {
    applyPageOverrides(payload)
  })

  // Tier 2: Resolve full detail on demand from main process
  ipcRenderer.on(ipcChannels.resolveNodeDetail, (_event, { nodeId, requestId }: { nodeId: string; requestId: string }) => {
    const detail = resolveNodeDetail(nodeId)
    ipcRenderer.send(ipcChannels.resolveNodeDetailResponse, { requestId, nodeId, detail })
  })

  // Filter toggle: allow the devtools panel to request unfiltered tree
  ipcRenderer.on(ipcChannels.setShowAllNodes, (_event, value: boolean) => {
    showAllNodes = value
    lastSentTreeKey = '' // force re-publish with new filter
    scheduleTreePublish()
  })

  const boot = () => {
    // Try to hook into React's commit cycle
    const hooked = installCommitHook()

    // Always install MutationObserver as a complement:
    // - If React hook succeeded: catches non-React DOM changes
    // - If React hook failed: serves as the primary trigger
    installMutationObserver()

    // If the hook isn't available yet (React hasn't loaded), retry a few times.
    // Once React loads and registers renderers, the hook becomes available.
    if (!hooked) {
      let retries = 0
      const retryInterval = setInterval(() => {
        if (installCommitHook() || retries >= 10) {
          clearInterval(retryInterval)
        }
        retries += 1
      }, 1000)
    }

    // Initial tree publish
    scheduleTreePublish()
  }

  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', () => setTimeout(boot, 300), { once: true })
  } else {
    setTimeout(boot, 300)
  }
}

export function getInspectableNodeIdForElement(element: Element | null): string | null {
  if (!element) return null
  return elementNodeIdMap.get(element) ?? null
}

export function getInspectableElementByNodeId(nodeId: string | null | undefined): Element | null {
  if (!nodeId) return null
  return nodeElementMap.get(nodeId) ?? null
}
