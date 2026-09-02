---
name: specular
description: Drive Specular — a spatial canvas for iterating on web UI — from Claude Code. Use this skill whenever you need to pull a live website into a shared canvas, arrange pages at breakpoints, annotate pages, or inspect snapshots.
allowed-tools: Bash(specular:*)
---

# Specular

Specular is a spatial canvas that lets you pull live web pages onto a
freeform surface, view them at different breakpoints, and annotate them. All
canvas and page operations go through the `specular` command.

The surface is **verbs**: read the canvas, patch it with `add` / `update` /
`delete`, arrange and connect entities, drive live pages. Every verb compiles
to one internal patch and hits one apply path, so the CLI reads like agent-browser
while the mutation path stays singular (see [ADR 0019](../../../docs/adr/0019-canvas-as-document-cli.md)).

## Core workflow

1. `specular canvas` — read the current canvas: pages, groups, edges, annotations.
2. `specular add page <url>` — pull a live site onto the canvas as a page.
3. `specular snapshot -i -f <pageId>` — get element refs for a page.
4. `specular click @<ref>` / `specular fill @<ref> "<text>"` — interact.
5. `specular snapshot -i -f <pageId>` — re-snapshot after DOM mutations (refs go stale).

## Read

| Command | Purpose |
|---|---|
| `specular canvas` | Print the canvas state as JSON Canvas (entities, edges, groups) |
| `specular selection` | Print the currently selected entities |
| `specular snapshot -i -f <id>` | Capture an accessibility snapshot of a page, with refs |
| `specular screenshot -f <id>` | Screenshot a page |

## Targeting a tab

Every verb writes to the canvas the **user is looking at** unless you say
otherwise. Name the target instead of inheriting their focus:

```bash
specular canvas                           # appState.activeTab + appState.tabs (ids, names)
specular tab new "sync-roads"             # create a canvas; prints its id; does NOT switch focus
specular tab switch <tab-id|tab-name>     # the only command that moves the user's view
specular tab delete <tab-id|tab-name>     # remove a canvas; deleting a background one does NOT move the user
specular add note "…" --tab <tab-id>      # write to that canvas in the background
specular apply --tab <tab-id> < patch.json
```

`--tab` takes a tab id or an exact tab name; an ambiguous or unknown ref errors
with the candidates rather than guessing. Snapshot the tab id from `canvas`
before a batch and confirm it after — the user can switch canvases mid-session.

Not every verb takes it. `canvas` reads, `apply`, and `add` (including the
placement lookups `add` runs) are tab-scoped; the rest — `unlink`, `ungroup`,
`auto-layout`, `arrange`, the annotation verbs — reject `--tab` with a 400
rather than quietly writing to the active canvas. Selection-driven verbs like
`arrange` are active-tab-only by nature: selection is UI state, so only the
canvas the user is looking at has one.

Two limits: pages can't be created or edited on a background tab (they need a
live view — `tab switch` first), and background writes are not on the user's
undo stack, so Cmd+Z will not reverse them.

## Add — kind is the subcommand

```bash
specular add page <url> [--at x,y] [--preset N] [--landscape]
specular add note <text> [--at x,y] [--color 3]
specular add file <path>      # md / html / image / video — kind inferred from extension
```

`add page` defaults to the Laptop preset; pass `--preset N` for another
breakpoint. `add note` auto-routes long / structured text to a `.md` note file;
short text stays a sticky note. `add file` infers the renderer from the
extension and sizes images / video from the file.

## Where to write files — the space folder

`specular add file` links the file where it sits — nothing is copied. A file
written to a temp or scratch directory leaves the canvas pointing at a path
the user can't see and the OS may clean up. Before writing any file you
intend to `add` (an explainer `.md`, a generated `.html` visualization, an
image), ask the app where the user's space lives and write it there:

```bash
specular space
# → { "path": "/Users/…/Documents/Specular", "assetsPath": "/Users/…/Documents/Specular/assets" }
```

`.md` and `.html` files go directly in `path`, next to the user's `.canvas`
files; images and video belong in `assetsPath`. Temp directories are for
throwaway intermediates only — never for anything that lands on the canvas.

## Edit

```bash
specular update <id> [--at x,y] [--size w,h] [--preset N] [--text T] [--color C] [--url U] [--gap N]
specular delete <id> [id...]
```

`move` / `resize` fold into `update --at` / `--size` flags — there are no
separate verbs. Kind is resolved from the doc by id; you never pass it. The
registry decides which flags a kind honors (pages size via `--preset`, not
`--size`), so a flag a kind ignores is simply a no-op rather than a silent lie.
`update <groupId> --gap N` sets a managed auto-layout group's packing gap (px)
and reflows its children; it no-ops on unmanaged groups.

## Arrange & connect

