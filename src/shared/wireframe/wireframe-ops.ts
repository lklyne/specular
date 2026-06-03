// Pure tree operations on .wireframe.json content.
//
// These are side-effect-free (file, …) → WireframeFile functions plus a
// structural validator. They live in src/shared/ so both the renderer (canvas
// gestures) and the main process (agent CLI route) drive the *same* op set —
// the "one op set for humans and agents" thesis. No Electron, no React, no
// Date.now()/Math.random(): every op is deterministic-by-input and unit-testable.

import type { WireframeFile, WireframeNode } from './wireframe-types'

// --- Tree query helpers ---

function findNodeParent(
  root: WireframeNode,
  nodeId: string,
): { parentId: string; index: number } | null {
  if (root.type !== 'frame') return null
  for (let i = 0; i < root.children.length; i++) {
    if (root.children[i].id === nodeId) return { parentId: root.id, index: i }
  }
  for (const child of root.children) {
    const result = findNodeParent(child, nodeId)
    if (result) return result
  }
  return null
}

export function findNodeById(root: WireframeNode, id: string): WireframeNode | null {
  if (root.id === id) return root
  if (root.type === 'frame') {
    for (const child of root.children) {
      const found = findNodeById(child, id)
      if (found) return found
    }
  }
  return null
}

// --- Tree mutation helpers ---

function removeNodeFromTree(
  node: WireframeNode,
  nodeId: string,
): { tree: WireframeNode; removed: WireframeNode | null } {
  if (node.type !== 'frame') return { tree: node, removed: null }

  const idx = node.children.findIndex((c) => c.id === nodeId)
  if (idx !== -1) {
    const removed = node.children[idx]
    return {
      tree: { ...node, children: [...node.children.slice(0, idx), ...node.children.slice(idx + 1)] },
      removed,
    }
  }

  let removedNode: WireframeNode | null = null
  const newChildren = node.children.map((child) => {
    if (removedNode) return child
    const { tree, removed } = removeNodeFromTree(child, nodeId)
    if (removed) removedNode = removed
    return tree
  })

  return { tree: { ...node, children: newChildren }, removed: removedNode }
}

function insertNodeInTree(
  tree: WireframeNode,
  parentId: string,
  index: number,
  nodeToInsert: WireframeNode,
): WireframeNode {
  if (tree.type !== 'frame') return tree

  if (tree.id === parentId) {
    const newChildren = [...tree.children]
    newChildren.splice(index, 0, nodeToInsert)
    return { ...tree, children: newChildren }
  }

  return {
    ...tree,
    children: tree.children.map((child) => insertNodeInTree(child, parentId, index, nodeToInsert)),
  }
}

// --- Reorder / text / state (lifted unchanged from the renderer) ---

export function reorderNode(
  file: WireframeFile,
  nodeId: string,
  targetParentId: string,
  targetIndex: number,
): WireframeFile {
  const sourceInfo = findNodeParent(file.root, nodeId)
  if (!sourceInfo) return file

  let adjustedIndex = targetIndex
  if (sourceInfo.parentId === targetParentId && sourceInfo.index < targetIndex) {
    adjustedIndex -= 1
  }

  if (sourceInfo.parentId === targetParentId && sourceInfo.index === adjustedIndex) {
    return file
  }

  const { tree, removed } = removeNodeFromTree(file.root, nodeId)
  if (!removed) return file

  const newRoot = insertNodeInTree(tree, targetParentId, adjustedIndex, removed)
  return { ...file, root: newRoot }
}

export function updateNodeText(file: WireframeFile, nodeId: string, value: string): WireframeFile {
  return { ...file, root: updateNodeTextInTree(file.root, nodeId, value) }
}

function updateNodeTextInTree(node: WireframeNode, nodeId: string, value: string): WireframeNode {
  if (node.id === nodeId) {
    switch (node.type) {
      case 'text':
      case 'button':
        return { ...node, text: value }
      case 'input':
      case 'dropdown':
        return { ...node, placeholder: value }
      case 'checkbox':
      case 'toggle':
        return { ...node, label: value }
      default:
        return node
    }
  }
  if (node.type === 'frame') {
    return { ...node, children: node.children.map((child) => updateNodeTextInTree(child, nodeId, value)) }
  }
  return node
}

export function toggleNodeState(file: WireframeFile, nodeId: string): WireframeFile {
  return { ...file, root: toggleNodeStateInTree(file.root, nodeId) }
}

function toggleNodeStateInTree(node: WireframeNode, nodeId: string): WireframeNode {
  if (node.id === nodeId) {
    if (node.type === 'checkbox') return { ...node, checked: !node.checked }
    if (node.type === 'toggle') return { ...node, on: !node.on }
    return node
  }
  if (node.type === 'frame') {
    return { ...node, children: node.children.map((child) => toggleNodeStateInTree(child, nodeId)) }
  }
  return node
}

// --- Structural ops (insert / delete / duplicate / property edit) ---

/**
 * Insert `node` as a child of the frame `parentId` at `index`. The index is
 * clamped to `[0, children.length]`. No-op (returns the file unchanged) when
 * `parentId` is unknown or is not a frame.
 */
export function insertNode(
  file: WireframeFile,
  parentId: string,
  index: number,
  node: WireframeNode,
): WireframeFile {
  const parent = findNodeById(file.root, parentId)
  if (!parent || parent.type !== 'frame') return file
  const clampedIndex = Math.max(0, Math.min(index, parent.children.length))
  return { ...file, root: insertNodeInTree(file.root, parentId, clampedIndex, node) }
}

