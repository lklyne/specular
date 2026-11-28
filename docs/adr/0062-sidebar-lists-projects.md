# 0062: The sidebar lists projects, and a repo is a project

**Status:** Accepted (option A), 2026-11-28, by Lyle. Built: see "As built".
**Date:** 2026-11-28
**Related:** [ADR 0033](./0033-user-chosen-space-folder.md) (one space per window, the folder is the space), [ADR 0043](./0043-per-canvas-state-in-a-space.md) (each canvas keeps its state in a Space), [ADR 0059](./0059-the-whole-window-from-mods.md) (the sidebar is a mod).
**Code today:** `specular-interact/src/space*`, `specular-interact/src/sidebar*`, `specular-interact/src/state/panels.rs` (`panels/sidebar`), `specular-mods/mods/sidebar/main.js`, `specular-agent/src/repos.rs`, `specular-shell/src/app/repos_run.rs`, `specular-shell/src/kit/settings_repos.rs`.

## Context

Two folder concepts sit side by side and never meet.

- **The space.** One folder per window (`spacePath` in the preferences). Its
  `.canvas` files sit flat inside it, ordered by `.specular/workspace-meta.json`.
  Every canvas is read when the space opens. The sidebar's Canvases section is
  this flat list, and the agent's working folder is this folder.
- **Repos.** A global list in `repos.json` in the app's data folder: a folder,
  a label, and the origins bound to it (`boundOrigins[].autoFix`). Its only job
  is to pick the agent's working folder when a turn is about a page on a bound
  origin. It is managed in Settings → Repos or from a page's folder icon.

So a person who works on a site has their canvases in one place and their code
in another, joined by an origin binding made in a settings page. The sidebar,
which is where people think about "what am I working on", knows nothing of the
repo. And because the space is flat, every canvas anyone makes, including the
ones agents make unasked, lands in one undifferentiated list.

## Options

### A. A project is a folder; the window lists several (recommended)

The sidebar's top level is **projects**. A project is a folder the person has
opened: a plain folder of canvases (today's space) or a code repo. Each project
row folds open to its canvases. Repos stop being a separate list: adding a repo
*is* opening a project.

```
▾ marketing-site          main ●      ← a repo; its origins live here
    Homepage v2
    Pricing explorations
▾ Design                              ← today's space
    Research
    Moodboard
▸ docs-site
＋ Open project…
```

- **Where canvases live.** In a plain folder, directly inside it, as today. In
  a repo, in `<repo>/.specular/canvases/` by default (one place to `.gitignore`
  or commit; never a scan of the whole tree). The per-project
  `.specular/workspace-meta.json` keeps its shape, so the Electron app and the
  current listing code read it unchanged.
- **Origins move onto the project.** `boundOrigins` move from `repos.json` to
  `<project>/.specular/project.json`. The project row's menu (or a popover on
  its head) lists them, adds one, and toggles auto-fix. A page whose origin is
  bound shows its project. The Settings → Repos page goes away.
- **The agent's working folder** is the project of the canvas the turn is on.
  The origin binding becomes a hint for a page that belongs to a *different*
  project than its canvas, instead of the only way to reach code at all.
- **Cost.** It undoes ADR 0033's "one space per window": `App` owns several
  Spaces, a tab ref can need a project to be unique, and the CLI and API grow
  a project argument. ADR 0043's "read every canvas at open" now multiplies by
  the projects listed; a folded project can stay unread until first opened.

### B. Projects are subfolders of the one space

Keep one space per window, and let it hold folders. The sidebar becomes a tree
of the space folder: top-level `.canvas` files, then each subfolder as a
project with its canvases. A project folder may *link* a repo elsewhere on
disk (`<space>/<project>/.specular/project.json` names the repo path and its
origins), which replaces Settings → Repos.

- **Cheapest.** No change to one-space-per-window, the CLI's addressing, or
  the agent's default working folder; migration is nothing (today's canvases
  are the top level).
- **But the split stays.** Canvases are still in one place and code in
  another, now joined by a link on a folder instead of a binding in settings.
  Canvases never travel with the repo, so a teammate cloning it sees none.

### C. Repos at the top, everything else under "Loose"

The sidebar lists repos; canvases are tagged with one (in the space's index)
and listed under it, with untagged canvases in a Loose section. Small to
build on today's data, but it makes the repo the only grouping, which suits
developers and not the designers in `docs/product.md` who have no repo.

## Prior art: Rex

Rex's sidebar is one flat level: a header per workspace ("Onboarding", a
repo name, a generated worktree name like `great-fermi`), and under each its
terminals, every row showing its own working folder. No nesting, no
separate repo settings. The workspace is a named group; the folder belongs
to each terminal, so one group can hold terminals in different folders.

Mapped to Specular, a header is a project and a row is a canvas. That is
option A's shape, drawn flat: headers, not a tree, which the sidebar mod's
`section()` already draws. Where Rex and A differ is where the folder sits.
In Rex it is per row; in A it is per project, because a canvas's files and
its agent's working folder should not disagree within one project. So A
shows the folder (and branch) once, on the header, not on every row. Rex's
throwaway worktree groups suggest one more thing worth keeping open: a
project should be cheap to add and remove from the list (closing one hides
it and deletes nothing), so a branch or worktree can come and go.

