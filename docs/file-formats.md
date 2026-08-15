# File Formats

Specular uses open, human-readable, local-first file formats. All data lives
on the user's machine as plain files. No proprietary formats, no server
dependency.

## Spaces (vault model)

A **space** is a folder on disk containing .canvas files and assets. Similar
to an Obsidian vault. The file system structure is the organizational model:

```
my-project/                      # a space
  research-8f21.canvas           # a canvas
  homepage-redesign-b04c.canvas
  breakpoint-review-1d7e.canvas
  assets/                        # referenced files
    screenshot.png
```

A canvas file is named for its tab, suffixed with the first four characters of
the tab's id. The suffix is what makes the path unique: two tabs sharing a name
would otherwise share one file, and saving, renaming, or deleting either would
clobber the other. Renaming a tab renames its file; the suffix stays.

Canvases are auto-saved to whatever `spaceDir()` resolves to
([ADR 0033](adr/0033-user-chosen-space-folder.md)): the user-chosen
`spacePath` preference, with `.canvas` files and `assets/` sitting directly
inside it — no `workspaces/<id>/` nesting. A new install picks its space
during onboarding, defaulting to `~/Specular` (one click to accept); an
existing space can be changed later from Settings → General, which prompts
to move canvases into the new folder, start fresh there, or cancel — it
never guesses. Installs that predate the ADR keep an unset `spacePath`,
which resolves to the legacy location — on macOS,
`~/Library/Application Support/Specular/workspaces/default/` — until the
user changes it. App metadata (`workspace-meta.json`) lives in a
`.specular/` subdirectory of the space, with a read-side fallback to a meta
file at the space's root for spaces written before that convention existed.

## .canvas (JSON Canvas v1.0)

The primary data format. Follows the JSON Canvas specification:
https://jsoncanvas.org/
https://github.com/obsidianmd/jsoncanvas/blob/main/spec/1.0.md

A .canvas file is JSON with two required arrays:

```json
{
  "nodes": [...],
  "edges": [...]
}
```

### Nodes

Every item on the canvas is a node with a position and size:

```json
{
  "id": "abc123",
  "type": "link",
  "x": 100,
  "y": 200,
  "width": 1280,
  "height": 800,
  "url": "https://example.com"
}
```

Node types (per JSON Canvas spec):

| type | fields | description |
|------|--------|-------------|
| `text` | `text` | Markdown/plain text note |
| `link` | `url` | Web page (rendered as live webview) |
| `file` | `file`, `subpath?` | Reference to a local file |
| `group` | `label?`, `background?` | Visual container for other nodes |

### Specular extensions

The JSON Canvas spec is designed to be extensible — unknown fields are ignored
by other tools. Specular adds:

**On link nodes:**
- `presetIndex` — viewport preset (device catalog index)
- `linked` — whether this page is linked to others for sync
- `label` — display name
- `parentGroupId` — group membership
- `metadata` — open-ended key-value store

**On file nodes:**
- `objectFit` — how the file content fits its bounds (`contain` / `cover` / `fill`)
- `presetIndex` — viewport preset (device catalog index), used by component renderers
- `metadata` — open-ended, namespaced by plugin id. Note that `.tsx` / `.jsx`
  file entities map to a connected Vite repo at render time by looking up
  the longest connected-repo prefix of the absolute file path — no
  metadata is required, and the entity heals automatically if a more
  specific repo is connected later.

**On group nodes:**
- `groupKind` — type of group (e.g., breakpoint set)
- `layoutMode` — auto-layout algorithm
- `entityIds` / `pageIds` — member references
- `managedLayout` — whether the group controls child positions

**On all nodes:**
- `color` — preset color "1"-"6" or hex "#RRGGBB"

**Shape nodes (`type: "shape"`):**

Shapes are a Specular node-type extension. Their geometry and border remain
plain, readable top-level fields (`shapeKind`, `text`, `strokeWidth`,
`borderStyle`, and `borderColor`). New Specular-only presentation fields live
in the namespaced `specular` object:

```json
{
  "id": "shape1",
  "type": "shape",
  "x": 100,
  "y": 100,
  "width": 240,
  "height": 120,
  "shapeKind": "rectangle",
  "text": "Review",
  "color": "4",
  "strokeWidth": 2,
  "borderStyle": "solid",
  "specular": {
    "fillStyle": "none",
    "textAlign": "left",
    "textVerticalAlign": "top"
  }
}
```

Missing `fillStyle` means `solid`; missing text alignment means
`center`/`middle`. This preserves existing files without migration. JSON Canvas
readers that do not support shapes or these optional fields can ignore them,
while the file stays valid, transparent JSON.

### Edges

Connections between two nodes:

```json
{
  "id": "edge1",
  "fromNode": "abc123",
  "toNode": "def456",
  "fromSide": "right",
  "toSide": "left",
  "color": "3",
  "strokeWidth": 3,
  "lineStyle": "dashed",
  "label": "navigates to"
}
```

