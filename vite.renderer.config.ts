import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'

// https://electron-forge.io/config/plugins/vite/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    watch: {
      ignored: ['**/*.md'],
    },
  },
  build: {
    rollupOptions: {
      input: {
        toolbar: resolve(__dirname, 'src/renderer/toolbar/index.html'),
        'canvas-bg': resolve(__dirname, 'src/renderer/canvas-bg/index.html'),
        'above-view': resolve(__dirname, 'src/renderer/above-view/index.html'),
        'agent-layer': resolve(__dirname, 'src/renderer/agent-layer/index.html'),
        'left-sidebar': resolve(__dirname, 'src/renderer/left-sidebar/index.html'),
        'devtools-resize-handle': resolve(__dirname, 'src/renderer/devtools-resize-handle/index.html'),
        'right-details-panel': resolve(__dirname, 'src/renderer/right-details-panel/index.html'),
        onboarding: resolve(__dirname, 'src/renderer/onboarding/index.html'),
        settings: resolve(__dirname, 'src/renderer/settings/index.html'),
        debug: resolve(__dirname, 'src/renderer/debug/index.html'),
      },
    },
  },
})
