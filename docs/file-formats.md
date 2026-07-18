# File Formats

Specular uses open, human-readable, local-first file formats. All data lives
on the user's machine as plain files. No proprietary formats, no server
dependency.

## Spaces (vault model)

A **space** is a folder on disk containing .canvas files and assets. Similar
to an Obsidian vault. The file system structure is the organizational model:

```
my-project/                      # a space
  research.canvas                # a canvas
  homepage-redesign.canvas
  breakpoint-review.canvas
  assets/                        # referenced files
    screenshot.png
```

Canvases are auto-saved. The location is currently:
`~/.config/Specular/workspaces/default/`

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
- `syncId` — sync-set identifier (string | null); pages with the same non-null `syncId` navigate in lockstep (replaces the old boolean `linked` field, which was removed by ADR 0027)
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
  "label": "navigates to"
}
```

Specular extensions: `edgeKind`, `edgeMetadata`.

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
