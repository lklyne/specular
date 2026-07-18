# Driving Google Sheets

Google Sheets renders its grid as a single `<canvas>` element — there are no
DOM cells, so `specular snapshot -i` can never give you per-cell refs. Select
cells by address through the Name box, not by ref.

Everything below was observed against Google Sheets at time of writing. Treat
it as a starting point, not a guarantee, for other canvas-rendered grids
(Excel Online, Airtable, and similar) — and re-test before trusting it;
these apps change under you.

## The write path that works

One cell per pass:

1. `click` the **Name box**, type the cell address (e.g. `B2`), press Enter — this selects the cell.
2. `click` the **formula bar** — it becomes a combobox in edit mode.
3. `type` the value.
4. `press Enter` to commit.

There is no shortcut past step 2: no focusable input exists on the active
cell until edit mode, so `keyboard type`, `keyboard inserttext`, and `press`
write nothing to a merely-selected cell.

## Focus traps

- **After committing, focus stays in the formula-bar combobox.** `click` the
  Name box between cells; skip it and the next navigation silently no-ops.
- **Native autocomplete can hijack the Enter-commit** — pre-clear the column
  before typing into it.
- **A hidden screen-reader textbox can silently swallow input** and still
  report "✓ Done" — treat that as a false success.

## Verify with screenshots

Most silent failures in these apps also report "✓ Done" — success is only
observable via a screenshot round-trip. After each write (or small batch),
`specular screenshot -f <pageId>` and confirm the cell actually renders the
value before moving on.
