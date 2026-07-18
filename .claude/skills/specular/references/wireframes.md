# Wireframes

Specular renders `.wireframe.json` files as interactive wireframe editors on the
canvas. Use them to sketch UI layouts, explore variants, and iterate spatially
alongside live frames.

## Creating wireframes

Wireframes are file entities. Write the JSON to disk first, then add it:

```bash
specular add file /tmp/my-layout.wireframe.json --at 500,200
```

The file must exist on disk and end in `.wireframe.json`. New file entities
default to 200×200; wireframes usually want more room. Resize after creation
with `specular update <id> --size 300,400`, or set `width` up front by
creating through `apply` (below).

## Batch placement

Place multiple wireframes side-by-side in one `apply` patch. The `layout`
directive handles the spacing; without an anchor (`originX`/`originY` or
`near`) it finds open canvas space automatically:

```bash
cat << 'EOF' | specular apply
{
  "layout": { "kind": "row", "gap": "m" },
  "entities": [
    { "kind": "file", "file": "/tmp/v1.wireframe.json", "width": 280 },
    { "kind": "file", "file": "/tmp/v2.wireframe.json", "width": 280 },
    { "kind": "file", "file": "/tmp/v3.wireframe.json", "width": 280 }
  ]
}
EOF
```

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

## Interactive features

Wireframes on the canvas are interactive:

- **Text** — click to edit inline
- **Checkboxes/Toggles** — click to toggle state
- **Nodes** — drag to reorder within their parent frame

Changes persist back to the `.wireframe.json` file on disk.

## Tips

- Every node needs a unique `id` within the file.
- Use `"width": "fill"` on the root frame and section frames so they stretch to the entity width on the canvas.
- Write wireframe files to `/tmp/` for throwaway explorations; write to the project directory if they should persist.
- Create a title label as a text note alongside wireframes for context:
  `specular add note "V1: Stacked list" --at 100,80`
