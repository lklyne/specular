#!/usr/bin/env python3
"""Checks the .canvas files the scenarios saved. Run by run.sh after the
scripts; exits non-zero and says what differs when a check fails.

Every check compares JSON values, so key order and whitespace do not count.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = Path(sys.argv[1])
RAN = set(sys.argv[2:])


def load(path):
    return json.loads(Path(path).read_text())


def differences(a, b, path=""):
    """Paths at which `a` and `b` differ, with both values."""
    if type(a) is not type(b):
        return [(path, a, b)]
    if isinstance(a, dict):
        out = []
        for key in sorted(set(a) | set(b)):
            if key not in a or key not in b:
                out.append((f"{path}/{key}", a.get(key, "<absent>"), b.get(key, "<absent>")))
            else:
                out += differences(a[key], b[key], f"{path}/{key}")
        return out
    if isinstance(a, list):
        keyed = all(isinstance(item, dict) and "id" in item for item in a + b)
        if keyed and [item["id"] for item in a] == [item["id"] for item in b]:
            return [d for x, y in zip(a, b) for d in differences(x, y, f"{path}/{x['id']}")]
        if len(a) != len(b) or keyed:
            return [(path, f"{len(a)} items", f"{len(b)} items, or another order")]
        return [d for i, (x, y) in enumerate(zip(a, b)) for d in differences(x, y, f"{path}/{i}")]
    return [] if a == b else [(path, a, b)]


failures = []


def expect(name, before, after, allowed=()):
    """`after` differs from `before` only at the `allowed` paths, and at
    every one of them. The camera is not part of the comparison."""
    if not RAN.issuperset(name.split("+")):
        return
    found = [d for d in differences(before, after) if not d[0].startswith("/appState")]
    paths = sorted(path for path, _, _ in found)
    if paths == sorted(allowed):
        print(f"ok   {name}")
        return
    failures.append(name)
    print(f"FAIL {name}: expected changes at {sorted(allowed)}")
    for path, x, y in found:
        print(f"       {path}: {json.dumps(x)[:70]} -> {json.dumps(y)[:70]}")


def saved(scenario, file):
    path = OUT / scenario / file
    return load(path) if path.exists() else None


EMPTY = load(HERE / "empty.canvas")
SINK = load(HERE / "../kitchen-sink.canvas")
WELCOME = load(HERE / "../../../resources/starter-space/Welcome.canvas")
RICH = load(HERE / "../../../tests/integration/__snapshots__/rich-workspace.canvas")
NUDGED = "shape_10ac9206"

a = "a-first-session"
expect(a, EMPTY, saved(a, "10-undone.canvas"))
expect(a, saved(a, "08-arranged.canvas"), saved(a, "12-redone.canvas"))
b = "b-kitchen-sink-selection"
expect(b, SINK, saved(b, "02-start.canvas"))
for undone in ["08-resize", "10-duplicate", "13-option-drag", "15-nudges", "17-delete"]:
    expect(b, SINK, saved(b, f"{undone}-undone.canvas"))
d = "d-text-edge-cases"
expect(d, EMPTY, saved(d, "15-undone.canvas"))
e = "e-clipboard"
expect(e, EMPTY, saved(e, "07-undone.canvas"))
g = "g-zoom-and-pan"
expect(g, SINK, saved(g, "13-end.canvas"))
h = "h-tools-and-escape"
if h in RAN:
    start, end = saved(h, "00-start.canvas"), saved(h, "14-end.canvas")
    # Every gesture up to here was escaped.
    expect(h, start, saved(h, "09-gestures-escaped.canvas"))
    sticky = start["nodes"][0]["id"]
    made = [node["id"] for node in end["nodes"] if node["id"] != sticky]
    # The sticky was typed into and one shape was drawn; every escaped
    # gesture left nothing.
    ok = len(made) == 1 and end["nodes"][0]["text"].startswith("keep me")
    print(f"{'ok  ' if ok else 'FAIL'} {h}: one sticky and one shape remain")
    if not ok:
        failures.append(h)
i = "i-comments"
if i in RAN:
    expect(i, SINK, saved(i, "00-start.canvas"))
    expect(i, SINK, saved(i, "27-undone.canvas"))
    made = saved(i, "18-commented.canvas")["annotations"][len(SINK["annotations"]):]
    fields = {"id", "anchor", "author", "text", "status", "replies", "createdAt", "pageAnchor", "metadata"}
    kinds = [a["anchor"]["type"] for a in made]
    elements = [a for a in made if a["anchor"]["type"] == "element"]
    ok = (
        kinds == ["canvas", "canvas", "element", "region", "region", "canvas"]
        and all(set(a) <= fields for a in made)
        and all(a["author"] == "user" and a["status"] == "pending" and a["replies"] == [] for a in made)
        # Each comment is queued into the canvas's agent thread, whose id it carries.
        and all(isinstance(a.get("metadata", {}).get("threadId"), str) for a in made)
        # A region is the page's when it grabbed something there, and then
        # it is kept in the page's document space; else it is on the canvas.
        and all(
            set(a["anchor"]) == ({"type", "docRect"} if "pageAnchor" in a else {"type", "canvasRect"})
            for a in made
            if a["anchor"]["type"] == "region"
        )
        and [("pageAnchor" in a) for a in made if a["anchor"]["type"] == "region"] == [False, True]
        and len(elements) == 1
        and set(elements[0]["anchor"]) >= {"type", "pageId", "selector", "boundingBox"}
        and elements[0]["pageAnchor"]["pageId"] == elements[0]["anchor"]["pageId"]
        and made[-1]["text"] == "first line\nsecond line"
    )
    print(f"{'ok  ' if ok else 'FAIL'} {i}: six comments in Electron's shape")
    if not ok:
        failures.append(i)
j = "j-toolbar-and-popups"
if j in RAN:
    nodes = lambda canvas: {n["id"]: n for n in canvas["nodes"]}
    edges = lambda canvas: {e["id"]: e for e in canvas.get("edges", [])}
    note_a, note_b = saved(j, "03-sticky-placed.canvas"), saved(j, "06-sticky-restyled.canvas")
    note = next(iter(nodes(note_a)))
    grid, border = saved(j, "08-shape-placed.canvas"), saved(j, "10-shape-restyled.canvas")
    joined, styled = saved(j, "11-edge-made.canvas"), saved(j, "14-edge-restyled.canvas")
    undone, redone, end = (saved(j, f) for f in ["15-edge-start-undone.canvas", "16-edge-start-redone.canvas", "23-end.canvas"])
    shape = next(id for id, n in nodes(border).items() if n["type"] == "shape")
    edge = next(iter(edges(styled)))
    sticky = lambda c: {k: nodes(c)[note].get(k) for k in ("color", "text")} | {"size": nodes(c)[note]["specular"]["textSize"]}
    first = lambda c: {k: nodes(c)[shape].get(k) for k in ("shapeKind", "color", "strokeWidth", "borderStyle", "borderColor")}
    look = lambda c: {k: edges(c)[edge].get(k) for k in ("color", "strokeWidth", "lineStyle", "fromEnd", "toEnd")}
    checks = {
        # The sticky tool's blue default, then green and a larger size from the popup.
        "sticky takes the tool's color": sticky(note_a) == {"color": "7", "text": "first note", "size": 14},
        "sticky recolored and resized": sticky(note_b) == {"color": "4", "text": "first note", "size": 33},
        # The shape tool's diamond and yellow, then a hexagon with a dashed red border.
        "shape takes the tool's kind and color": first(grid) == {"shapeKind": "diamond", "color": "3", "strokeWidth": 2, "borderStyle": None, "borderColor": None},
        "shape kind and border changed": first(border) == {"shapeKind": "hexagon", "color": "3", "strokeWidth": 3, "borderStyle": "dashed", "borderColor": "1"},
        "edge dragged between the shapes is plain": look(joined) == {"color": None, "strokeWidth": None, "lineStyle": None, "fromEnd": None, "toEnd": "arrow"},
        "edge restyled": look(styled) == {"color": "7", "strokeWidth": 3, "lineStyle": "dashed", "fromEnd": "arrow", "toEnd": "arrow"},
        "undo takes back the arrowhead only": look(undone) == look(styled) | {"fromEnd": None},
        "redo puts it back": look(redone) == look(styled),
        # Picking a zoom level and closing a list change no document.
        "zoom leaves the document alone": (nodes(end), edges(end)) == (nodes(styled), edges(styled)),
    }
    for name, held in checks.items():
        print(f"{'ok  ' if held else 'FAIL'} {j}: {name}")
        if not held:
            failures.append(f"{j}: {name}")
k = "k-page-chrome"
if k in RAN:
    page = lambda c: next(n for n in c["nodes"] if n["type"] == "link")
    size = lambda c: (page(c)["width"], page(c)["height"])
    placed, custom, undone = (saved(k, f) for f in ["02-page-placed.canvas", "07-custom-size.canvas", "08-size-undone.canvas"])
    checks = {
        # The Desktop preset the page tool was set to, not the first one.
        "page placed at the tool's preset": size(placed) == (1440, 900) and page(placed).get("presetIndex") == 7,
        "typed width and height make a custom size": size(custom) == (900, 600) and page(custom)["metadata"]["pageSizeMode"] == "custom",
        "undo takes the two sizes back": size(undone) == (1440, 900),
    }
    for name, held in checks.items():
        print(f"{'ok  ' if held else 'FAIL'} {k}: {name}")
        if not held:
            failures.append(f"{k}: {name}")
l = "l-sidebar"
expect(l, SINK, saved(l, "00-start.canvas"))
# Canvases added, renamed and deleted, folds, rows that select and reveal,
# and a zoom to fit are all views: the kitchen sink is as it was.
expect(l, SINK, saved(l, "99-end.canvas"))
m = "m-context-menu-and-arrange"
if m in RAN:
    nodes = lambda c: {n["id"]: n for n in c["nodes"]}
    start = saved(m, "00-start.canvas")
    duplicated, row, column = (saved(m, f) for f in ["02-duplicated.canvas", "08-arranged-row.canvas", "10-arranged-column.canvas"])
    bold = nodes(saved(m, "16-bold.canvas"))["sticky-list"]["text"]
    at = lambda c, id: (nodes(c)[id]["x"], nodes(c)[id]["y"])
    checks = {
        # The sticky under the pointer was selected and copied by the menu.
        "the menu's duplicate adds one sticky": len(duplicated["nodes"]) == len(start["nodes"]) + 1,
        # Evened gaps across the row's footprint, tops lined up.
        "a row evens the gaps": [at(row, i) for i in ("shape-rectangle", "shape-ellipse", "shape-pill")] == [(0, 1300), (600, 1300), (1200, 1300)],
        "a column lines the left edges up": [at(column, i) for i in ("shape-rectangle", "shape-rounded", "shape-ellipse")] == [(0, 1300), (0, 1520), (0, 1740)],
        "a press on the bold button wraps the selected text": bold.startswith("**") and bold.endswith("**"),
    }
    for name, held in checks.items():
        print(f"{'ok  ' if held else 'FAIL'} {m}: {name}")
        if not held:
            failures.append(f"{m}: {name}")
    for undone in ["03-duplicate-undone", "09-row-undone", "11-column-undone", "17-bold-undone"]:
        expect(m, SINK, saved(m, f"{undone}.canvas"))
n = "n-chrome-session"
if n in RAN:
    start, duplicated, fresh, back = (saved(n, f) for f in ["00-start.canvas", "05-duplicated.canvas", "06-new-canvas.canvas", "07-first-canvas.canvas"])
    pages = [node for node in duplicated["nodes"] if node.get("url") == "https://example.org/docs"]
    checks = {
        # The address typed in the popup reached the page.
        "the address entered in the popup is the page's": len(pages) == 1,
        # The menu's duplicate copied one sticky and nothing else.
        "the menu's duplicate adds one sticky": len(duplicated["nodes"]) == len(start["nodes"]) + 1,
        # The sticky went to the canvas that was added, which has nothing else.
        "the new canvas holds only its own sticky": len(fresh["nodes"]) == 1,
        # Switching back shows the first canvas as it was left.
        "the first canvas is as it was left": back == duplicated,
    }
    for name, held in checks.items():
        print(f"{'ok  ' if held else 'FAIL'} {n}: {name}")
        if not held:
            failures.append(f"{n}: {name}")
f1 = "f1-reload-own-save"
expect(f"{a}+{f1}", saved(a, "08-arranged.canvas"), saved(f1, "01-reloaded.canvas"))
f2 = "f2-electron-file-one-change"
if f2 in RAN:
    node = next(n["id"] for n in WELCOME["nodes"] if n["id"].startswith(NUDGED))
    expect(f2, WELCOME, saved(f2, "01-untouched.canvas"))
    expect(f2, WELCOME, saved(f2, "02-one-change.canvas"), [f"/nodes/{node}/x"])
    expect(f2, WELCOME, saved(f2, "03-undone.canvas"))
f3 = "f3-fixture-one-change"
expect(f3, RICH, saved(f3, "01-one-change.canvas"), ["/nodes/generated-id-2/y"])

sys.exit(1 if failures else 0)
