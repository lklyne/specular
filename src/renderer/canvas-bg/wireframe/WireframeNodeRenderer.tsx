import { useRef } from 'react'
import type {
  WireframeNode,
  WireframeFrame,
  WireframeText,
  WireframeButton,
  WireframeInput,
  WireframeDropdown,
  WireframeCheckbox,
  WireframeToggle,
  WireframeImage,
  DropTarget,
} from '../../../shared/wireframe/wireframe-types'
import type { WireframeThemeColors } from './wireframe-themes'
import { sizingToFlex, sizingToWidth, sizingToHeight, parsePadding } from './wireframe-utils'

export interface WireframeNodeRendererProps {
  node: WireframeNode
  theme: WireframeThemeColors
  canEdit: boolean
  draggedNodeId: string | null
  dropTarget: DropTarget | null
  editingNodeId: string | null
  onNodePointerDown: (nodeId: string, parentId: string, e: React.PointerEvent) => void
  onDropTargetChange: (target: DropTarget) => void
  onStartEdit: (nodeId: string) => void
  onCommitEdit: (nodeId: string, value: string) => void
  onCancelEdit: () => void
  onToggleState: (nodeId: string) => void
}

function EditableText({
  nodeId,
  value,
  isEditing,
  style,
  onStartEdit,
  onCommitEdit,
  onCancelEdit,
  canEdit,
}: {
  nodeId: string
  value: string
  isEditing: boolean
  style: React.CSSProperties
  onStartEdit: (id: string) => void
  onCommitEdit: (id: string, value: string) => void
  onCancelEdit: () => void
  canEdit: boolean
}) {
  if (isEditing) {
    return (
      <input
        autoFocus
        defaultValue={value}
        onBlur={(e) => onCommitEdit(nodeId, e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            onCommitEdit(nodeId, (e.target as HTMLInputElement).value)
            e.preventDefault()
          }
          if (e.key === 'Escape') {
            onCancelEdit()
            e.preventDefault()
          }
          e.stopPropagation()
        }}
        onPointerDown={(e) => e.stopPropagation()}
        onClick={(e) => e.stopPropagation()}
        style={{
          ...style,
          border: 'none',
          outline: '2px solid currentColor',
          outlineOffset: 1,
          borderRadius: 2,
          background: 'transparent',
          font: 'inherit',
          padding: 0,
          margin: 0,
          width: '100%',
          boxSizing: 'border-box',
        }}
      />
    )
  }

  return (
    <span
      onClick={
        canEdit
          ? (e) => {
              e.stopPropagation()
              onStartEdit(nodeId)
            }
          : undefined
      }
      style={{
        ...style,
        cursor: canEdit ? 'text' : 'default',
        minWidth: 20,
      }}
    >
      {value || '\u00A0'}
    </span>
  )
}

function DropIndicator({
  direction,
  theme,
}: {
  direction: 'horizontal' | 'vertical'
  theme: WireframeThemeColors
}) {
  return (
    <div
      style={{
        width: direction === 'horizontal' ? '100%' : 3,
        height: direction === 'horizontal' ? 3 : '100%',
        background: theme.accent,
        borderRadius: 2,
        flexShrink: 0,
        position: 'relative',
      }}
    >
      <div
        style={{
          position: 'absolute',
          width: 7,
          height: 7,
          borderRadius: '50%',
          background: theme.accent,
          ...(direction === 'horizontal'
            ? { left: -2, top: -2 }
            : { top: -2, left: -2 }),
        }}
      />
    </div>
  )
}

// --- Node type renderers ---

