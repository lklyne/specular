# Bundled binaries

This directory holds third-party binaries shipped inside the Specular app
bundle and exposed to the main process via environment variables.

## `agent-browser`

A pinned release of [vercel-labs/agent-browser][ab] (a native Rust binary).

The binary is **not committed** — it's fetched at build time by
`scripts/fetch-agent-browser.sh`, which pins a `VERSION` + `SHA256` and drops
the matching `darwin-arm64` release asset into `resources/bin/agent-browser`.
`pnpm build` and `pnpm dist:mac` run it (via `pnpm fetch:agent-browser`) before
packaging. The fetched binary is gitignored.

### How it's resolved at runtime

`src/main/agent-browser-install.ts` calls `configureBundledAgentBrowser()`
during app startup, which sets `AGENT_BROWSER_PATH` to this binary so the
existing resolver in `src/main/shared/browse-handler.ts` picks it up
**before** walking `$PATH`. **Bundled wins.** A user-installed
`agent-browser` on `$PATH` is detected and surfaced in the Setup window for
visibility, but is not used by Specular by default.

This keeps Specular on a known-good agent-browser version that's tested
against its CLI command surface.

### How updates flow

agent-browser updates ship inside Specular app updates:

1. New agent-browser release is published upstream.
2. Bump `VERSION` + `SHA256` in `scripts/fetch-agent-browser.sh` (see below).
3. Bump Specular's version in `package.json` and publish.
4. `update-electron-app` (already in `package.json`) auto-downloads the
   Specular update on the user's machine.
5. After restart, the new binary is in the app bundle and `AGENT_BROWSER_PATH`
   picks it up automatically.

No separate update channel for agent-browser.

### Manual override

A user who needs a specific agent-browser version can set
`AGENT_BROWSER_PATH` in their environment before launching Specular. The
value Specular sets at startup respects an existing env var and won't
overwrite it.

### Updating the pinned binary

1. Pick the new tag from https://github.com/vercel-labs/agent-browser/releases.
2. In `scripts/fetch-agent-browser.sh`, set `VERSION` and delete the old
   `SHA256` (leave it wrong for now).
3. Run `pnpm fetch:agent-browser` — it downloads the new asset; the checksum
   check fails and prints the actual hash. Paste that into `SHA256`.
4. Re-run `pnpm fetch:agent-browser` to confirm it passes, then
   `./resources/bin/agent-browser --version`.
5. Commit the script change alongside any matching skill changes in
   `resources/skills/agent-browser/` (run `pnpm sync:agent-browser`).

This directory is wired into `forge.config.ts` `extraResource` so its
contents are copied into the packaged app's `Resources/bin/`.

[ab]: https://github.com/vercel-labs/agent-browser
