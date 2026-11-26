# ADR 0057: Work lands on `nightly`, and `main` only moves when a release ships

**Status:** Accepted by Lyle on 2026-11-26. The workflow and skill changes are in a separate PR. The branch rename, the default-branch switch and the protection rules are done in repository settings.
**Date:** 2026-11-26
**Related:** `CLAUDE.md` (Branches, Releases), `.claude/skills/release/SKILL.md`, `.github/workflows/release.yml`, `.github/workflows/nightly.yml`.

## Context

The Rust app grew on `rust-native` while `main` stayed on the last Electron release, v0.11.2. On 2026-11-26 `rust-native` was 421 commits ahead of `main` and 0 behind, and every recent PR targeted it. That makes `rust-native` the branch where work lands in practice, but nothing says so. `main` is still the default branch, so new PRs and agent branches start from the Electron app unless someone remembers to change the base.

We want to merge often into one branch so we can test it, and to move a separate, stable branch only when we release.

## Decision

**Two long-lived branches.**

- **`nightly`** is where work lands and the default branch on GitHub. Feature and fix PRs target it. Merging needs no review, so it can move many times a day. `rust-native` is renamed to `nightly`, which keeps its history and open PRs, and GitHub redirects the old name.
- **`main`** is stable. It always matches the newest release, and it only moves by a promotion PR or a hotfix. Release tags `vX.Y.Z` are only cut on `main`.

**Promotion.** To release:

1. On `nightly`, commit the changelog and the version bump (the `release` skill does this).
2. Open a PR from `nightly` into `main` titled `Release vX.Y.Z`. Merge it with **Create a merge commit**, never squash or rebase, so `main` stays an ancestor of `nightly` and the next promotion carries only new commits.
3. Tag the merge commit on `main` as `vX.Y.Z` and push the tag. `release.yml` builds, signs, notarizes and publishes, and refuses a tag that is not on `main`. Tags sit on merge commits that `nightly` doesn't contain, so find the last release with `git describe --tags --abbrev=0 origin/main`.

**Hotfixes.** Branch from `main`, PR into `main`, release as a patch from `main`, then merge `main` back into `nightly` so the fix is not lost at the next promotion.

**Nightly builds.** A scheduled workflow builds and signs `nightly` once a night when it has new commits, and keeps the DMG as a workflow artifact. It does not publish a GitHub release or an appcast. Sparkle follows the newest GitHub release, so publishing nightlies there would push them to every user.

**Protection.** `main` accepts changes only through PRs and can't be force-pushed or deleted. `nightly` can't be force-pushed or deleted. Neither one requires checks: Native and Server CI only run when their paths change, so a required check would block every docs-only PR forever. Merge when the CI that ran is green.

## Consequences

- New work starts from `nightly` by default, so it stops starting from the Electron app by mistake.
- The first promotion turns `main` into the Rust app. Electron installs on v0.11.2 update through Electron's feed, not Sparkle, so that release needs its own plan for moving them over.
- A promotion merge commit lives only on `main`, so GitHub shows `nightly` as "behind" by those merges. They carry no changes and never conflict.
- Clones that track `rust-native` have to be renamed by hand: `git branch -m rust-native nightly && git fetch && git branch -u origin/nightly`.
- PRs opened against `main` before this change are Electron-era work. They stay on `main` until each one is closed or rebuilt on `nightly`.
- Nobody tests a nightly build unless someone downloads it from the workflow run. If that's too much friction, the next step is a separate nightly Sparkle channel. It is not part of this decision.