function FrameNodeRenderer({
  node,
  props,
}: {
  node: WireframeFrame
  props: WireframeNodeRendererProps
}) {
  const containerRef = useRef<HTMLDivElement>(null)
  const isVertical = node.direction !== 'horizontal'
  const indicatorDir = isVertical ? 'horizontal' : 'vertical'

  const lastDropRef = useRef<{ parentId: string; index: number } | null>(null)
  const handlePointerMove = (e: React.PointerEvent) => {
    if (!props.draggedNodeId) return
    const container = containerRef.current
    if (!container) return

    const childEls = Array.from(
      container.querySelectorAll(':scope > [data-wf-child]'),
    ) as HTMLElement[]
    const mousePos = isVertical ? e.clientY : e.clientX

    let insertIndex = 0
    for (let i = 0; i < childEls.length; i++) {
      const rect = childEls[i].getBoundingClientRect()
      const mid = isVertical
        ? rect.top + rect.height / 2
        : rect.left + rect.width / 2
      if (mousePos > mid) insertIndex = i + 1
    }

    const last = lastDropRef.current
    if (last && last.parentId === node.id && last.index === insertIndex) return
    lastDropRef.current = { parentId: node.id, index: insertIndex }
    props.onDropTargetChange({ parentId: node.id, index: insertIndex })
  }

  const frameStyle: React.CSSProperties = {
    display: 'flex',
    flexDirection: isVertical ? 'column' : 'row',
    gap: node.gap ?? 0,
    padding: parsePadding(node.padding),
    ...sizingToFlex(isVertical ? node.height : node.width),
    width: sizingToWidth(node.width),
    height: sizingToHeight(node.height),
    minWidth: 0,
    minHeight: 0,
    overflow: 'hidden',
  }

  // If either axis is 'fill', add the flex for that axis
  if (node.width === 'fill') {
    frameStyle.flex = frameStyle.flex ?? 1
    frameStyle.minWidth = 0
  }
  if (node.height === 'fill') {
    frameStyle.flex = frameStyle.flex ?? 1
    frameStyle.minHeight = 0
  }

  return (
    <div ref={containerRef} style={frameStyle} onPointerMove={handlePointerMove}>
      {node.children.map((child, index) => {
        const isDragged = props.draggedNodeId === child.id
        const showIndicator =
          props.dropTarget?.parentId === node.id &&
          props.dropTarget.index === index &&
          props.draggedNodeId !== null

        return (
          <div key={child.id} style={{ display: 'contents' }}>
            {showIndicator && (
              <DropIndicator direction={indicatorDir} theme={props.theme} />
            )}
            <div
              data-wf-child
              data-wf-child-id={child.id}
              style={{
                opacity: isDragged ? 0.25 : 1,
                transition: 'opacity 0.15s',
                display: child.type === 'frame' ? 'flex' : undefined,
                ...(child.type === 'spacer'
                  ? { flex: 1 }
                  : child.type === 'frame'
                    ? sizingToFlex(
                        isVertical ? child.width : child.height,
                      )
                    : {}),
              }}
              onPointerDown={(e) => {
                if (props.canEdit && !props.editingNodeId) {
                  e.stopPropagation()
                  props.onNodePointerDown(child.id, node.id, e)
                }
              }}
            >
              <WireframeNodeRenderer {...props} node={child} />
            </div>
          </div>
        )
      })}
      {props.dropTarget?.parentId === node.id &&
        props.dropTarget.index === node.children.length &&
        props.draggedNodeId !== null && (
          <DropIndicator direction={indicatorDir} theme={props.theme} />
        )}
    </div>
  )
}

function TextNodeRenderer({
  node,
  props,
}: {
  node: WireframeText
  props: WireframeNodeRendererProps
}) {
  const levelStyles: Record<string, React.CSSProperties> = {
    h1: { fontSize: 24, fontWeight: 700, lineHeight: 1.2 },
    h2: { fontSize: 20, fontWeight: 600, lineHeight: 1.3 },
    h3: { fontSize: 16, fontWeight: 600, lineHeight: 1.4 },
    body: { fontSize: 14, fontWeight: 400, lineHeight: 1.5 },
    caption: { fontSize: 12, fontWeight: 400, lineHeight: 1.4, opacity: 0.7 },
  }
  const style = levelStyles[node.level ?? 'body'] ?? levelStyles.body

  return (
    <EditableText
      nodeId={node.id}
      value={node.text}
      isEditing={props.editingNodeId === node.id}
      style={{ ...style, color: props.theme.text, fontFamily: 'system-ui, sans-serif' }}
      canEdit={props.canEdit}
      onStartEdit={props.onStartEdit}
      onCommitEdit={props.onCommitEdit}
      onCancelEdit={props.onCancelEdit}
    />
  )
}

