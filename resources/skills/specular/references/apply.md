# Apply — worked examples

`specular apply` reads one JSON patch from stdin and applies it in a single
transaction. The patch shape, field rules, and spacing tokens live in
SKILL.md ("Fallback — the JSON door"); these are worked examples of the batch
cases where one patch beats a string of verbs.

## Create three pages in a row at breakpoints

```bash
cat << 'EOF' | specular apply
{
  "layout": { "kind": "row", "gap": "m", "originX": 200, "originY": 200 },
  "entities": [
    {"kind":"page","url":"https://example.com","presetIndex":0},
    {"kind":"page","url":"https://example.com","presetIndex":3},
    {"kind":"page","url":"https://example.com","presetIndex":6}
  ]
}
EOF
```

## Reorganize six existing pages into a 3×2 grid

Items with only an `id` are position-only updates — the layout directive
moves them without touching their content:

```bash
cat << 'EOF' | specular apply
{
  "layout": { "kind": "grid", "cols": 3, "gap": "m", "near": "frame_a" },
  "entities": [
    {"id":"frame_a"}, {"id":"frame_b"}, {"id":"frame_c"},
    {"id":"frame_d"}, {"id":"frame_e"}, {"id":"frame_f"}
  ]
}
EOF
```
