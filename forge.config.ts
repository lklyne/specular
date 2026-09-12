import { cpSync, existsSync, mkdirSync, readdirSync } from 'fs'
import path from 'path'
import type { ForgeConfig } from '@electron-forge/shared-types'
import { VitePlugin } from '@electron-forge/plugin-vite'

/**
 * The Claude Agent SDK cannot be bundled: it locates its bundled Claude Code
 * runtime and per-platform binary packages relative to its own real path on
 * disk. It is external in vite.main.config.ts, so the packaged app needs the
 * actual package (plus whichever platform binary packages pnpm installed)
 * present in node_modules. Dereference on copy — pnpm's layout is symlinks.
 */
function copyAgentSdkPackages(buildPath: string): void {
  const scopeSrc = path.resolve(__dirname, 'node_modules', '@anthropic-ai')
  const scopeDest = path.join(buildPath, 'node_modules', '@anthropic-ai')
  mkdirSync(scopeDest, { recursive: true })
  for (const name of readdirSync(scopeSrc)) {
    if (!name.startsWith('claude-agent-sdk')) continue
    const src = path.join(scopeSrc, name)
    if (!existsSync(src)) continue
    cpSync(src, path.join(scopeDest, name), { recursive: true, dereference: true })
  }
}
import { MakerDMG } from '@electron-forge/maker-dmg'
import { MakerZIP } from '@electron-forge/maker-zip'
import { PublisherGithub } from '@electron-forge/publisher-github'

const isSigning = Boolean(process.env.CSC_LINK)

const config: ForgeConfig = {
  packagerConfig: {
    appBundleId: 'com.lyleklyne.specular',
    name: 'Specular',
    icon: 'build/icon',
    extraResource: [
      'out/main/mcp-helper.js',
      'out/main/cli.js',
      'resources/specular-cli.sh',
      'resources/skills',
      'resources/bin',
    ],
    ignore: [],
    ...(isSigning && {
      osxSign: {},
      osxNotarize: {
        appleId: process.env.APPLE_ID!,
        appleIdPassword: process.env.APPLE_PASSWORD!,
        teamId: process.env.APPLE_TEAM_ID!,
      },
    }),
  },
  makers: [
    new MakerDMG({ icon: 'build/icon.icns' }),
    new MakerZIP({}, ['darwin']),
  ],
  publishers: [
    new PublisherGithub({
      repository: { owner: 'lklyne', name: 'specular' },
      prerelease: false,
      draft: false,
    }),
  ],
  hooks: {
    packageAfterCopy: async (_forgeConfig, buildPath) => {
      copyAgentSdkPackages(buildPath)
    },
  },
  plugins: [
    new VitePlugin({
      build: [
        {
          entry: 'src/main/index.ts',
          config: 'vite.main.config.ts',
          target: 'main',
        },
        {
          entry: 'src/preload/debug.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/toolbar.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/canvas-bg.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/left-sidebar.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/right-details-panel.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/devtools-resize-handle.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/page-content.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/onboarding.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
        {
          entry: 'src/preload/settings.ts',
          config: 'vite.preload.config.ts',
          target: 'preload',
        },
      ],
      renderer: [
        {
          name: 'main_window',
          config: 'vite.renderer.config.ts',
        },
      ],
    }),
  ],
}

export default config
