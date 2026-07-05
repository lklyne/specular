import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: ['tests/boot/**/*.test.ts'],
    globalSetup: ['tests/boot/global-setup.ts'],
    testTimeout: 15_000,
    hookTimeout: 30_000,
    fileParallelism: false,
    reporters: ['verbose'],
  },
})