/**
 * Remove the subtree rooted at `id`. No-op for an unknown id, and refuses to
 * delete the root (a wireframe always has a root frame).
 */
export function deleteNode(file: WireframeFile, id: string): WireframeFile {
  if (file.root.id === id) return file
  const { tree, removed } = removeNodeFromTree(file.root, id)
  if (!removed) return file
  return { ...file, root: tree }
}

/**
 * Deep-clone the subtree rooted at `id`, assigning every cloned node a fresh id
 * from `genId`, and insert the clone directly after the original. No-op for an
 * unknown id or for the root (which has no parent to duplicate into).
 *
 * `genId` is the deterministic-by-input id source — pass `createNodeIdGenerator`
 * or any counter so the result is reproducible (no Date.now()/Math.random()).
 */
export function duplicateNode(
  file: WireframeFile,
  id: string,
  genId: () => string,
): WireframeFile {
  const info = findNodeParent(file.root, id)
  const original = findNodeById(file.root, id)
  if (!info || !original) return file
  const clone = cloneWithFreshIds(original, genId)
  return { ...file, root: insertNodeInTree(file.root, info.parentId, info.index + 1, clone) }
}

function cloneWithFreshIds(node: WireframeNode, genId: () => string): WireframeNode {
  if (node.type === 'frame') {
    return {
      ...node,
      id: genId(),
      children: node.children.map((child) => cloneWithFreshIds(child, genId)),
    }
  }
  return { ...node, id: genId() }
}

/**
 * A deterministic id generator: `${prefix}-1`, `${prefix}-2`, …. Same prefix +
 * same call order ⇒ same ids, so callers stay reproducible without wall-clock
 * or randomness in the pure layer.
 */
export function createNodeIdGenerator(prefix: string): () => string {
  let counter = 0
  return () => {
    counter += 1
    return `${prefix}-${counter}`
  }
}

/** Props that may be patched per node type (id/type/children are never patchable). */
const LEGAL_PROPS: Record<WireframeNode['type'], ReadonlySet<string>> = {
  frame: new Set(['direction', 'gap', 'padding', 'width', 'height']),
  text: new Set(['text', 'level']),
  button: new Set(['text', 'variant']),
  input: new Set(['placeholder', 'label']),
  dropdown: new Set(['placeholder', 'options', 'label']),
  checkbox: new Set(['label', 'checked']),
  toggle: new Set(['label', 'on']),
  image: new Set(['width', 'height', 'alt']),
  divider: new Set([]),
  spacer: new Set([]),
}

/**
 * Patch the named node's props in place. No-op for an unknown id. Throws when a
 * patch key is not legal for the node's type — the validating boundary the CLI
 * route surfaces as a 4xx.
 */
export function updateNodeProps(
  file: WireframeFile,
  id: string,
  patch: Record<string, unknown>,
): WireframeFile {
  const node = findNodeById(file.root, id)
  if (!node) return file
  const legal = LEGAL_PROPS[node.type]
  for (const key of Object.keys(patch)) {
    if (!legal.has(key)) {
      throw new Error(`Property "${key}" is not valid for node type "${node.type}"`)
    }
  }
  return { ...file, root: updateNodePropsInTree(file.root, id, patch) }
}

function updateNodePropsInTree(
  node: WireframeNode,
  id: string,
  patch: Record<string, unknown>,
): WireframeNode {
  if (node.id === id) {
    return { ...node, ...patch } as WireframeNode
  }
  if (node.type === 'frame') {
    return { ...node, children: node.children.map((child) => updateNodePropsInTree(child, id, patch)) }
  }
  return node
}

// --- Validation ---

const NODE_TYPES: ReadonlySet<string> = new Set([
  'frame',
  'text',
  'button',
  'input',
  'dropdown',
  'checkbox',
  'toggle',
  'image',
  'divider',
  'spacer',
])

export type WireframeValidation = { ok: true } | { ok: false; errors: string[] }

/**
 * Structurally validate parsed wireframe content. Returns the full error list so
 * the CLI route can surface a legible 4xx. Checks: version === '1.0', a frame
 * root, every node has a known type and an id, frames carry a children array,
 * and non-frame nodes carry no children.
 */
export function validateWireframe(content: unknown): WireframeValidation {
  const errors: string[] = []

  if (typeof content !== 'object' || content === null) {
    return { ok: false, errors: ['Wireframe must be an object'] }
  }

  const file = content as Record<string, unknown>
  if (file.version !== '1.0') {
    errors.push('Missing or invalid version (expected "1.0")')
  }

  const root = file.root
  if (typeof root !== 'object' || root === null) {
    errors.push('Missing root node')
  } else {
    if ((root as Record<string, unknown>).type !== 'frame') {
      errors.push('Root node must be a frame')
    }
    validateNode(root, errors)
  }

  return errors.length === 0 ? { ok: true } : { ok: false, errors }
}

function validateNode(node: unknown, errors: string[]): void {
  if (typeof node !== 'object' || node === null) {
    errors.push('Node must be an object')
    return
  }
  const n = node as Record<string, unknown>

  if (typeof n.type !== 'string' || !NODE_TYPES.has(n.type)) {
    errors.push(`Unknown node type: ${String(n.type)}`)
    return
  }
  if (typeof n.id !== 'string' || n.id.length === 0) {
    errors.push('Node missing id')
  }

  if (n.type === 'frame') {
    if (!Array.isArray(n.children)) {
      errors.push(`Frame ${String(n.id)} missing children array`)
    } else {
      for (const child of n.children) validateNode(child, errors)
    }
  } else if (n.children !== undefined) {
    errors.push(`Non-frame node ${String(n.id)} (${n.type}) cannot have children`)
  }
}