function ButtonNodeRenderer({
  node,
  props,
}: {
  node: WireframeButton
  props: WireframeNodeRendererProps
}) {
  const { theme } = props
  const isPrimary = node.variant === 'primary'
  const isGhost = node.variant === 'ghost'

  const style: React.CSSProperties = {
    padding: '6px 14px',
    borderRadius: 6,
    fontSize: 13,
    fontWeight: 500,
    fontFamily: 'system-ui, sans-serif',
    cursor: 'default',
    border: isGhost ? 'none' : `1px solid ${isPrimary ? theme.accent : theme.border}`,
    background: isPrimary ? theme.accent : isGhost ? 'transparent' : theme.surface,
    color: isPrimary ? theme.accentText : theme.text,
    whiteSpace: 'nowrap' as const,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
  }

  return (
    <div style={style}>
      <EditableText
        nodeId={node.id}
        value={node.text}
        isEditing={props.editingNodeId === node.id}
        style={{ color: 'inherit', fontSize: 'inherit', fontWeight: 'inherit' }}
        canEdit={props.canEdit}
        onStartEdit={props.onStartEdit}
        onCommitEdit={props.onCommitEdit}
        onCancelEdit={props.onCancelEdit}
      />
    </div>
  )
}

function InputNodeRenderer({
  node,
  props,
}: {
  node: WireframeInput
  props: WireframeNodeRendererProps
}) {
  const { theme } = props

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {node.label && (
        <span
          style={{
            fontSize: 12,
            fontWeight: 500,
            color: theme.text,
            fontFamily: 'system-ui, sans-serif',
          }}
        >
          {node.label}
        </span>
      )}
      <div
        style={{
          padding: '7px 10px',
          borderRadius: 6,
          border: `1px solid ${theme.border}`,
          background: theme.inputBg,
          fontSize: 13,
          fontFamily: 'system-ui, sans-serif',
        }}
      >
        <EditableText
          nodeId={node.id}
          value={node.placeholder ?? ''}
          isEditing={props.editingNodeId === node.id}
          style={{ color: theme.textMuted, fontSize: 13 }}
          canEdit={props.canEdit}
          onStartEdit={props.onStartEdit}
          onCommitEdit={props.onCommitEdit}
          onCancelEdit={props.onCancelEdit}
        />
      </div>
    </div>
  )
}

function DropdownNodeRenderer({
  node,
  props,
}: {
  node: WireframeDropdown
  props: WireframeNodeRendererProps
}) {
  const { theme } = props

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {node.label && (
        <span
          style={{
            fontSize: 12,
            fontWeight: 500,
            color: theme.text,
            fontFamily: 'system-ui, sans-serif',
          }}
        >
          {node.label}
        </span>
      )}
      <div
        style={{
          padding: '7px 10px',
          borderRadius: 6,
          border: `1px solid ${theme.border}`,
          background: theme.inputBg,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 8,
          fontSize: 13,
          fontFamily: 'system-ui, sans-serif',
        }}
      >
        <EditableText
          nodeId={node.id}
          value={node.placeholder ?? 'Select...'}
          isEditing={props.editingNodeId === node.id}
          style={{ color: theme.textMuted, fontSize: 13, flex: 1 }}
          canEdit={props.canEdit}
          onStartEdit={props.onStartEdit}
          onCommitEdit={props.onCommitEdit}
          onCancelEdit={props.onCancelEdit}
        />
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke={theme.textMuted}
          strokeWidth="1.5"
          strokeLinecap="round"
        >
          <path d="M3 4.5L6 7.5L9 4.5" />
        </svg>
      </div>
    </div>
  )
}

function CheckboxNodeRenderer({
  node,
  props,
}: {
  node: WireframeCheckbox
  props: WireframeNodeRendererProps
}) {
  const { theme } = props

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        cursor: props.canEdit ? 'pointer' : 'default',
      }}
      onClick={
        props.canEdit && props.editingNodeId !== node.id
          ? (e) => {
              e.stopPropagation()
              props.onToggleState(node.id)
            }
          : undefined
      }
    >
      <div
        style={{
          width: 16,
          height: 16,
          borderRadius: 3,
          border: `1.5px solid ${node.checked ? theme.accent : theme.border}`,
          background: node.checked ? theme.accent : 'transparent',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        {node.checked && (
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
            fill="none"
            stroke={theme.accentText}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M2 5L4.5 7.5L8 3" />
          </svg>
        )}
      </div>
      <EditableText
        nodeId={node.id}
        value={node.label}
        isEditing={props.editingNodeId === node.id}
        style={{ fontSize: 13, color: theme.text, fontFamily: 'system-ui, sans-serif' }}
        canEdit={props.canEdit}
        onStartEdit={props.onStartEdit}
        onCommitEdit={props.onCommitEdit}
        onCancelEdit={props.onCancelEdit}
      />
    </div>
  )
}

