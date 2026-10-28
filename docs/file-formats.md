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

Canvases are auto-saved to the space folder
([ADR 0033](adr/0033-user-chosen-space-folder.md)): the user-chosen
`spacePath` preference, with `.canvas` files and `assets/` sitting directly
inside it — no `workspaces/<id>/` nesting. A new install picks its space on
the first-run view (create a new folder or open an existing one); Settings →
General opens another folder later and leaves the current one alone. An
install whose `spacePath` was never set opens the legacy location — on
macOS, `~/Library/Application Support/Specular/workspaces/default/`. App
metadata (`workspace-meta.json`) lives in a
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
- An HTML file (`.html` / `.htm`) is a live page,
  loaded from its file's `file://` URL and reloaded when it or a
  stylesheet, script or JSON file beside it changes. It stays a `file`
  node: `presetIndex` and `metadata` hold its viewport preset and device
  frame as a link's do, and the frame grows outward from `x`/`y`, which
  place the page itself (a link's `x`/`y` place its frame's corner).
- `specular.syncId` / `specular.colorScheme` — an HTML file's sync set and
  color-scheme override, as `syncId` / `colorScheme` on a link. A sync set
  holds links or HTML files, never both.
- A comment or item anchored to an HTML file in the space folder records
  `pageUrl` as the file's path relative to that folder, never an absolute
  `file://` URL, so the canvas moves with its folder.

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
`edgeKind`, `edgeMetadata` and `route`. Missing stroke fields render as a 1.5px
solid connection for compatibility with existing canvases.

`route` is the path a flow layout gave the edge: its corners in canvas
units, `[[x, y], ...]`, from where it leaves `fromNode` to where it reaches
`toNode`, every run level or upright. It is drawn only while its first point
sits on `fromSide` of its first node and its last on `toSide` of the other;
once either node moves, the edge is drawn as a curve again and the route is
kept until the next layout replaces it. Other tools can ignore it.

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
    "leftSidebarOpen": true,
    "itemTabs": [
      { "id": "abc123", "lens": "device" },
      { "id": "def456", "lens": "fill" }
    ]
  }
}
```

Other tools ignore this field per the spec's extensibility model. Older files
may contain `browserTabMode`; Specular reads it only as a legacy restore hint
and no longer writes it.

`itemTabs` lists the pages and
Documents of this canvas that are open in a tab of their own, in tab order,
each with its lens (`fill` or `device`, see ADR 0045). A canvas with no tab
open has no `itemTabs`. On load an entry with no `id` is skipped, an entry
whose node is gone or is not a page or a Document is dropped, and a missing or
unknown `lens` is `fill`. Which tab was showing is not stored, so a space opens
on its Canvas tab.

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

The reader and writer are `native/crates/specular-doc/src/canvas/`
(`read.rs`, `write.rs`, `fields.rs`). The typed entities they produce are in
`native/crates/specular-doc/src/`. A save writes keys in the order the
Electron app did, so a file it wrote loads and saves to the same bytes
(`specular-doc/tests/canvas_repo.rs`).

## Compatibility

.canvas files created by Specular should open in Obsidian and other tools
that support JSON Canvas v1.0. Specular-specific extensions are ignored by
those tools. Conversely, .canvas files from other tools should open in
Specular (link nodes render as live pages, text nodes as notes, etc.).
