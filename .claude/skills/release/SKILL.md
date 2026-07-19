---
name: release
description: Cut a new Specular release. Bumps the version, updates CHANGELOG.md, tags, pushes, then builds and publishes locally. Use when the user says "cut a release", "ship a release", "release v0.2.6", "bump the version", or wants to update the changelog and tag.
allowed-tools: Bash(git *), Bash(pnpm *), Bash(npm version*), Bash(bash scripts/release-local.sh), Edit, Write, Read, AskUserQuestion
---

# Release Skill

Cuts a new release of Specular. Updates the changelog, bumps the version, tags, pushes, then builds and publishes the macOS app locally via `scripts/release-local.sh`. There is no GitHub-hosted release job — the repo is private, and macOS runners bill at 10x Actions minutes, so the build/sign/notarize/publish step runs on your own Mac.

## Guiding principles

- **The changelog is user-facing.** Commits are developer-facing. Translate, group, and trim — don't dump commit messages.
- **Ask before writing.** Version bump and changelog wording are the two decisions that matter. Never guess silently.
- **One commit per release.** The version bump commit includes the staged CHANGELOG. `npm version` handles this for us.
- **No prereleases.** SemVer prerelease suffixes (`-alpha.N`, `-beta.N`) are a footgun with `update.electronjs.org`. Patch/minor/major only.

## Workflow

### Step 1: Preflight

Stop and report to the user if any of these fail:

- Working tree is clean (`git status --porcelain` empty)
- On `main` (or user explicitly said to release from another branch)
- Up to date with `origin/main` (`git fetch` then compare)

If any check fails, explain what's off and stop — don't try to fix it silently.

### Step 2: Gather context

Collect in parallel:

- Current version: `node -p "require('./package.json').version"`
- Last tag: `git describe --tags --abbrev=0`
- Commits since last tag: `git log <last-tag>..HEAD --oneline`
- Full commit bodies for those commits: `git log <last-tag>..HEAD --format="%h%n%s%n%b%n---"`
- Existing `## [Unreleased]` section from `changelog.md`
- Bundled agent-browser freshness: `pnpm check:agent-browser`

If there are zero commits since the last tag, stop — nothing to release.

If `check:agent-browser` reports we're behind, surface it: show the pinned →
latest versions and the release notes it printed, and ask the user whether to
bump the pin as part of this release (it's a judgment call — patch-level upstream
fixes are usually worth it; skip if the notes look risky or unrelated). To bump:
set `VERSION` in `scripts/fetch-agent-browser.sh`, run `pnpm fetch:agent-browser`
to get the new `SHA256`, paste it back, and commit alongside the changelog. Don't
bump silently and don't block the release on it.

### Step 3: Infer version bump, ask user to confirm

Infer the bump from commits — don't present it as an open question. Read the actual changes, not just prefixes:

- Breaking change to data model, file format, IPC surface, CLI, or HTTP API → **major**
- New feature, new entity type, new surface, or meaningful capability → **minor**
- Fixes, polish, internal changes, docs → **patch**

Prefixes (`feat:`, `fix:`, `BREAKING CHANGE:`, `!`) are hints, not rules — weight them against what the diffs actually do.

Use **AskUserQuestion** framed as confirmation of your inference, not as a menu. State the bump you inferred, the one-line reason, and offer options: `confirm <inferred>`, `<other bump>`, `<other bump>`. The inferred option is the default.

### Step 4: Draft the changelog entry

Every release gets a **title** — a short headline that captures what's in this release. Follow Conductor's pattern: descriptive, title-cased, comma-separated when multiple themes.

- Feature release: name the features. `Big Terminal Mode`, `File Previews, Codex Personalities`
- Fix-heavy release: say so. `Bug Fixes`, `Stability Fixes and Polish`
- Milestone/holiday: let personality in. `Conductor Wrapped`, `Happy Valentine's Day`
- Mixed: list the top 1–3 themes. `Bug Fixes, Instant Archiving, @terminal`

Then build the body — combine:

1. Everything already under `## [Unreleased]` in `changelog.md`
2. User-facing summaries of commits since the last tag (deduped against the Unreleased items — many will already be covered)

The target voice is **Conductor's changelog** (`conductor.build/changelog`): punchy, conversational, user-facing, light on ceremony. Not strict Keep a Changelog — warmer than that.

Style rules:

- **Voice**: write to the user like you're telling a friend what changed. Active, concrete, a little personality. Follow the user's UI copy voice memory — lowercase gerunds, no corporate stiffness.
- **Grouping**: use informal headings that fit the release. Common ones: `Improvements`, `Fixes`, `Misc`. Use `New` for headline features. Don't force strict `Added/Changed/Fixed` if the release doesn't fit that shape. Omit empty sections.
- **Length**: most bullets are one short sentence. Headline features can get a 1–2 sentence lede before a bullet list.
- **Merge related commits**: multiple commits around one thing become one bullet.
- **Describe the outcome**, not the diff. "Agent cursors fade out gracefully when retired" beats "refactor retirement animation timing."
- **Drop internal-only noise**: `chore`, dep bumps without user impact, pure refactors, test-only changes, internal docs. If the user wouldn't notice it, it doesn't belong.
- **Personality is welcome** when a release genuinely calls for it (holidays, milestones). Don't force it — a terse release should stay terse.

### Step 5: Confirm the draft

Show the user the full drafted section — title + body, formatted as it will appear in `changelog.md`. Use **AskUserQuestion** with options:

- `approve` — write it as shown
- `edit title` — user wants to change the title; iterate
- `edit body` — user wants to change entries; iterate
- `cancel` — stop, make no changes

Iterate until the user approves. Respect their wording exactly — don't "improve" it.

### Step 6: Write changelog

Compute the new version: apply the chosen bump to the current version (e.g., `0.2.5` + patch → `0.2.6`).

Edit `changelog.md`:

- Replace the `## [Unreleased]` heading (and its contents) with:
  ```
  ## [Unreleased]

  ## [X.Y.Z] - YYYY-MM-DD — <Title>

  <approved content>
  ```
- Use today's date from the environment (`currentDate`).
- Leave the rest of the file untouched.

### Step 7: Commit changelog, bump, tag

npm 10+ refuses to run `npm version` when the tree has any porcelain output — staged files count as dirty, so the changelog has to land in its own commit before the bump.

1. Commit the changelog on its own:
   ```
   git add CHANGELOG.md
   git commit -m "docs: changelog for X.Y.Z"
   ```
   (Git stores the file as `CHANGELOG.md` on case-insensitive macOS filesystems even though the path is `changelog.md` — either works for `git add`.)
2. Run the matching release script on a clean tree:
   - patch → `pnpm release:patch`
   - minor → `pnpm release:minor`
   - major → no script exists; run `npm version major && git push --follow-tags` directly, and tell the user you're adding a `release:major` script next time they're in `package.json` (don't modify it now)

`npm version` will:
- Bump `package.json`
- Create a `chore: bump version to X.Y.Z`-style commit (package.json only)
- Create an annotated tag `vX.Y.Z`
- (The `release:*` scripts then push with `--follow-tags`)

Result: two commits land (`docs: changelog ...` then the version bump), with the tag on the bump commit. If `npm version` still errors about a dirty tree, stop and investigate — something unexpected is staged or modified.

### Step 8: Build and publish locally

There is no CI release job — build and publish from the local Mac:

```
bash scripts/release-local.sh
```

The script checks it's on a tag, then builds, signs, notarizes, uploads to GitHub Releases, and populates the release notes from `changelog.md`. It needs these env vars set (see the script header): `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`, `GITHUB_TOKEN` (PAT with `repo` scope), and optionally `SENTRY_DSN`.

This only works on macOS with the Developer ID signing cert in the login keychain. If the skill is running somewhere without those (e.g. a Linux agent session), stop after the push and tell the user to run `scripts/release-local.sh` on their Mac.

### Step 9: Report

Tell the user:

- New version and tag (e.g., `v0.2.6`)
- Whether the local build/publish ran, or that they need to run `scripts/release-local.sh` on their Mac
- Link to the release: `https://github.com/lklyne/specular/releases`
- Reminder to check the GitHub Release and flip "prerelease" off if needed before publishing

## Notes

- Never use `--no-verify` or `-f` on `npm version`. If it fails, the tree is dirty for a reason — investigate.
- The `update.electronjs.org` caveat is in `changelog.md` line 5. If a user asks for an alpha release, point them there and confirm they really want to do it before proceeding.
- `changelog.md` and `CHANGELOG.md` resolve to the same file on macOS (case-insensitive FS). Use `changelog.md` — that's what exists on disk.
- If the user invokes this skill mid-session with uncommitted work, offer to stash it before preflight fails, but don't auto-stash.