function ToggleNodeRenderer({
  node,
  props,
}: {
  node: WireframeToggle
  props: WireframeNodeRendererProps
}) {
  const { theme } = props

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        cursor: props.canEdit ? 'pointer' : 'default',
      }}
      onClick={
        props.canEdit && props.editingNodeId !== node.id
          ? (e) => {
              e.stopPropagation()
              props.onToggleState(node.id)
            }
          : undefined
      }
    >
      <div
        style={{
          width: 32,
          height: 18,
          borderRadius: 9,
          background: node.on ? theme.accent : theme.border,
          position: 'relative',
          flexShrink: 0,
          transition: 'background 0.15s',
        }}
      >
        <div
          style={{
            width: 14,
            height: 14,
            borderRadius: '50%',
            background: node.on ? theme.accentText : theme.surface,
            position: 'absolute',
            top: 2,
            left: node.on ? 16 : 2,
            transition: 'left 0.15s',
          }}
        />
      </div>
      <EditableText
        nodeId={node.id}
        value={node.label}
        isEditing={props.editingNodeId === node.id}
        style={{ fontSize: 13, color: theme.text, fontFamily: 'system-ui, sans-serif' }}
        canEdit={props.canEdit}
        onStartEdit={props.onStartEdit}
        onCommitEdit={props.onCommitEdit}
        onCancelEdit={props.onCancelEdit}
      />
    </div>
  )
}

function ImageNodeRenderer({
  node,
  props,
}: {
  node: WireframeImage
  props: WireframeNodeRendererProps
}) {
  const { theme } = props

  return (
    <div
      style={{
        width: node.width ?? 200,
        height: node.height ?? 120,
        border: `1.5px dashed ${theme.border}`,
        borderRadius: 6,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 6,
        background: theme.surface,
        color: theme.textMuted,
      }}
    >
      <svg
        width="24"
        height="24"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="3" y="3" width="18" height="18" rx="2" />
        <circle cx="8.5" cy="8.5" r="1.5" />
        <path d="M21 15l-5-5L5 21" />
      </svg>
      {node.alt && (
        <span style={{ fontSize: 11, fontFamily: 'system-ui, sans-serif' }}>{node.alt}</span>
      )}
    </div>
  )
}

function DividerRenderer({ theme }: { theme: WireframeThemeColors }) {
  return (
    <div
      style={{
        width: '100%',
        height: 1,
        background: theme.border,
        flexShrink: 0,
      }}
    />
  )
}

function SpacerRenderer() {
  return <div style={{ flex: 1 }} />
}

// --- Main renderer ---

export function WireframeNodeRenderer(props: WireframeNodeRendererProps) {
  const { node, theme } = props

  // Defensive: a malformed children array (a JSON entry that's `null` or
  // shaped like `{}` without a `type`) used to crash the whole canvas here.
  // Render an inline marker so the rest of the wireframe still draws.
  if (!node || typeof node !== 'object' || typeof (node as { type?: unknown }).type !== 'string') {
    return (
      <span
        style={{
          display: 'inline-block',
          padding: '2px 6px',
          fontSize: 10,
          fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
          color: '#7f1d1d',
          background: '#fee2e2',
          border: '1px dashed #b91c1c',
          borderRadius: 4,
        }}
      >
        invalid wireframe node
      </span>
    )
  }

  switch (node.type) {
    case 'frame':
      return <FrameNodeRenderer node={node} props={props} />
    case 'text':
      return <TextNodeRenderer node={node} props={props} />
    case 'button':
      return <ButtonNodeRenderer node={node} props={props} />
    case 'input':
      return <InputNodeRenderer node={node} props={props} />
    case 'dropdown':
      return <DropdownNodeRenderer node={node} props={props} />
    case 'checkbox':
      return <CheckboxNodeRenderer node={node} props={props} />
    case 'toggle':
      return <ToggleNodeRenderer node={node} props={props} />
    case 'image':
      return <ImageNodeRenderer node={node} props={props} />
    case 'divider':
      return <DividerRenderer theme={theme} />
    case 'spacer':
      return <SpacerRenderer />
    default:
      return null
  }
}
