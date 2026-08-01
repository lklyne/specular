# ADR 0033 — The space folder is user-chosen, and one space is open per window

**Status:** Proposed
**Date:** 2026-07-31
**Related:** [ADR 0018 — Cloud sync and canvas sharing](./0018-cloud-sync-and-canvas-sharing.md) (store-an-id/resolve-a-location for assets; a chosen space root is the desktop half of that resolution); [ADR 0025 — Single workspace mutation seam](./0025-single-workspace-mutation-seam.md) (the seam this threads a resolved root through)

## Context

`docs/product.md` states the belief plainly: *"The file system is the data
model. A space is a folder. A canvas is a file."* `docs/file-formats.md`
elaborates the vault model, Obsidian-analogous, and then admits the gap in one
word — *"The location is **currently**:
`~/.config/Specular/workspaces/default/`"*.

That location is `app.getPath('userData')/workspaces/default/`. It is hard-coded,
undiscoverable, un-openable in Finder without knowing the incantation, outside
anything a user would back up or sync, and buried in a directory macOS hides by
default. The product promises the file system is the model while placing the
files where a user cannot reasonably participate in them. Every downstream
promise — diffable, versionable, editable by agents and other tools, no server
dependency — is technically true and practically inert, because nobody can find
the folder.

Nothing in the codebase configures this. There is no `electron-store`, no path
key in `preferences.json`, no picker. The single override is
`--user-data-dir=` (`src/main/index.ts:98-101`), a dev/test flag that relocates
*everything* — prefs, caches, repos — not just canvases.

Three questions have to be settled before code, because each has a wrong answer
that is expensive to walk back: what happens to existing files when the location
changes, what happens when a configured folder is absent at boot, and whether
"space" is singular.

## Decision

### 1. A space is a folder the user picks, and it is the folder — not a parent of one

`preferences.json` gains a `spacePath: string` key. When set, that path **is**
the space: `.canvas` files sit directly inside it, alongside `assets/`. The
`workspaces/<id>/` nesting does not follow the user into their own folder; it is
an artifact of sharing `userData` with unrelated app state, and a chosen folder
has no such neighbours.

All path construction funnels through one accessor — `spaceDir()` — which
returns `prefs.spacePath` when set and `userData/workspaces/default` when unset.
`DEFAULT_WORKSPACE_ID` survives only as the legacy fallback's path segment; it
stops being a concept.

**App metadata moves into `.specular/` inside the space.** `workspace-meta.json`
in the root of someone's `~/Documents/Design` reads as litter. A dot-directory is
the vault-model convention (`.obsidian/`, `.git/`) and keeps the promise that
what a user sees in the folder is *their* content.

### 2. One space per window; one window today

A window opens exactly one space. This is the constraint that keeps the change
small: no space switcher, no multi-root resolution, no ambiguity about which
space a CLI or HTTP call addresses. Additional windows — each with its own space
— are the anticipated growth direction, so `spaceDir()` is written as a resolver
that could take a window, not as a module-level singleton. Nothing else is built
for that future now.

Changing a window's space is a **reopen**, not a live re-point: the workspace
tears down and reloads from the new root. Mutating the root under a live Y.Doc,
open pages, and in-flight autosaves is the kind of state-crossing bug that is
cheap to prevent and miserable to diagnose.

### 3. Changing the location prompts about migration; it never guesses

When the user picks a new folder and the current space contains canvases, they
are asked, with three outcomes:

- **Move my canvases** — copy `.canvas` files, `assets/`, and `.specular/` to
  the new folder, verify the copy, then remove the originals. Copy-verify-delete,
  never a rename across volumes.
- **Start fresh here** — re-point only. The old folder is left untouched and its
  path is surfaced in the confirmation so the user can find it again.
- **Cancel.**

If the destination already contains `.canvas` files, that is not an error — it is
someone re-opening an existing space, and the migration question does not apply.
Merging two populated spaces is out of scope: offer to open the destination as-is
or cancel.

Silent migration is rejected. Moving a user's files without asking is
indistinguishable from losing them.

### 4. A missing space at boot prompts; it does not fall back

External drive unmounted, sync folder not yet materialized, folder renamed
outside the app. On boot, if `spacePath` is set but absent or unreadable, the app
prompts: **Locate…** (folder picker), **Open the default space**, or **Quit**.

Quietly falling back to the default location is the dangerous option: the user
gets an empty canvas, concludes their work is gone, and — worse — the app begins
autosaving into the fallback, so the real space and the session diverge. An
unavailable space is a question, not a recoverable condition.

### 5. Onboarding asks once, with a real default

The onboarding flow (`src/renderer/onboarding/`, today `'cli' | 'skill'`) gains a
space step: pick a folder, or accept the offered default of `~/Specular`. One
click to accept — a picker with no default is a wall in front of a first-run
user.

Existing installs are not migrated on upgrade and are not prompted. An unset
`spacePath` keeps resolving to the legacy location; those users change it from
Settings when they care to. Onboarding state is already `{ completed: boolean }`
(`src/main/runtime/preferences.ts:94-102`), so a completed user never sees the
new step.

### 6. Settings gets a General section

