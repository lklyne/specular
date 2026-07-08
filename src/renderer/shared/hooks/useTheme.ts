import { useEffect, useState } from 'react'
import type { ThemeData } from '../../../shared/types'

export function useTheme(
  initialTheme: ThemeData,
  onThemeChanged: (callback: (data: ThemeData) => void) => () => void
): ThemeData {
  const [theme, setTheme] = useState<ThemeData>(() => {
    document.documentElement.classList.toggle('dark', initialTheme.isDark)
    return initialTheme
  })

  useEffect(() => {
    const cleanup = onThemeChanged((data) => {
      setTheme(data)
      document.documentElement.classList.toggle('dark', data.isDark)
    })
    return cleanup
  }, [onThemeChanged])

  return theme
}
