---
name: specular
description: Drive Specular — a spatial canvas for iterating on web UI — from Claude Code. Use this skill whenever you need to pull a live website into a shared canvas, arrange pages at breakpoints, annotate pages, or inspect snapshots.
---

# Specular

Specular is a spatial canvas that lets you pull live web pages onto a
freeform surface, view them at different breakpoints, and annotate them. All
canvas and page operations go through the `specular` command.

## Core workflow

1. `specular workspace` — list the current canvas: pages, groups, edges, annotations.
2. `specular create page <url>` — pull a live site onto the canvas as a page.
3. `specular snapshot -i` — get element refs for the selected page (or `-f <pageId>`).
4. `specular click @<ref>` / `specular fill @<ref> "<text>"` — interact.
5. `specular snapshot -i` — re-snapshot after DOM mutations (refs go stale).

## Common commands

| Command | Purpose |
|---|---|
| `specular workspace` | Print the current canvas state as JSON |
| `specular selection` | Print the currently selected entities |
| `specular create page <url>` | Add a live page to the canvas |
| `specular create note <text>` | Add a text note to the canvas |
| `specular upsert --json < items.json` | Batch create/update entities (pages, notes, files) |
| `specular update <id> …` | Update properties on an existing entity |
| `specular delete <id>` | Remove an entity |
| `specular focus <id>` | Scroll the viewport so the entity is centered |
| `specular find-placement` | Find open canvas space for new entities |
| `specular link <a> <b>` | Connect two pages with an edge |
| `specular group <id…>` | Group entities together |
| `specular auto-layout <id…>` | Make a managed auto-layout row from a selection (or convert a single group); children pack left-to-right and can be drag-reordered |
| `specular breakpoints <id>` | Cycle through device breakpoints for a page |
| `specular annotate "<text>"` | Leave a comment on the canvas (anchor type from context — viewport by default; `--page-id` for a page anchor) |
| `specular annotations` | List unresolved annotations (pending + acknowledged) |
| `specular annotations --status <s>` | Filter by specific status (`pending`, `acknowledged`, `resolved`, `dismissed`) |
| `specular annotations --all` | Include resolved + dismissed too |
| `specular annotation <id>` | Get full detail for one annotation (elements, screenshot, replies) |
| `specular ack <id>` / `specular resolve <id>` | Respond to an annotation |

Annotations are a single concept (a comment thread) discriminated by **anchor type** — `element` (DOM element on a page), `canvas` (a free canvas point), `page` (anchored to a page in viewport coords), or `region` (a rectangle in canvas space). One UI tool — `comment` — produces all of them; the gesture decides the anchor (click on an element → `element`; click off-page → `canvas`; drag a marquee → `region`). There is no `--kind` flag on `specular annotate`; pass `--page-id` to scope a comment to a page rather than the viewport.
| `specular snapshot -i` | Capture an accessibility snapshot with refs |
| `specular click @<ref>` | Click an element by ref |
| `specular fill @<ref> "<text>"` | Fill a form field |
| `specular screenshot -f <id>` | Screenshot a page |

## Entity types

| Kind | Created via | Description |
|---|---|---|
| page | `specular create page <url>` | Live web page rendered in a webview |
| text | `specular create note <text>` | Short text note (sticky-note style) |
| file | `specular upsert --json` | File entity — markdown (`.md`), wireframe (`.wireframe.json`), or HTML (`.html`) |

### Upsert tips

`upsert --json` is the fastest way to create many entities at once. It accepts
two shapes on stdin: a bare array of items (legacy), or `{layout, items}` with
a declarative `layout` directive that places everything for you.

**Use the `layout` directive for any composition of more than 2 items.** It
takes a `kind` (`row` / `column` / `grid`), a `gap` (token or pixel number),
and an anchor (`originX`/`originY`, `near: <id>`, or implicit). Same shape
creates new items *and* reorganizes existing ones — pass an `id` to re-lay-out
an entity that's already on the canvas.

```bash
# Create three pages in a row at breakpoints
cat << 'EOF' | specular upsert --json
{
  "layout": { "kind": "row", "gap": "m", "originX": 200, "originY": 200 },
  "items": [
    {"kind":"page","url":"https://example.com","presetIndex":0},
    {"kind":"page","url":"https://example.com","presetIndex":3},
    {"kind":"page","url":"https://example.com","presetIndex":6}
  ]
}
EOF
```

```bash
# Reorganize 6 existing pages into a 3x2 grid
cat << 'EOF' | specular upsert --json
{
  "layout": { "kind": "grid", "cols": 3, "gap": "m", "near": "frame_a" },
  "items": [
    {"id":"frame_a"}, {"id":"frame_b"}, {"id":"frame_c"},
    {"id":"frame_d"}, {"id":"frame_e"}, {"id":"frame_f"}
  ]
}
EOF
```