`src/renderer/settings/Sidebar.tsx` sections go from `'skills' | 'models' |
'repos'` to a set including `'general'`, which shows the current space path, a
**Change…** button (§3's flow), and **Reveal in Finder**. Reveal is the smallest
change here and possibly the highest-value one: it is the affordance that makes
the file-system-as-model claim true from inside the app.

### 7. One vocabulary, three levels: space, canvas, tab — "workspace" is retired

The word "workspace" currently means the folder level in code
(`workspace-persistence.ts`, `workspaceTabs`) and the document level in the UI
(the sidebar's "Workspaces" section lists `.canvas` files). Introducing "space"
in Settings and onboarding one panel away from a sidebar that says "Workspaces"
would ship both meanings to users simultaneously. This ADR settles the language:

- **Space** — the folder. Matches `docs/product.md` ("a space is a folder") and
  the vault model this ADR commits to: one per window, changed by reopen,
  defaulting to a single `~/Specular`.
- **Canvas** — the document. The sidebar section becomes **Canvases**; its
  actions become "Add canvas" / "Rename canvas" / "Delete canvas". Matches
  product.md ("a canvas is a file") and the `.canvas` extension users see in
  Finder, so the name is self-verifying against the disk.
- **Tab** — a canvas open in the app. Internal and CLI term only
  (`specular tab new/switch`); the sidebar presents tabs as canvases.
- **Workspace** — retired. It appears in no user-facing copy, and code
  converges during implementation: the folder-level modules rename
  `workspace-*` → `space-*` (`workspaceTabs` → `spaceTabs`, `workspaceDir()`
  already becomes `spaceDir()` per §1), and the CLI's `specular workspace` verb
  becomes `specular canvas` — it prints the current canvas, so the old name was
  wrong twice — with `workspace` kept as a hidden alias so existing agent
  skills don't break mid-transition. After implementation, "workspace" appears
  nowhere: space-level code says space, document-level UI says canvas,
  document-level code says tab.

The sidebar rename is copy-only and ships with this ADR. The identifier and
CLI renames land with the implementation, each as its own mechanical commit.

A refinement deliberately left open: the sidebar header could show the space
name itself (rows listed under it), doubling as the in-app affordance for
"this is a folder on your disk" and a natural home for Reveal in Finder. That
is sidebar design, not a decision that is expensive to reverse, so it is noted
and not decided.

## Consequences

`src/main/runtime/workspace-persistence.ts` is already fully parameterized on
`userDataPath` — it never calls `app.getPath` itself — so the change is
concentrated at the ~6 callers that pass it in (`workspace-autosave.ts:38,67`,
`workspace-tab-context.ts:295`, `workspace-tab-operations.ts:186,312,331`), each
of which switches to `spaceDir()`.

**`image-assets.ts:8` and `note-assets.ts:7` are the trap.** Both re-join
`workspaces/default` themselves rather than calling `workspaceDir()`. Change only
the persistence module and images and notes keep silently writing to the old
location while canvases move — a split-brain space that looks fine until an image
fails to resolve. Both must route through `spaceDir()` in the same change.

Writing outside `userData` means writing where the OS asks questions: on macOS,
`~/Documents` and `~/Desktop` are TCC-protected, and a sandboxed build would need
security-scoped bookmarks rather than a bare path string. The app is not
sandboxed today, so a path string suffices — but `spacePath` should be treated as
a value that may need to become a bookmark, not one that is definitionally a
string.

Per `src/main/runtime/CLAUDE.md`, anything touching `workspace-*.ts` needs
integration coverage: `tests/integration/persistence.test.ts` is the relevant
file, and `tab-file-identity.test.ts:106` constructs paths the same way and will
need updating.

`docs/file-formats.md:26-27` is corrected as part of this work. It currently
states a Linux path (`~/.config/Specular/…`) for what is primarily a macOS app,
where the real location is `~/Library/Application Support/Specular/…`.

## Alternatives considered

**Keep `workspaces/<id>/` nesting inside the chosen folder.** Smaller diff — the
existing layout carries over unchanged. Rejected because it makes the user's
chosen folder a container for a folder containing their work, which is the
vault model with a pointless floor between. The promise is "a space is a
folder", not "a space is a folder in a folder".

**Migrate everyone on upgrade to `~/Specular`.** Tempting: it retires the legacy
path immediately and leaves one code path. Rejected because it moves files
without being asked (§3) for users who never expressed a preference, and an
upgrade that relocates your work is a bad way to learn a feature exists.

**Multiple spaces open at once, tabs or a switcher.** Rejected as scope. It
forces every path resolution, CLI invocation, and HTTP route to carry a space
identity, and the one-per-window model gets most of the value with none of that.
Revisit when multi-window lands.

**Fall back to the default space when the configured one is missing.** Rejected
in §4 — the failure mode is silent divergence between the real space and the one
being written to.

**Keep "Workspaces" as the document noun, renaming the folder to "Project".**
"Project" is a genuinely good folder name for this audience — a designer's
space will often sit alongside a repo, and "choose where your project lives" is
the most natural onboarding sentence any option produces. Rejected because it
keeps the expensive half of the problem: "workspace" would still swap meaning
between all existing ADRs/history (where it means the whole open folder) and
new usage (a single document), the folder-level code rename is mandatory either
way, and product.md's "a space is a folder, a canvas is a file" would need
rewriting rather than converging on. It also inverts outside intuition — in
Slack, Notion, VS Code, and Figma, a workspace is the *largest* container, not
a single file. If the vault model later gives way to cheap folder-switching and
one-folder-per-repo usage, renaming *space* → *project* at the app level is a
clean swap that none of this ADR's decisions make more expensive.