Specular extensions: `strokeWidth`, `lineStyle` (`solid` or `dashed`),
`edgeKind`, and `edgeMetadata`. Missing stroke fields render as a 1.5px solid
connection for compatibility with existing canvases.

Routing extensions, written by the connect tool (`X`) and the anchor-drag
gesture: `routing` (`bezier` | `elbow` | `straight`; absent renders as
`bezier`, so no existing canvas changes appearance) and, only when
`routing === 'elbow'`, `elbowSplit` (a normalized 0–1 position for a dragged
crossbar) paired with `elbowSplitAxis` (`x` or `y`, the axis the crossbar was
dragged on — the split is meaningless without it, since the same number means
a different placement on each axis).

#### Free-ended edges (`specular.freeEdges`)

An edge endpoint can be a bare canvas-space point instead of a node: dragging
the connect tool from empty space starts an edge with nothing bound yet, and
deleting the entity at one end of an existing edge detaches that end to a
point rather than deleting the edge. JSON Canvas requires `fromNode`/`toNode`
on every entry in `edges[]`, so there is no spec-legal way to write a
dangling edge there.

Any edge with a free end is written instead into a top-level
`specular.freeEdges` array, with `fromNode`/`toNode` omitted for the free side
and a matching `fromPoint`/`toPoint` (`{ x, y }` in canvas coordinates)
carrying its position:

```json
{
  "nodes": [...],
  "edges": [...],
  "specular": {
    "freeEdges": [
      {
        "id": "edge2",
        "toNode": "def456",
        "toSide": "left",
        "fromPoint": { "x": 120, "y": 340 }
      }
    ]
  }
}
```

A strict JSON Canvas reader sees a fully valid `edges[]` and simply doesn't
see free-ended edges. Specular merges `specular.freeEdges` back into the same
runtime edge collection on load — there is exactly one edge collection at
runtime; the `edges[]` / `specular.freeEdges` split exists only in the
serialized form. Re-binding a free end to an entity moves the edge back into
`edges[]` on the next save. See [ADR
0034](./adr/0034-free-edges-outside-json-canvas-edges-array.md).

### App state (extension)

Specular stores viewport and UI state in an `appState` field:

```json
{
  "nodes": [...],
  "edges": [...],
  "appState": {
    "zoom": 0.5,
    "pan": { "x": -200, "y": -100 },
    "selectedEntityIds": ["abc123"],
    "leftSidebarOpen": true
  }
}
```

Other tools ignore this field per the spec's extensibility model. Older files
may contain `browserTabMode`; Specular reads it only as a legacy restore hint
and no longer writes it.

### Annotations (extension)

Freehand drawings/annotations stored in an `annotations` array:

```json
{
  "nodes": [...],
  "edges": [...],
  "annotations": [
    {
      "id": "ann1",
      "canvasX": 100,
      "canvasY": 200,
      "width": 300,
      "height": 150,
      "strokes": [...],
      "color": "#ff0000"
    }
  ]
}
```

#### Comment anchors

Comment annotations carry an `anchor` (discriminated by `anchor.type`) and,
when page-bound, a `pageAnchor { pageId, pageUrl? }` (see ADR 0031). The
anchor variants:

```json
{ "type": "canvas", "canvasX": 100, "canvasY": 200 }
{ "type": "page", "pageId": "p1", "offsetX": 0.5, "offsetY": 0.25 }
{ "type": "element", "pageId": "p1", "selector": "…", "boundingBox": { … } }
{ "type": "region", "canvasRect": { "x": 0, "y": 0, "width": 80, "height": 60 } }
{ "type": "region", "docRect":   { "x": 20, "y": 20, "width": 80, "height": 60 } }
```

**Region anchors have two arms**, distinguished by which rect field is present:

- `canvasRect` — a **canvas-anchored** region (its marquee grabbed no page
  content). The rect is in canvas coordinates; it marks canvas space and never
  moves with a page.
- `docRect` — a **page-anchored** region (its marquee grabbed page content).
  The rect is in the *document* CSS pixels of the page named by `pageAnchor`,
  so the region scroll-follows and travels with the page (rendered through the
  scroll-aware transform). Read `docRect` iff the field is present; a region
  with only `canvasRect` — including every file written before scroll
  tracking — is canvas-anchored, with no migration.

## workspace-meta.json

Metadata about the canvas tabs within a space:

```json
{
  "activeTabId": "tab_1",
  "tabs": [
    {
      "id": "tab_1",
      "name": "Research",
      "updatedAt": "2025-01-15T10:30:00Z",
      "expanded": true
    }
  ]
}
```

Older metadata may contain `viewMode`; Specular reads it only as a legacy
restore hint and no longer writes it.

## Type definitions

See `src/shared/json-canvas-types.ts` for the full TypeScript types.
See `src/shared/types.ts` for internal entity types.

## Compatibility

.canvas files created by Specular should open in Obsidian and other tools
that support JSON Canvas v1.0. Specular-specific extensions are ignored by
those tools. Conversely, .canvas files from other tools should open in
Specular (link nodes render as live webviews, text nodes as notes, etc.).