| Command | Purpose |
|---|---|
| `specular arrange row\|column\|grid [id…] [--cols N]` | Tidy entities (ids, or current selection) in place: keep the footprint, even the gaps. Add `--gap m` to pack tight to a fixed gap instead |
| `specular group <id…>` | Group entities together |
| `specular ungroup <groupId>` | Dissolve a group |
| `specular auto-layout <id…> [--gap N]` | Make a managed auto-layout row or column from a selection (or convert a single group) — the mode follows the selection's dominant axis; children pack along it and can be drag-reordered; `--gap N` sets the packing gap (px) |
| `specular focus <id…>` | Scroll the viewport so the entity is centered |
| `specular link <a> <b> [--label <text>]` | Connect two entities with an edge |
| `specular unlink <edgeId…>` | Remove edges |
| `specular find-placement` | Find open canvas space for new entities |
| `specular breakpoints <url>` | Lay a URL out across device breakpoints |

## Comment

Annotations are a single concept (a comment thread) discriminated by **anchor
type** — `element` (DOM element on a page), `canvas` (a free canvas point),
`page` (anchored to a page in viewport coords), or `region` (a rectangle in
canvas space). There is no `--kind` flag on `specular annotate`; pass
`--page-id` to scope a comment to a page rather than the viewport.

| Command | Purpose |
|---|---|
| `specular annotate "<text>"` | Leave a comment (viewport anchor by default; `--page-id` for a page anchor) |
| `specular annotations` | List unresolved annotations (pending + acknowledged) |
| `specular annotations --status <s>` | Filter by status (`pending`, `acknowledged`, `resolved`, `dismissed`) |
| `specular annotations --all` | Include resolved + dismissed too |
| `specular annotation <id>` | Get full detail for one annotation (selection contents, elements, screenshot, replies) |
| `specular ack <id>` / `specular resolve <id>` / `specular dismiss <id>` | Respond to an annotation |
| `specular reply <id> "<text>"` | Reply on a thread |
| `specular annotate-selection "<text>" [--ids id1,id2]` | One region comment over a multi-selection's union bounds; omit `--ids` to use the current selection |

**Selection annotations.** `specular annotation <id>` is the whole context
bundle for a selection comment — one call, no canvas read, no filtering by
id. Alongside the comment text it carries:

- `selection.members` — every selected entity resolved: a text note's `text`,
  a page's `url` and `pageName`, a file's `filePath`, a group's `label`, plus
  `bounds`. A selected group expands to its descendants.
- `selection.priorFeedback` — unresolved comments already sitting on those
  same items, so you don't re-litigate feedback the user already left.
- `metadata.selectionTarget` — the one page or file entity the request is
  about, present only when the selection names exactly one (omitted for
  selections spanning several artifacts).

The command also writes the region's screenshot to a temp PNG and prints the
path — read it to see what the user was looking at. It captures the visible
part of the region, so a selection larger than the window comes back with
canvas-background fill where the offscreen part was.

**Results belong on the canvas.** The canvas is the surface the user works
on, so anything you create while acting on a comment — a new route, a
duplicated prototype, a variant — is invisible until it is placed there
(`specular find-placement` for a free spot, then `specular add page <url>`
or `specular add file <path> --at x,y`). Pages already on the canvas reload
themselves, so an in-place source edit needs nothing extra.

Files in the user's space folder have no version control behind them; the
repo bound in the Comments panel does. Weigh that when a request is
ambiguous about whether it wants the original changed or a copy to compare
against.

## Drive a page

Browse verbs need a target page. There is no persistent "active page" binding —
pass `-f <pageId>` on every browse call:

```bash
specular snapshot -i -f <pageId>
specular click @e3 -f <pageId>
specular fill @e7 "search term" -f <pageId>
```

| Command | Purpose |
|---|---|
| `specular click @<ref>` | Click an element by ref |
| `specular fill @<ref> "<text>"` | Fill a form field |
| `specular type @<ref> "<text>"` | Type into an element |
| `specular select @<ref> "<value>"` | Select a dropdown option |
| `specular scroll <direction> [amount]` | Scroll the page |
| `specular back` / `specular forward` / `specular reload` | Browser history navigation |
| `specular wait [--load <state>] [--timeout N]` | Wait for the page |

`specular focus <id>` only scrolls the canvas viewport — it does not set the
active page.

## Targeting & stale refs

`@refs` from `specular snapshot -i` are assigned fresh per snapshot and die on
ANY DOM change — hot reload included. After any source edit, re-snapshot
before using refs.

For iteration loops prefer re-resolving targets, which resolve fresh on every
call:

```bash
specular click "#submit" -f <pageId>
specular find text "Submit" click -f <pageId>
specular find role button click --name "Submit" -f <pageId>
```

Wait for expected state instead of sleeping:

```bash
specular wait --text "Order confirmed" -f <pageId>
specular wait --url "**/checkout" -f <pageId>
```

If a ref-based action fails after the page changed, the error says refs may
be stale — re-snapshot or switch to a selector.

## Fallback — the JSON door

`specular apply` is the one declarative door, for the genuinely batch case
("create 6 pages in a 3×2 grid") where verbs would be six calls. A **patch** is
the single shape every verb compiles to:

```jsonc
{
  "entities": [ {"kind":"page","url":"…"}, {"id":"text_7","text":"updated"} ],
  "edges":    [ {"fromEntityId":"a","toEntityId":"b"} ],
  "delete":   ["text_3", "edge_9"],
  "layout":   {"kind":"grid","cols":3,"gap":"m","near":"page_1"}
}
```

