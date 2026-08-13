# Development

## Prerequisites

- Node.js >= 22
- [pnpm](https://pnpm.io/)

## Getting started

```bash
pnpm install
pnpm dev                     # start the Electron app
pnpm typecheck               # type-check both node and web tsconfigs
pnpm test:unit               # fast unit tests (no Electron)
pnpm test:integration        # real runtime in-process (no Electron)
pnpm test:boot               # smoke-boot with real Electron (pre-release; needs built app)
pnpm test:agent              # scripted agent scenarios (out-of-band, not CI)
pnpm build                   # package for distribution
```

See `tests/README.md` for the full test tier guide and what each bucket covers.

## Managing the dev app

`pnpm dev` runs the app in the foreground; Forge/Vite/Electron output goes to
that terminal. Run it in its own terminal or pane so it can be watched and
stopped independently of other work. Persistent app errors and performance
traces land in Electron's app logs directory.

## Environment variables

See [`.env.example`](../.env.example) for all available configuration options.

## Releasing

Releases are built, signed, notarized, and published automatically via GitHub Actions when a version tag is pushed.

### Setup (one-time)

Add these secrets to the GitHub repo settings (**Settings > Secrets > Actions**):

| Secret | Value |
|---|---|
| `APPLE_ID` | Apple Developer account email |
| `APPLE_PASSWORD` | An [app-specific password](https://support.apple.com/en-us/102654) for the Apple ID |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `CSC_LINK` | Base64-encoded `.p12` certificate |
| `CSC_KEY_PASSWORD` | Password used when exporting the `.p12` |

### Publishing a release

```bash
# Patch release (e.g. 0.2.2 -> 0.2.3)
pnpm release:patch

# Minor release (e.g. 0.2.2 -> 0.3.0)
pnpm release:minor
```

Must be run from a clean `main`. These run `npm version` (which commits and tags) then `git push --follow-tags`. The `release.yml` workflow picks up the `v*` tag, builds a signed+notarized macOS DMG, and publishes it as a GitHub Release. Users with the app installed receive the update automatically via [`update.electronjs.org`](https://update.electronjs.org) (powered by `update-electron-app`).

> **Do not use SemVer prerelease suffixes** (e.g. `0.2.1-alpha.3`). `update.electronjs.org` filters them out, so alpha/beta tags will never reach installed clients. Just increment patch/minor versions.
>
> Patch numbers can grow arbitrarily (`0.2.31`, `0.2.147`) — that's expected during active development. Bump the minor version when you want a "grouping" signal, not because the patch number got big.