## Recommendation

**A**, built in three steps so each one ships on its own:

1. **Many projects, no repo change.** `App` holds a list of Spaces; the
   preferences' `spacePath` becomes `projects: [path…]` plus the active one,
   and today's space is the first project. The sidebar shows project rows
   that fold. A canvas belongs to the project whose folder holds it.
2. **Repos become projects.** Opening a folder that is a git repo makes a
   project with its canvases in `.specular/canvases/`. On first launch each
   `repos.json` entry becomes a listed project (folded, unread until opened)
   and its `boundOrigins` move to that project's `project.json`. `repos.json`
   is left in place for the Electron app and not written again.
3. **Settings → Repos goes.** Origin binding and auto-fix live on the project
   row; the page's folder icon picks a project instead of a repo.

B is the fallback if many Spaces in one `App` turns out to be too large a
change for now: it gets the folders into the sidebar and the repo page out of
Settings, and A can still follow (a B project folder is a valid A project).

## How mods and agents see it

The sidebar is a shipped mod, so the model is the API.

- **State.** `panels/sidebar` gains `projects: [{id, label, path, kind:
  "folder" | "repo", branch?, active, folded, origins: [{origin, autoFix}],
  canvases: [row…]}]`; the canvas rows keep today's shape (`id, canvas, label,
  active, count`). A new `projects` key gives the same list without controls
  for mods that are not the sidebar. `canvas/info` names the active project.
- **Commands.** `project.open` (asks for a folder), `project.close`,
  `project.reveal`, `project.bind-origin {project, origin}`,
  `project.unbind-origin`, `project.auto-fix {project, origin, on}`, and
  `canvas.new` / `canvas.move` take an optional `{project}`. They go in the
  command registry, so the palette, keys, `specular run` and mods get them
  together.
- **CLI.** `specular project` lists projects; a tab ref may be
  `<project>/<canvas>` when a bare name is ambiguous. `specular tab new`
  creates in the active project unless told otherwise, and the skill already
  says to run it only when the person asks.
- **Agents.** The run's working folder is the canvas's project, so "edit the
  code behind this page" needs no binding when the canvas lives in the repo.

## Migration

- A space opened today becomes a project with no file moved; its
  `.specular/workspace-meta.json` is read as it is.
- `repos.json` entries become folded projects (step 2), their origins copied
  to `project.json`. Nothing is deleted.
- An Electron install keeps working on the same folder: each project's index
  file has the shape it reads, and neither app removes unknown keys.

## Open questions

- In a repo, commit the canvases or ignore them by default? Committing makes
  them travel with the code; ignoring keeps pages' screenshots and assets out
  of diffs.
- One window with many projects, or a window per project? A assumes one
  window; a window per project is ADR 0033's growth direction and could follow.
- Does a project ever hold more than one folder (a monorepo's app plus its
  design folder)? A says no: one project, one folder.

## As built

All three steps, in one change, with these choices where the plan above
left room or was changed:

- **One `Space`, many projects.** `Space` keeps one list of canvases across
  every project, each canvas naming its `project`, so switching, parking
  and undo (ADR 0043) are unchanged. `Space::folder()` is the canvas folder
  of the active canvas's project: what relative paths, assets and threads
  start from. A switch into another project sends `Effect::ProjectEntered`
  first, and the shell re-roots its image and note loaders.
- **Ids.** A canvas keeps its index's id unless a canvas of another project
  had it first; then it is known as `<id>~<project>` and its index still
  writes the old id (`Canvas::tab`). Names are unique per project, and a
  tab ref may be `<project>/<name>`.
- **Bindings stay in `repos.json`.** Origins are not moved to a
  `project.json`: the sidebar shows a project's bindings by its folder, and
  the Electron app keeps reading the same file. Connecting or binding a
  folder opens it as a project.
- **The git choice is in the preferences** (`projectGit`) and written as
  `.specular/.gitignore`; until the person picks, nothing is written.
- **Not done:** folded projects are read in full at open (no lazy load),
  and a project is one window's; a window per project is still open.
- **Mods (API 1.7):** the `projects` key, a project section per project in
  `panels/sidebar` with its controls by id, the `project.*` commands and
  `canvas.new {project}`. The shipped sidebar mod draws Rex-style headers
  with a + and a ⋯ for details.
- **CLI and API:** `specular project [open <folder> | close <id|name>]`
  (`GET/POST /projects`, `POST /projects/close`) and `tab new --project`.

## Consequences

If A is taken: ADR 0033's one-space-per-window is superseded; ADR 0043 stands
per project; `CONTEXT.md` gains **Project** and retires **Space** as a user
word (it stays the type name, or becomes `Project` in code); Settings → Repos
and `settings_repos.rs` are deleted at step 3.
