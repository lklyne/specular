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

1. `specular workspace` — read the current canvas: pages, groups, edges, annotations.
2. `specular add page <url>` — pull a live site onto the canvas as a page.
3. `specular snapshot -i -f <pageId>` — get element refs for a page.
4. `specular click @<ref>` / `specular fill @<ref> "<text>"` — interact.
5. `specular snapshot -i -f <pageId>` — re-snapshot after DOM mutations (refs go stale).

## Read

| Command | Purpose |
|---|---|
| `specular workspace` | Print the canvas state as JSON Canvas (entities, edges, groups) |
| `specular selection` | Print the currently selected entities |
| `specular snapshot -i -c -f <id>` | Capture a compact interactive-elements snapshot of a page, with refs |
| `specular snapshot -i -s "<selector>" -f <id>` | Scope a snapshot to one section/container |
| `specular screenshot -f <id>` | Screenshot a page |

## Add — kind is the subcommand

```bash
specular add page <url> [--at x,y] [--preset N] [--landscape]
specular add note <text> [--at x,y] [--color 3]
specular add file <path>      # md / wireframe / html / image / video — kind inferred from extension
```

`add page` defaults to the Laptop preset; pass `--preset N` for another
breakpoint. `add note` auto-routes long / structured text to a `.md` note file;
short text stays a sticky note. `add file` infers the renderer from the
extension and sizes images / video from the file.

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
| `specular annotation <id>` | Get full detail for one annotation (elements, screenshot, replies) |
| `specular ack <id>` / `specular resolve <id>` / `specular dismiss <id>` | Respond to an annotation |
| `specular reply <id> "<text>"` | Reply on a thread |

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

Prefer progressive disclosure for noisy pages:

```bash
specular snapshot -i -c -f <pageId>                 # compact interactive refs
specular snapshot -i -c -s "#main" -f <pageId>      # scoped section
specular snapshot -i -c -d 3 -f <pageId>            # depth-limited
specular snapshot -i -u -c -f <pageId>              # include link hrefs only when needed
specular get text "#main" -f <pageId>               # read text without a full ref tree
specular read --outline -f <pageId>                 # passthrough: compact page outline
specular read --filter "pricing" -f <pageId>        # passthrough: filtered page read
```

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
page creation at breakpoints, reorganizing existing entities into a grid,
batch wireframe placement.

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

## Wireframes

Files ending in `.wireframe.json` render as interactive wireframe editors on
the canvas. Use them to sketch UI layouts, explore design variants, and iterate
spatially alongside live pages. Write the JSON file to disk, then add it:

```bash
specular add file /tmp/my-layout.wireframe.json
```

See [references/wireframes.md](references/wireframes.md) for the full node
schema, layout patterns, and examples.

## HTML pages

Drop a `.html` file onto the canvas to render it inline (charts, mockups,
generated visualizations): `specular add file /abs/path/viz.html`. Rendered
display-only; edit the file to update.

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
> If something behaves unexpectedly, re-test before trusting the list — and
> update it (remove the entry, or tighten its wording) when you confirm a
> change. A stale warning is worse than no warning.

When you encounter new gaps, append them to the tracking issue (see below).

- **`specular breakpoints <url>` may produce malformed page URLs** — confirm the new pages loaded the intended host before relying on them.
- **`specular link` does not validate entity ids** — self-edges and edges to nonexistent ids are accepted and stored. Confirm both endpoints exist before calling `link`.
- **Search box `fill` + `click` may not trigger navigation** — `fill` may not fire input events. If a click on Search fails, re-fill and retry, or click an autocomplete option ref instead.
- **`update <pageId> --url` lags `workspace`** — changing a page's URL navigates the page async, so the new URL isn't readable via `specular workspace` for a few hundred ms after the `updated` reply. Re-read (or brief wait) before relying on it in an `update → workspace` chain.
- **Google Sheets (and likely other canvas-rendered grids): no per-cell refs** — the grid is a single `<canvas>` element, not DOM cells, so snapshots can never target cells. Before driving Sheets, read [references/google-sheets.md](references/google-sheets.md) — it has the one write path that works (Name box → formula bar) and the focus traps that silently eat input while reporting "✓ Done".
- **Wrapped inline links can click dead space** — upstream agent-browser may click the center of a multi-line link's union bounding box, report success, and not navigate. If `find text ... click` or `click @ref` succeeds but URL/state does not change, assert with `wait --url` / `get url` and fall back to deterministic DOM navigation (`specular eval "location.href = document.querySelector('a[href*=...]').href" -f <pageId>`). Tracked in [lklyne/specular#324](https://github.com/lklyne/specular/issues/324).

## Tracking issue (localhost sessions)

When running against a local Specular app (`http://localhost:*`), you are working in a development context. If you hit a new CLI gap not listed above:

1. Add it to the list above in this skill file.
2. Append it as a comment to **[lklyne/specular#7](https://github.com/lklyne/specular/issues/7)** using:
   ```bash
   gh issue comment 7 --repo lklyne/specular --body "..."
   ```

Keep entries terse: one-line description, observed behavior, expected behavior. No need to comment for known issues already tracked.

When a limitation is fixed (confirmed by testing, not just by a closed issue), **remove it from the "Known CLI limitations" list above** and close or note it on the GitHub issue. A stale warning is worse than no warning.

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
