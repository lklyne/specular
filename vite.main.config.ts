import { defineConfig } from 'vite'
import { builtinModules } from 'module'

// https://electron-forge.io/config/plugins/vite/
export default defineConfig(() => {
  return {
    server: {
      watch: {
        ignored: ['**/*.md'],
      },
    },
    build: {
      rollupOptions: {
        external: [
          'electron',
          ...builtinModules,
          ...builtinModules.map((m) => `node:${m}`),
          'bufferutil',
          'utf-8-validate',
          // Ships its own Claude Code runtime as sibling files + per-platform
          // binary packages, all resolved relative to the module's real path —
          // inlining it breaks that. forge.config.ts copies the package into
          // the app's node_modules instead.
          '@anthropic-ai/claude-agent-sdk',
        ],
      },
    },
    resolve: {
      // Resolve bare specifiers to node_modules so bundled deps work.
      // Packages that should NOT be bundled are listed in `external` above.
      conditions: ['node'],
    },
  }
})
