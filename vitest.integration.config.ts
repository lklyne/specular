import { defineConfig } from 'vitest/config'
import { resolve } from 'path'

export default defineConfig({
  resolve: {
    alias: {
      // The integration suite runs the real main-process runtime in plain
      // Node. Electron's API surface is replaced with inert fakes; see
      // tests/integration/electron-stub.ts.
      electron: resolve(__dirname, 'tests/integration/electron-stub.ts'),
    },
  },
  test: {
    include: ['tests/integration/**/*.test.ts'],
    testTimeout: 5_000,
    reporters: ['verbose'],
  },
})
