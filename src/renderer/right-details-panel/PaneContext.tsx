import { createContext, useContext, type ReactNode } from 'react'

const PaneThemeCtx = createContext(false)

export function PaneProvider({ isDark, children }: { isDark: boolean; children: ReactNode }) {
  return <PaneThemeCtx.Provider value={isDark}>{children}</PaneThemeCtx.Provider>
}

export function usePaneTheme(): boolean {
  return useContext(PaneThemeCtx)
}
