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
