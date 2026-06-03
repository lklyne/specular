// Schema types for .wireframe.json files

export type WireframeThemeName = 'light' | 'dark' | 'blueprint'
export type WireframeSizing = number | 'fill' | 'hug'

export interface WireframeFile {
  version: '1.0'
  theme?: WireframeThemeName
  root: WireframeNode
}

export type WireframeNode =
  | WireframeFrame
  | WireframeText
  | WireframeButton
  | WireframeInput
  | WireframeDropdown
  | WireframeCheckbox
  | WireframeToggle
  | WireframeImage
  | WireframeDivider
  | WireframeSpacer

export interface WireframeFrame {
  id: string
  type: 'frame'
  direction?: 'horizontal' | 'vertical'
  gap?: number
  padding?: number | [number, number] | [number, number, number, number]
  width?: WireframeSizing
  height?: WireframeSizing
  children: WireframeNode[]
}

export interface WireframeText {
  id: string
  type: 'text'
  text: string
  level?: 'h1' | 'h2' | 'h3' | 'body' | 'caption'
}

export interface WireframeButton {
  id: string
  type: 'button'
  text: string
  variant?: 'primary' | 'secondary' | 'ghost'
}

export interface WireframeInput {
  id: string
  type: 'input'
  placeholder?: string
  label?: string
}

export interface WireframeDropdown {
  id: string
  type: 'dropdown'
  placeholder?: string
  options: string[]
  label?: string
}

export interface WireframeCheckbox {
  id: string
  type: 'checkbox'
  label: string
  checked?: boolean
}

export interface WireframeToggle {
  id: string
  type: 'toggle'
  label: string
  on?: boolean
}

export interface WireframeImage {
  id: string
  type: 'image'
  width?: number
  height?: number
  alt?: string
}

export interface WireframeDivider {
  id: string
  type: 'divider'
}

export interface WireframeSpacer {
  id: string
  type: 'spacer'
}

export interface DropTarget {
  parentId: string
  index: number
}