When a directive is present, per-item `canvasX`/`canvasY` are ignored. Without
`originX/Y` or `near`, the directive anchors at the bounding box of any
existing items in `items[]` (so re-layout doesn't teleport the cluster); with
no existing items, it falls back to `find-placement`.

**Spacing scale.** Tokens are aligned to the canvas grid (20px multiples).
Numbers work too as an escape hatch:

| Token | Pixels |
|---|---|
| `xs` | 20 |
| `s` | 40 |
| `m` | 60 |
| `l` | 100 |
| `xl` | 160 |

For ad-hoc single drops, the bare-array form still works and honors `canvasX`,
`canvasY`, `presetIndex`, and `orientation: "landscape" | "portrait"` per item.

For **files** (markdown, wireframe, image) without a directive, upsert ignores
`canvasX/canvasY` — the layout engine always places them. Images additionally
ignore `width`.

### Note colors

`color` on text notes (and group labels) expects a **JSON Canvas preset id
`"1"`–`"6"`** or a hex string. CSS color names like `"yellow"` or `"red"` are
NOT presets — they silently fall through and render as raw CSS (vivid
`#FFFF00`, `#FF0000`, etc.), which clashes hard with the canvas palette.

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

### Note sizes

Default to the built-in 200×200 sticky-note size — it's tuned to sit well
next to pages on the canvas. Only pass `width`/`height` when there's an
explicit reason (e.g. a long-form card that really needs more room). Custom
sizes tend to look off against the rest of the workspace.

### Wireframes

Files ending in `.wireframe.json` render as interactive wireframe editors on the
canvas. Use them to sketch UI layouts, explore design variants, and iterate
spatially alongside live pages. Write the JSON file to disk, then upsert it:

```bash
cat << 'EOF' | specular upsert --json
[{ "kind": "file", "file": "/tmp/my-layout.wireframe.json", "width": 300 }]
EOF
```

See [references/wireframes.md](references/wireframes.md) for the full node schema, layout patterns, and examples.

### HTML pages

Drop a `.html` file onto the canvas to render it inline (charts, mockups, generated visualizations). Write the file to disk, then upsert with `{ "kind": "file", "file": "/abs/path/viz.html" }`. Rendered display-only; edit the file to update.

## Passing URLs

Always pass full URLs (including scheme and host) to `specular create page`.
The canvas can contain pages from different origins, so bare paths like
`/garden` are ambiguous. Use `http://localhost:4321/garden`, not `/garden`.

## Chaining

Commands can be chained with `&&` for atomic sequences:

```
specular create page http://localhost:3000 && specular snapshot -i
```

## Switching the active page

Browse verbs (`snapshot`, `click`, `fill`, `scroll`, `screenshot`) need a target
page. There is no persistent "active page" binding — pass `-f <pageId>` on
every browse call:

```
specular snapshot -i -f <pageId>
specular click @e3 -f <pageId>
```

`specular focus <id>` only scrolls the canvas viewport — it does not set the
active page.

## Useful verbs

> **Assumed — verify when you first use each.** `specular back` has been
> confirmed once; `forward` and `reload` are inferred from the CLI help and
> have not been directly tested.

- `specular back` / `specular forward` / `specular reload` — browser history
  navigation inside the active page. Handy after `click` navigates you away
  and you want to return.

## Known CLI limitations

> **Treat this list as known assumptions, not ground truth.** Entries reflect
> behavior observed at the time they were added. Codepaths change, and some
> items here may already be fixed or may present differently in your session.
> If something behaves unexpectedly, re-test before trusting the list — and
> update it (remove the entry, or tighten its wording) when you confirm a
> change. A stale warning is worse than no warning.

When you encounter new gaps, append them to the tracking issue (see below).

- **`specular breakpoints <id>` creates sibling pages with malformed URLs** — the new pages get `https://<sourceFrameId>/` instead of the source page's real URL, so they load an invalid host instead of mirroring the page. Unusable as a multi-breakpoint primitive until fixed.
- **`specular update <id> --url` is a silent no-op** — the command returns `updated: [id]` but neither the webview nor the workspace URL field changes. Use `specular click` on a link, or delete + recreate the page, to navigate.
- **`specular update` silently ignores unsupported flags** — `--width`, `--label`, `--url` all return `updated: [id]` while applying nothing. Only `--preset / --at / --text / --color / --landscape / --portrait` actually take effect. Re-read workspace to verify.
- **`specular link` does not validate entity ids** — self-edges and edges to nonexistent ids are accepted and stored. Confirm both endpoints exist before calling `link`.
- **`specular delete <annotation_id>` silently lies** — the generic `delete` verb accepts annotation ids and returns `{"items":[{"kind":"file","id":"ann_...","deleted":true}]}` but does NOT call the annotation DELETE route. The annotation stays. Call `DELETE /annotations/:id` via raw HTTP, or wait for a dedicated verb.
- **`specular delete --json` requires `[{"id":"..."}]`, not `["..."]`** — the natural string-array shape crashes with `Cannot read properties of undefined (reading 'startsWith')`. Wrap each id in an object.
- **`upsert --json` ignores `x`/`y`/`width` for `file` entities** — text entities honor explicit coordinates; file entities (markdown, wireframe, image) are always placed by layout. Images additionally ignore `width` and land at 128×128.
- **Search box `fill` + `click` may not trigger navigation** — `fill` may not fire input events. If a click on Search fails, re-fill and retry, or click an autocomplete option ref instead.

## Tracking issue (localhost sessions)

When running against a local Specular app (`http://localhost:*`), you are working in a development context. If you hit a new CLI gap not listed above:

1. Add it to the list above in this skill file.
2. Append it as a comment to **[lklyne/specular#7](https://github.com/lklyne/specular/issues/7)** using:
   ```bash
   gh issue comment 7 --repo lklyne/specular --body "..."
   ```

Keep entries terse: one-line description, observed behavior, expected behavior. No need to comment for known issues already tracked.

When a limitation is fixed (confirmed by testing, not just by a closed issue), **remove it from the "Known CLI limitations" list above** and close or note it on the GitHub issue. A stale warning is worse than no warning.

## See also

- `agent-browser` skill — deeper browser-automation reference (invoked via
  `specular snapshot`, `specular click`, etc. under the hood).
