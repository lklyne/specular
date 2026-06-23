# Wireframes

Specular renders `.wireframe.json` files as interactive wireframe editors on the
canvas. Use them to sketch UI layouts, explore variants, and iterate spatially
alongside live frames.

## Creating wireframes

Wireframes are file entities. Create them with `specular upsert --json`:

```bash
cat << 'EOF' | specular upsert --json
[{
  "kind": "file",
  "file": "/tmp/my-layout.wireframe.json",
  "canvasX": 500,
  "canvasY": 200,
  "width": 300
}]
EOF
```

The file must exist on disk and end in `.wireframe.json`. Write the JSON first,
then upsert it as a file entity.

## Batch placement

Place multiple wireframes side-by-side in one call:

```bash
cat << 'EOF' | specular upsert --json
[
  { "kind": "file", "file": "/tmp/v1.wireframe.json", "canvasX": 100, "canvasY": 100, "width": 280 },
  { "kind": "file", "file": "/tmp/v2.wireframe.json", "canvasX": 430, "canvasY": 100, "width": 280 },
  { "kind": "file", "file": "/tmp/v3.wireframe.json", "canvasX": 760, "canvasY": 100, "width": 280 }
]
EOF
```

Use `specular find-placement` to find open canvas space before placing.

## File format

```json
{
  "version": "1.0",
  "theme": "light",
  "root": { "id": "root", "type": "frame", ... }
}
```

- **version** — always `"1.0"`
- **theme** — `"light"`, `"dark"`, or `"blueprint"`
- **root** — a single `frame` node containing the layout tree

## Node types

### frame (layout container)

The primary layout primitive. Arranges children in a row or column.

```json
{
  "id": "sidebar",
  "type": "frame",
  "direction": "vertical",
  "gap": 8,
  "padding": 16,
  "width": 200,
  "height": "fill",
  "children": [...]
}
```

- **direction** — `"horizontal"` or `"vertical"` (default vertical)
- **gap** — pixel spacing between children
- **padding** — single number, `[vertical, horizontal]`, or `[top, right, bottom, left]`
- **width/height** — pixel number, `"fill"` (stretch), or `"hug"` (shrink to content)

### text

```json
{ "id": "title", "type": "text", "text": "Welcome", "level": "h1" }
```

Levels: `"h1"`, `"h2"`, `"h3"`, `"body"`, `"caption"`

### button

```json
{ "id": "save", "type": "button", "text": "Save", "variant": "primary" }
```

Variants: `"primary"`, `"secondary"`, `"ghost"`

### input

```json
{ "id": "email", "type": "input", "label": "Email", "placeholder": "you@example.com" }
```

### dropdown

```json
{ "id": "lang", "type": "dropdown", "label": "Language", "placeholder": "Select...", "options": ["English", "Spanish"] }
```

### checkbox

```json
{ "id": "agree", "type": "checkbox", "label": "I agree", "checked": true }
```

### toggle

```json
{ "id": "notify", "type": "toggle", "label": "Notifications", "on": true }
```

### image (placeholder)

```json
{ "id": "hero", "type": "image", "width": 400, "height": 200, "alt": "Hero image" }
```

Renders as a gray placeholder rectangle with the alt text.

### divider

```json
{ "id": "sep", "type": "divider" }
```

Horizontal rule spanning the parent width.

### spacer

```json
{ "id": "gap", "type": "spacer" }
```

Flexible space — pushes siblings apart (like CSS `flex: 1`).

## Layout patterns

**Header with right-aligned actions:**

```json
{
  "id": "header", "type": "frame", "direction": "horizontal", "padding": [12, 16], "gap": 12, "width": "fill",
  "children": [
    { "id": "logo", "type": "text", "text": "App", "level": "h3" },
    { "id": "s", "type": "spacer" },
    { "id": "login", "type": "button", "text": "Log In", "variant": "primary" }
  ]
}
```

**Form section:**

```json
{
  "id": "form", "type": "frame", "direction": "vertical", "padding": 16, "gap": 12, "width": "fill",
  "children": [
    { "id": "name", "type": "input", "label": "Name", "placeholder": "Jane Doe" },
    { "id": "role", "type": "dropdown", "label": "Role", "options": ["Admin", "Editor", "Viewer"] },
    { "id": "submit", "type": "button", "text": "Save", "variant": "primary" }
  ]
}
```

**Two-column layout:**

```json
{
  "id": "body", "type": "frame", "direction": "horizontal", "gap": 0, "width": "fill", "height": "fill",
  "children": [
    { "id": "sidebar", "type": "frame", "direction": "vertical", "width": 200, "padding": 16, "gap": 8, "children": [...] },
    { "id": "content", "type": "frame", "direction": "vertical", "width": "fill", "padding": 24, "gap": 16, "children": [...] }
  ]
}
```

## Editing wireframes from the CLI

`specular wireframe` edits a wireframe's tree node-by-node, applying the same
structural ops the canvas uses. Each command is one undoable change that projects
back to the `.wireframe.json` file — so an agent edit and a canvas edit are
interchangeable, and undo on the canvas reverts either.

Target a wireframe by its file-entity id (from `specular workspace`) or by the
path you wrote to disk:

```bash
# Insert a default node of <nodeType> as a child of <parentId> at <index>
specular wireframe <fileId|path> insert <parentId> <index> <nodeType>
# …or insert a fully-specified node (must include a unique "id")
specular wireframe <fileId|path> insert <parentId> <index> --node '{"id":"cta","type":"button","text":"Buy","variant":"primary"}'

# Remove a subtree
specular wireframe <fileId|path> delete <nodeId>

# Clone a subtree (fresh ids) right after the original
specular wireframe <fileId|path> duplicate <nodeId>

# Move a node under a target frame at an index
specular wireframe <fileId|path> reorder <nodeId> <targetParentId> <targetIndex>

# Set properties on a node (key/value pairs, or --props for arrays/objects)
specular wireframe <fileId|path> set <nodeId> text "Sign up" variant primary
specular wireframe <fileId|path> set <nodeId> --props '{"options":["A","B"],"label":"Plan"}'
```

`set` values are coerced: `true`/`false` → boolean, numeric strings → number,
everything else → string (so `set frame gap 8` sets `8`, `set frame width fill`
sets `"fill"`). Use `--props` when a value must stay a string despite looking
numeric, or for arrays/objects like `options`/`padding`.

Only the props legal for a node type may be set (see each node type above). The
command fails with a legible error on an unknown node id, an illegal prop, or a
non-frame insert/reorder target — nothing is written when it fails.

## Interactive features

Wireframes on the canvas are also interactive:

- **Text** — click to edit inline
- **Checkboxes/Toggles** — click to toggle state
- **Nodes** — drag to reorder within their parent frame

Changes persist back to the `.wireframe.json` file on disk.

## Tips

- Every node needs a unique `id` within the file.
- Use `"width": "fill"` on the root frame and section frames so they stretch to the entity width on the canvas.
- Write wireframe files to `/tmp/` for throwaway explorations; write to the project directory if they should persist.
- Create a title label as a text note alongside wireframes for context:
  `{ "kind": "text", "text": "V1: Stacked list", "canvasX": 100, "canvasY": 80 }`
