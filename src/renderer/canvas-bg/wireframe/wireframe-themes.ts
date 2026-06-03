import type { WireframeThemeName } from '../../../shared/wireframe/wireframe-types'

export interface WireframeThemeColors {
  bg: string
  surface: string
  border: string
  text: string
  textMuted: string
  accent: string
  accentText: string
  inputBg: string
}

export const wireframeThemes: Record<WireframeThemeName, WireframeThemeColors> = {
  light: {
    bg: '#ffffff',
    surface: '#f5f5f5',
    border: '#e0e0e0',
    text: '#1a1a1a',
    textMuted: '#999999',
    accent: '#2563eb',
    accentText: '#ffffff',
    inputBg: '#ffffff',
  },
  dark: {
    bg: '#18181b',
    surface: '#27272a',
    border: '#3f3f46',
    text: '#e4e4e7',
    textMuted: '#71717a',
    accent: '#3b82f6',
    accentText: '#ffffff',
    inputBg: '#27272a',
  },
  blueprint: {
    bg: '#0f2744',
    surface: '#163557',
    border: '#2a5a8a',
    text: '#c8ddf0',
    textMuted: '#6a9bc7',
    accent: '#4da6ff',
    accentText: '#0f2744',
    inputBg: '#163557',
  },
}
