// Flat ESLint config. Minimal surface: enforces the two spec §6
// interaction-layer invariants via local custom rules. General lint
// coverage can be expanded later; this config exists first to lock
// down the layout-pass and pointer-event invariants.

import tseslint from 'typescript-eslint'
import localRules from './eslint-rules/index.js'

export default tseslint.config(
  {
    ignores: [
      '.vite/**',
      'out/**',
      'dist/**',
      'node_modules/**',
      '**/*.d.ts',
      'eslint-rules/**',
    ],
  },
  {
    files: ['**/*.{ts,tsx,js,jsx}'],
    languageOptions: {
      parser: tseslint.parser,
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    plugins: { local: localRules },
    rules: {
      // no-direct-view-mutation is 'error': the layout pass now owns every
      // view mutation (epic oqwsau8q completed the pass), so any new direct
      // setBounds/setVisible/addChildView/removeChildView outside
      // layout-engine / layer-stack is a real invariant break.
      'local/no-direct-view-mutation': 'error',
      'local/no-mouse-events': 'error',
      // Bridge functions passed directly as JSX event handlers serialize the
      // React event across the contextBridge (walks window; first touch of
      // window.speechSynthesis blocks the browser process ~800ms on macOS).
      'local/no-bridge-event-handlers': 'error',
    },
  },
)
