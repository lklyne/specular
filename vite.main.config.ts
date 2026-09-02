import { defineConfig, loadEnv } from 'vite'
import { builtinModules } from 'module'

// https://electron-forge.io/config/plugins/vite/
export default defineConfig(({ mode }) => {
  // loadEnv pulls variables from `.env`, `.env.local`, etc. plus the shell —
  // shell wins over files. Passing `''` as the prefix (vs. Vite's default
  // `VITE_`) lets us expose bare names like `SENTRY_DSN`.
  const env = loadEnv(mode, process.cwd(), '')

  return {
    server: {
      watch: {
        ignored: ['**/*.md'],
      },
    },
    define: {
      // Baked at build time so distributed .dmg users get error reporting
      // without needing a runtime env var. Set SENTRY_DSN in specular/.env
      // for local dev or as a GitHub Actions secret for release builds.
      'import.meta.env.SENTRY_DSN': JSON.stringify(env.SENTRY_DSN ?? ''),
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