No `id` → create. `id` present → update. `id` in `delete` → remove. Applied in
one transaction.

See [references/apply.md](references/apply.md) for worked examples — batch
page creation at breakpoints, reorganizing existing entities into a grid.

The `layout` directive takes a `kind` (`row` / `column` / `grid`), a `gap`
(token or pixel number), and an anchor (`originX`/`originY`, `near: <id>`, or
implicit). It places new entities *and* reorganizes existing ones. Without
`originX/Y` or `near`, it anchors at the bounding box of any existing entities
in the patch; with none, it falls back to `find-placement`.

**Spacing scale.** Tokens align to the canvas grid (20px multiples). Numbers
work too as an escape hatch:

| Token | Pixels |
|---|---|
| `xs` | 20 |
| `s` | 40 |
| `m` | 60 |
| `l` | 100 |
| `xl` | 160 |

`drawing` and `shape` have no ergonomic `add` verb (they're interactive-creation
kinds) — create them via `apply` with a patch.

> `specular upsert --json` is the legacy batch door (a bare items array or
> `{layout, items}`). Prefer `apply`; `upsert` is retained for older callers.

## Note colors

`--color` on text notes (and group labels) expects a **JSON Canvas preset id
`"1"`–`"6"`** or a hex string. CSS color names like `"yellow"` or `"red"` are
NOT presets — they silently fall through and render as raw CSS, which clashes
hard with the canvas palette.

| id | label | hex |
|---|---|---|
| `"1"` | Red | `#e8b4b8` |
| `"2"` | Orange | `#e8ccb0` |
| `"3"` | Yellow | `#FFE18E` |
| `"4"` | Green | `#b8d8c8` |
| `"5"` | Cyan | `#b0d0d8` |
| `"6"` | Purple | `#c8b8d8` |

Use `"3"` not `"yellow"`. Hex (`"#FFE18E"`) is also valid when you need a
custom tone.

## Note sizes

Default to the built-in 200×200 sticky-note size — it's tuned to sit well next
to pages on the canvas. Only pass an explicit `--size` when there's a real
reason (e.g. a long-form card that needs more room). Custom sizes tend to look
off against the rest of the workspace.


## HTML pages

Drop a `.html` file onto the canvas to render it inline (charts, mockups,
generated visualizations): `specular add file /abs/path/viz.html`. Rendered
display-only; edit the file to update. Write the file into the space folder
(`specular space`) first — see "Where to write files".

## Passing URLs

Always pass full URLs (including scheme and host) to `specular add page`. The
canvas can contain pages from different origins, so bare paths like `/garden`
are ambiguous. Use `http://localhost:4321/garden`, not `/garden`.

## Chaining

Commands can be chained with `&&` for atomic sequences:

```bash
specular add page http://localhost:3000 && specular snapshot -i -f <pageId>
```

## Known CLI limitations

> **Treat this list as known assumptions, not ground truth.** Entries reflect
> behavior observed at the time they were added. Codepaths change, and some
> items here may already be fixed or may present differently in your session.
> If something behaves unexpectedly, re-test before trusting the list — a
> stale warning is worse than no warning.

- **`specular breakpoints <url>` may produce malformed page URLs** — confirm the new pages loaded the intended host before relying on them.
- **`specular link` does not validate entity ids** — self-edges and edges to nonexistent ids are accepted and stored. Confirm both endpoints exist before calling `link`.
- **Search box `fill` + `click` may not trigger navigation** — `fill` may not fire input events. If a click on Search fails, re-fill and retry, or click an autocomplete option ref instead.
- **`update <pageId> --url` lags `canvas`** — changing a page's URL navigates the page async, so the new URL isn't readable via `specular canvas` for a few hundred ms after the `updated` reply. Re-read (or brief wait) before relying on it in an `update → canvas` chain.
- **Google Sheets (and likely other canvas-rendered grids): no per-cell refs** — the grid is a single `<canvas>` element, not DOM cells, so snapshots can never target cells. Before driving Sheets, read [references/google-sheets.md](references/google-sheets.md) — it has the one write path that works (Name box → formula bar) and the focus traps that silently eat input while reporting "✓ Done".

## Passthrough to agent-browser

Unrecognized browse verbs forward to the bundled agent-browser driver —
useful examples: `eval`, `keyboard inserttext`, `focus`, `clipboard`, `find`.

ALWAYS run them via `specular <verb> ... -f <pageId>` — never by running
`agent-browser` directly. Calling it directly drives a separate browser
disconnected from the canvas.

For the deep command reference, run `specular skills get core` — it prints
the upstream driver's full command docs. Read it as: every verb there runs
as `specular <verb> -f <pageId>`; ignore the session/launch/open lifecycle
commands in that reference — Specular owns the browser lifecycle.

Lifecycle verbs (`launch`, `close`, `quit`, `install`, `upgrade`, `connect`)
are blocked. Use the Specular equivalents instead: `specular delete <id>` to
close a page, `specular add page <url>` to open one.
