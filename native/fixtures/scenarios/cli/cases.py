"""The `specular` CLI end to end against a running app on a scratch space.

run.sh starts the app and calls this with the CLI, the space folder and the
discovery file. Each case runs verbs as an agent would, on a canvas of its
own (made with `tab new` and shown with `tab switch`, so its writes are on
the undo stack), and asserts on what the verb printed, its exit code, the
canvas the app reads back and the .canvas file it saved.

Usage: cases.py CLI SPACE [case ...]
"""

import glob
import json
import os
import subprocess
import sys
import time
import traceback
import urllib.request

CLI, SPACE = sys.argv[1], sys.argv[2]
ONLY = set(sys.argv[3:])
DISCOVERY = os.environ["SPECULAR_DISCOVERY_FILE"]


class Failed(Exception):
    pass


def check(ok, message):
    if not ok:
        raise Failed(message)


def equal(got, want, what=""):
    check(got == want, f"{what}: got {got!r}, want {want!r}")


class Ran:
    def __init__(self, args, proc):
        self.args, self.code = args, proc.returncode
        self.out, self.err = proc.stdout, proc.stderr

    def json(self):
        try:
            return json.loads(self.out)
        except ValueError:
            raise Failed(f"specular {' '.join(self.args)}: not JSON:\n{self.out}{self.err}")


def cli(*args, stdin=None, code=0, env=None):
    """Runs the CLI and checks its exit code."""
    full_env = dict(os.environ, **(env or {}))
    proc = subprocess.run(
        [CLI, *args], input=stdin, capture_output=True, text=True, timeout=60, env=full_env
    )
    ran = Ran(args, proc)
    if code is not None and ran.code != code:
        raise Failed(
            f"specular {' '.join(args)} exited {ran.code}, want {code}\n"
            f"stdout: {ran.out}\nstderr: {ran.err}"
        )
    return ran


def ok(*args, stdin=None):
    return cli(*args, stdin=stdin).json()


def refused(want, *args, stdin=None, env=None):
    """The verb fails with exit 1 and `want` in its error."""
    ran = cli(*args, stdin=stdin, code=1, env=env)
    check(want in ran.err, f"specular {' '.join(args)}: wanted {want!r} in:\n{ran.err}{ran.out}")
    return ran


def http(path, body=None):
    with open(DISCOVERY) as f:
        found = json.load(f)
    request = urllib.request.Request(
        f"http://127.0.0.1:{found['port']}{path}",
        data=json.dumps(body or {}).encode(),
        headers={"x-specular-secret": found["secret"], "content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=10) as answer:
        return json.load(answer)


def undo():
    check(http("/history/undo")["ok"], "nothing to undo")


def nodes():
    return {node["id"]: node for node in ok("canvas")["nodes"]}


def node(id):
    return nodes().get(id)


def saved(name, test, what):
    """Waits for the autosave of canvas `name` to pass `test`. A canvas
    made by `tab new` is saved as `<name>-<id>.canvas`."""
    path = os.path.join(SPACE, f"{name}.canvas")
    last = None
    for _ in range(50):
        found = glob.glob(os.path.join(SPACE, glob.escape(name) + "*.canvas"))
        path = found[0] if found else path
        try:
            with open(path) as f:
                last = json.load(f)
            if test(last):
                return last
        except (OSError, ValueError):
            pass
        time.sleep(0.1)
    raise Failed(f"{path} never showed {what}; last read: {json.dumps(last)[:600]}")


def saved_ids(document):
    return {n["id"] for n in document.get("nodes", [])}


def fresh(name, notes=False):
    """A new canvas, shown, empty or with the three notes a, b, c of the
    API tests (an edge from a to b)."""
    ok("tab", "new", name)
    ok("tab", "switch", name)
    if notes:
        made = ok(
            "apply",
            stdin=json.dumps(
                {
                    "entities": [
                        {"kind": "text", "text": t, "canvasX": x, "canvasY": 0}
                        for t, x in [("one", 0), ("two", 300), ("three", 600)]
                    ]
                }
            ),
        )["created"]
        return made
    return []


CASES = []


def case(fn):
    CASES.append(fn)
    return fn


# --- reaching the app -------------------------------------------------------


@case
def help_and_discovery():
    ran = cli()
    check(ran.out.startswith("usage: specular <verb>"), ran.out)
    check("print-pdf" not in ran.out and "record" not in ran.out, "cut verbs are gone from help")
    refused(
        "Specular app is not available. Launch the app first.",
        "canvas",
        env={"SPECULAR_DISCOVERY_FILE": os.path.join(SPACE, "..", "no-such-file.json")},
    )
    with open(DISCOVERY) as f:
        found = json.load(f)
    stale = os.path.join(SPACE, "..", "stale.json")
    with open(stale, "w") as f:
        json.dump(dict(found, secret="wrong"), f)
    refused("stale credentials", "canvas", env={"SPECULAR_DISCOVERY_FILE": stale})
    with open(stale, "w") as f:
        json.dump(dict(found, version="0"), f)
    refused("version mismatch", "canvas", env={"SPECULAR_DISCOVERY_FILE": stale})


# --- tabs -------------------------------------------------------------------


@case
def tabs_are_made_shown_written_in_the_background_and_deleted():
    before = ok("tab")
    home = before["activeTab"]
    made = ok("tab", "new", "Back room")
    equal(made["name"], "Back room", "tab new")
    refused("usage: specular tab new <name>", "tab", "new")
    refused("usage: specular tab switch", "tab", "switch")
    refused("usage: specular tab [new", "tab", "rename")
    equal(ok("tab")["activeTab"], home, "a new tab leaves the user where they are")

    # A write to a background canvas lands there and is saved there.
    text = ok("add", "note", "behind", "--at", "40,40", "--tab", "Back room")["created"][0]
    check(text not in nodes(), "a --tab write stays off the active canvas")
    back = ok("canvas", "--tab", "Back room")
    check(any(n["id"] == text for n in back["nodes"]), "the background canvas holds it")
    saved("Back room", lambda d: text in saved_ids(d), "the background note")
    refused("--tab is not supported", "unlink", "e1", "--tab", "Back room")
    refused("Back nowhere", "canvas", "--tab", "Back nowhere")

    equal(ok("tab", "switch", "Back room")["activeTab"]["name"], "Back room", "switch")
    equal(ok("tab", "switch", home["id"])["activeTab"], home, "switch back by id")
    deleted = ok("tab", "delete", "Back room")
    equal(deleted["deleted"]["name"], "Back room", "tab delete")
    check(all(t["name"] != "Back room" for t in ok("tab")["tabs"]), "it is gone")


# --- add, update, delete ----------------------------------------------------


@case
def add_makes_notes_documents_pages_and_files():
    fresh("Adds")
    note = ok("add", "note", "short", "--at", "0,0", "--color", "4")["created"][0]
    check(note.startswith("text_"), note)
    equal((node(note)["text"], node(note)["color"], node(note)["x"]), ("short", "4", 0), "note")

    long = "# Findings\n\nA note with a heading is a Document, not a sticky."
    doc = ok("add", "note", long)["created"][0]
    check(doc.startswith("file_"), doc)
    equal(node(doc)["file"], "Findings.md", "the document's file")
    path = os.path.join(SPACE, "Findings.md")
    for _ in range(30):
        if os.path.exists(path):
            break
        time.sleep(0.1)
    with open(path) as f:
        check("A note with a heading" in f.read(), "the .md is written")

    page = ok("add", "page", "https://example.com/a", "--preset", "9", "--landscape")["created"][0]
    p = node(page)
    equal((p["url"], p["presetIndex"]), ("https://example.com/a", 9), "page")
    check(p["width"] > p["height"], f"landscape turns the preset: {p['width']}x{p['height']}")
    laptop = ok("add", "page", "http://localhost:4321/garden", "--at", "0,2000")["created"][0]
    equal((node(laptop)["presetIndex"], node(laptop)["width"]), (6, 1280), "Laptop by default")
    refused("url must be a full URL", "add", "page", "/garden")

    png = os.path.join(SPACE, "..", "dot.png")
    write_png(png, 320, 180)
    picture = ok("add", "file", png)["created"][0]
    equal((node(picture)["width"], node(picture)["height"]), (320, 180), "sized from the image")
    check(node(picture)["file"].startswith("assets/"), node(picture)["file"])
    check(os.listdir(os.path.join(SPACE, "assets")), "copied into assets")
    refused("does-not-exist.png", "add", "file", "/nowhere/does-not-exist.png")

    for args, line in [
        (["add"], "usage: specular add <page|note|file>"),
        (["add", "page"], "usage: specular add page <url>"),
        (["add", "note"], "usage: specular add note <text>"),
        (["add", "file"], "usage: specular add file <path>"),
    ]:
        refused(line, *args)
    # New things are placed clear of each other.
    a, b = nodes()[note], nodes()[doc]
    check(a["x"] + a["width"] <= b["x"] or a["y"] + a["height"] <= b["y"] or b["x"] + b["width"] <= a["x"],
          "an unplaced note lands clear of the one before")
    saved("Adds", lambda d: {note, doc, page, picture} <= saved_ids(d), "the four creates")


@case
def update_moves_resizes_and_changes_fields_and_delete_removes():
    a, b, c = fresh("Edits", notes=True)
    equal(ok("update", a, "--at", "40,60", "--text", "changed", "--color", "2"),
          {"created": [], "updated": [a]}, "update")
    n = node(a)
    equal((n["x"], n["y"], n["text"], n["color"]), (40, 60, "changed", "2"), "updated note")
    ok("update", b, "--size", "300,240")
    equal((node(b)["width"], node(b)["height"]), (300, 240), "resized")
    refused("usage: specular update <id>", "update")
    refused("missing or unknown kind", "update", "text_nope", "--text", "x")

    page = ok("add", "page", "https://example.com/one", "--at", "0,600")["created"][0]
    ok("update", page, "--url", "https://example.com/two", "--preset", "1")
    for _ in range(20):
        if node(page)["url"] == "https://example.com/two":
            break
        time.sleep(0.1)
    equal((node(page)["url"], node(page)["presetIndex"]), ("https://example.com/two", 1), "page")

    equal(ok("delete", c), {"deleted": [c]}, "delete")
    check(node(c) is None, "deleted")
    equal(ok("delete", "text_nope"), {"deleted": []}, "an unknown id deletes nothing")
    equal(ok("delete", "--json", stdin=json.dumps([{"id": b}])), {"deleted": [b]}, "delete --json")
    refused("usage: specular delete <id>", "delete")
    undo()
    check(node(b) is not None, "undo brings it back")
    saved("Edits", lambda d: c not in saved_ids(d) and b in saved_ids(d), "the delete and the undo")


# --- the JSON door ------------------------------------------------------------


@case
def apply_creates_every_kind_in_one_undo_step():
    fresh("Apply kinds")
    seed = ok("apply", stdin=json.dumps({"entities": [
        {"kind": "text", "text": "a", "canvasX": 0, "canvasY": 0},
        {"kind": "text", "text": "b", "canvasX": 200, "canvasY": 0},
    ]}))["created"]
    done = ok("apply", stdin=json.dumps({"entities": [
        {"kind": "page", "url": "https://example.com", "presetIndex": 9, "canvasX": 0, "canvasY": 400},
        {"kind": "shape", "shapeKind": "rectangle", "text": "box", "canvasX": 400, "canvasY": 400},
        {"kind": "drawing", "canvasX": 800, "canvasY": 400, "width": 100, "height": 100,
         "strokes": [{"id": "s1", "color": "#000", "width": 2,
                      "points": [{"x": 0, "y": 0}, {"x": 50, "y": 50}]}]},
        {"kind": "group", "entityIds": seed, "label": "pair"},
        {"kind": "text", "text": "alpha", "canvasX": 200, "canvasY": 800},
        {"kind": "note", "text": "short note", "canvasX": 400, "canvasY": 800},
    ]}))
    created = done["created"]
    equal(len(created), 6, "created")
    all_nodes = nodes()
    equal([all_nodes[i]["type"] for i in created],
          ["link", "shape", "drawing", "group", "text", "text"], "types")
    equal((all_nodes[created[0]]["width"], all_nodes[created[0]]["height"]), (466, 678), "page size")
    equal(all_nodes[created[3]]["label"], "pair", "group label")
    equal(all_nodes[seed[0]]["specular"]["parentGroupId"], created[3], "grouped")
    equal((all_nodes[created[4]]["color"], all_nodes[created[4]]["x"]), ("3", 200), "a yellow sticky")

    # A bad item refuses the whole patch.
    refused("entities[1]: missing or unknown kind", "apply", stdin=json.dumps({"entities": [
        {"kind": "text", "text": "good", "canvasX": 0, "canvasY": 0},
        {"kind": "not-a-kind"},
    ]}))
    equal(len(nodes()), 8, "nothing leaked from the refused patch")
    undo()
    equal(set(nodes()), set(seed), "the six-item patch was one undo step")


@case
def apply_updates_edges_and_deletes_by_id():
    a, b, _c = fresh("Apply edits", notes=True)
    done = ok("apply", stdin=json.dumps({
        "entities": [{"kind": "text", "text": "new", "canvasX": 0, "canvasY": 400}, {"id": a, "text": "after"}],
        "edges": [{"fromEntityId": a, "toEntityId": b, "kind": "connection", "label": "v1"}],
    }))
    equal((len(done["created"]), done["updated"], len(done["edges"])), (1, [a], 1), "answer")
    edge = done["edges"][0]
    equal(node(a)["text"], "after", "an update with no kind")
    relabeled = ok("apply", stdin=json.dumps({"edges": [{"id": edge, "label": "v2"}]}))
    equal(relabeled["edges"], [edge], "edge patched in place")
    equal([e["label"] for e in ok("canvas")["edges"]], ["v2"], "one edge, relabelled")
    undo()
    equal([e["label"] for e in ok("canvas")["edges"]], ["v1"], "undo restores the label")

    gone = ok("apply", stdin=json.dumps({"delete": [a]}))
    equal(gone["deleted"], [a], "delete in a patch")
    equal(ok("canvas")["edges"], [], "an edge that lost an end goes too")

    for patch, want in [
        ("{not json", "apply: patch is not valid JSON"),
        (json.dumps({"layout": {"kind": "pile"}}), "apply: layout.kind: expected 'row' | 'column' | 'grid'"),
        (json.dumps({"entities": {}}), "entities: expected an array"),
        ("", "usage: specular apply"),
    ]:
        refused(want, "apply", stdin=patch)


@case
def apply_round_trips_styling_and_moves_carry_their_contents():
    fresh("Apply styling")
    shape = ok("apply", stdin=json.dumps({"entities": [{
        "kind": "shape", "shapeKind": "rectangle", "canvasX": 0, "canvasY": 0,
        "fillStyle": "solid", "borderStyle": "solid", "borderColor": "4", "strokeWidth": 3,
        "textAlign": "center", "textVerticalAlign": "middle",
    }]}))["created"][0]

    def styling():
        n = node(shape)
        s = n["specular"]
        return [s["fillStyle"], s["textAlign"], s["textVerticalAlign"],
                n["borderStyle"], n["borderColor"], n["strokeWidth"]]

    seeded = ["solid", "center", "middle", "solid", "4", 3]
    equal(styling(), seeded, "seeded styling")
    ok("apply", stdin=json.dumps({"entities": [{
        "id": shape, "fillStyle": "none", "borderStyle": "none", "borderColor": "1",
        "strokeWidth": 1, "textAlign": "right", "textVerticalAlign": "bottom",
    }]}))
    equal(styling(), ["none", "right", "bottom", "none", "1", 1], "restyled")
    undo()
    equal(styling(), seeded, "undo restores the styling")

    drawing = ok("apply", stdin=json.dumps({"entities": [{
        "kind": "drawing", "canvasX": 0, "canvasY": 400, "width": 100, "height": 100,
        "strokes": [{"id": "s", "color": "#000", "width": 2, "points": [{"x": 10, "y": 420}]}],
    }]}))["created"][0]
    before = node(drawing)
    ok("update", drawing, "--at", "300,800")
    moved = node(drawing)
    dx, dy = moved["x"] - before["x"], moved["y"] - before["y"]
    equal((moved["x"], moved["y"]), (300, 800), "drawing moved")
    p0, p1 = before["strokes"][0]["points"][0], moved["strokes"][0]["points"][0]
    equal((p1["x"] - p0["x"], p1["y"] - p0["y"]), (dx, dy), "its strokes went with it")

    texts = ok("apply", stdin=json.dumps({"entities": [
        {"kind": "text", "text": "a", "canvasX": 1000, "canvasY": 0},
        {"kind": "text", "text": "b", "canvasX": 1040, "canvasY": 0},
    ]}))["created"]
    group = ok("group", *texts, "--label", "g")
    gid = group["id"]
    equal(group["entityIds"], texts, "group answer")
    g0, a0 = node(gid), node(texts[0])
    ok("update", gid, "--at", f"{g0['x'] + 100},{g0['y'] + 50}")
    a1 = node(texts[0])
    equal((a1["x"] - a0["x"], a1["y"] - a0["y"]), (100, 50), "a group move carries its members")
    undo()
    equal((node(texts[0]), node(gid)), (a0, g0), "and is one undo step")

    ok("apply", stdin=json.dumps({"entities": [{"id": gid, "text": "renamed"}]}))
    equal(node(gid)["label"], "renamed", "text is a group's label")
    ok("apply", stdin=json.dumps({"entities": [{"id": gid, "text": "ignored", "label": "explicit"}]}))
    equal(node(gid)["label"], "explicit", "an explicit label wins")
    ok("update", gid, "--color", "4")
    equal((node(gid)["color"], node(gid)["groupColor"]), ("4", "4"), "a group's colour")


@case
def apply_lays_out_by_directive_and_upsert_still_works():
    a, b, c = fresh("Layouts", notes=True)
    made = ok("apply", stdin=json.dumps({
        "entities": [{"kind": "text", "text": f"n{i}"} for i in range(4)],
        "layout": {"kind": "grid", "cols": 2, "gap": "xs", "originX": 0, "originY": 600},
    }))["created"]
    at = [(node(i)["x"], node(i)["y"]) for i in made]
    equal(at, [(0, 600), (220, 600), (0, 820), (220, 820)], "a 2-column grid at the origin")

    ok("apply", stdin=json.dumps({
        "entities": [{"id": c}, {"id": a}, {"id": b}],
        "layout": {"kind": "row", "gap": 20},
    }))
    equal([node(i)["x"] for i in (c, a, b)], [0, 220, 440], "existing entities rearranged")

    ran = cli("upsert", "--json", stdin=json.dumps({
        "layout": {"kind": "column", "near": a, "gap": 20},
        "items": [{"kind": "text", "text": "below", "canvasX": 999}],
    }))
    below = ran.json()["created"][0]
    equal((node(below)["x"], node(below)["y"]), (node(a)["x"], node(a)["y"] + 220), "near a, below it")
    check("canvasX/canvasY ignored" in ran.err, "a position under a directive is warned about")
    plain = ok("upsert", "--json", "--layout", "column", stdin=json.dumps([
        {"kind": "text", "text": "x"}, {"kind": "text", "text": "y"}]))
    xs = [node(i)["x"] for i in plain["created"]]
    equal(xs[0], xs[1], "--layout column stacks them")
    refused("usage: specular upsert --json", "upsert")
    refused("upsert: expected an array of items or { layout, items }", "upsert", "--json", stdin="{}")
    refused("upsert: layout.cols: expected positive integer", "upsert", "--json",
            stdin=json.dumps({"layout": {"kind": "grid", "cols": 0}, "items": []}))


# --- arrange and connect -------------------------------------------------------


@case
def link_group_auto_layout_arrange_and_focus():
    a, b, c = fresh("Arranging", notes=True)
    edge = ok("link", a, b, "--label", "about")["edgeIds"]
    equal(len(edge), 1, "link")
    equal(ok("canvas")["edges"][0]["label"], "about", "the edge's label")
    piped = ok("link", stdin=json.dumps([{"fromEntityId": b, "toEntityId": c}]))["edgeIds"]
    equal(len(piped), 1, "link from stdin")
    refused("usage: specular link", "link", a)
    equal(ok("unlink", *edge, *piped)["deletedEdgeIds"], edge + piped, "unlink")
    equal(ok("unlink", a)["deletedEdgeIds"], [], "unlink leaves an entity alone")
    check(node(a) is not None, "still there")

    group = ok("group", a, b)
    equal(node(group["id"])["label"], "Group", "a group with no label")
    equal(ok("ungroup", group["id"])["entityIds"], [a, b], "ungroup")
    refused("usage: specular ungroup", "ungroup")

    managed = ok("auto-layout", a, b, c, "--gap", "40", "--label", "Row")
    equal((managed["layoutMode"], managed["managedLayout"], managed["layoutGap"]), ("row", True, 40), "auto-layout")
    equal([node(i)["x"] for i in (a, b, c)], [0, 240, 480], "packed at the gap")
    ok("update", managed["id"], "--gap", "20")
    equal([node(i)["x"] for i in (a, b, c)], [0, 220, 440], "update --gap reflows it")
    undo()
    undo()
    equal([node(i)["x"] for i in (a, b, c)], [0, 300, 600], "two undo steps back")
    refused("usage: specular auto-layout", "auto-layout")

    equal(ok("arrange", "row", c, a, b, "--gap", "20"), {"changed": True}, "arrange --gap")
    equal([node(i)["x"] for i in (a, b, c)], [0, 220, 440], "packed in reading order")
    ok("arrange", "column", a, b, c, "--gap", "m")
    equal({node(i)["x"] for i in (a, b, c)}, {0}, "a column")
    refused("usage: specular arrange <row|column|grid|flow>", "arrange", "pile")
    http("/selection/deselect")
    equal(ok("arrange", "row"), {"changed": False}, "no ids and nothing selected")

    focused = ok("focus", a)
    check(focused.get("ok", True), f"focus: {focused}")
    refused("usage: specular focus", "focus")


@case
def screens_are_laid_out_as_a_flow_along_their_edges():
    fresh("Flow")
    screen = lambda ref: {"kind": "shape", "ref": ref, "width": 390, "height": 844}
    made = ok(
        "apply",
        stdin=json.dumps(
            {
                "entities": [screen("cart"), screen("pay"), screen("done")],
                "edges": [
                    {"fromEntityId": "cart", "toEntityId": "pay", "label": "Checkout"},
                    {"fromEntityId": "pay", "toEntityId": "done"},
                    {"fromEntityId": "done", "toEntityId": "cart", "label": "Shop again"},
                ],
                "layout": {"kind": "flow"},
            }
        ),
    )
    cart, pay, done = (made["refs"][ref] for ref in ("cart", "pay", "done"))
    xs = [node(i)["x"] for i in (cart, pay, done)]
    check(xs[0] + 390 <= xs[1] and xs[1] + 390 <= xs[2], f"a column a screen, left to right: {xs}")
    routes = [edge.get("route", []) for edge in ok("canvas")["edges"]]
    equal(sum(1 for route in routes if len(route) >= 2), 3, "every edge is routed")

    equal(ok("arrange", "flow", cart, pay, done, "--dir", "TB"), {"changed": True}, "arrange flow")
    ys = [node(i)["y"] for i in (cart, pay, done)]
    check(ys[0] + 844 <= ys[1] and ys[1] + 844 <= ys[2], f"a row a screen, top to bottom: {ys}")
    refused("layout.dir: expected 'LR' | 'TB'", "arrange", "flow", cart, pay, "--dir", "up")
    refused("apply: layout.dir", "apply", stdin=json.dumps({"layout": {"kind": "flow", "dir": 3}}))
    undo()
    undo()
    equal(ok("canvas")["nodes"], [], "the flow and its layout are two undo steps")


@case
def placement_selection_and_breakpoints():
    fresh("Breakpoints")
    spot = ok("find-placement", "--width", "300", "--height", "200")
    check({"canvasX", "canvasY", "reason"} <= set(spot), f"find-placement: {spot}")
    equal(ok("selection"), {}, "nothing selected")
    refused("No page specified and nothing is selected.", "snapshot")

    done = ok("breakpoints", "https://example.com/shop", "--no-focus")
    equal(done["resolvedPresets"], ["iPhone 16", "iPad mini", "Desktop"], "the default presets")
    pages = done["pageIds"]
    all_nodes = nodes()
    equal({all_nodes[p]["url"] for p in pages}, {"https://example.com/shop"}, "one url")
    equal(all_nodes[done["groupId"]]["label"], "Breakpoints: example.com/shop", "group label")
    equal(len(done["edgeIds"]), 2, "the pages are linked in a row")
    named = ok("breakpoints", "https://example.com/x", "--presets", "Desktop", "--label", "Wide")
    equal((named["resolvedPresets"], all_nodes.get(named["groupId"], node(named["groupId"]))["label"]),
          (["Desktop"], "Wide"), "named presets and label")
    refused("usage: specular breakpoints <url>", "breakpoints")
    undo()
    undo()
    equal(nodes(), {}, "each breakpoints run is one undo step")
    saved("Breakpoints", lambda d: not d.get("nodes"), "the empty canvas")


# --- comments -----------------------------------------------------------------


@case
def comments_are_made_answered_listed_and_read():
    a, b, _c = fresh("Comments", notes=True)
    made = ok("annotate", "tighten", "this")
    cid = made["id"]
    check(cid.startswith("ann_"), cid)
    equal((made["text"], made["author"], made["status"], made["anchor"]["type"]),
          ("tighten this", "agent", "pending", "canvas"), "annotate")
    refused("usage: specular annotate <text>", "annotate")

    listed = ok("annotations")["annotations"]
    equal([n["id"] for n in listed], [cid], "listed while unresolved")
    check("metadata" not in listed[0] or "summary" not in listed[0]["metadata"], "a point has no summary")
    equal(ok("ack", cid)["status"], "acknowledged", "ack")
    equal(ok("reply", cid, "on", "it")["replies"][0]["text"], "on it", "reply")
    refused("usage: specular reply", "reply", cid)
    equal(ok("dismiss", cid, "--reason", "dup")["metadata"], {"dismissReason": "dup"}, "dismiss")
    equal(ok("annotations")["annotations"], [], "a dismissed comment is resolved")
    equal(len(ok("annotations", "--status", "dismissed")["annotations"]), 1, "--status")
    equal(len(ok("annotations", "--all")["annotations"]), 1, "--all")
    equal(ok("resolve", cid)["status"], "resolved", "resolve")
    refused("usage: specular ack", "ack")

    region = ok("annotate-selection", "compare", "these", "--ids", f"{a},{b}")
    equal(region["selectionEntityIds"], [a, b], "annotate-selection")
    equal(region["anchor"]["type"], "region", "a region")
    stub = [n for n in ok("annotations")["annotations"] if n["id"] == region["id"]][0]
    equal(stub["metadata"]["summary"]["type"], "region", "the list summarises a region")
    detail = cli("annotation", region["id"])
    body = json.loads(detail.out.split("\n}\n")[0] + "\n}")
    equal(body["text"], "compare these", "annotation detail")
    equal([m["text"] for m in body["selection"]["members"]], ["one", "two"], "its members")
    check("regionScreenshot" not in body.get("metadata", {}), "the picture is not in the JSON")
    lines = detail.out.strip().splitlines()
    if "(image: use Read(" in lines[-1]:
        check(os.path.exists(lines[-2]), f"the picture was saved: {lines[-2]}")
    plain = cli("annotation", region["id"], "--no-screenshot")
    check("(image:" not in plain.out, "--no-screenshot")
    refused("usage: specular annotation <id>", "annotation")
    refused("ann_nope", "annotation", "ann_nope")

    page = ok("add", "page", "https://example.com/a/", "--at", "0,600")["created"][0]
    on_page = ok("annotate", "here", "--page-id", page)
    equal(on_page["anchor"]["type"], "page", "--page-id")
    by_page = ok("annotations", "--page-id", page)["annotations"]
    equal([n["id"] for n in by_page], [on_page["id"]], "listed by page")
    equal(by_page[0]["pageAnchor"]["pageUrl"], "https://example.com/a/", "with its page")
    equal(len(ok("annotations", "--url", "https://example.com/a")["annotations"]), 1, "by address")
    saved("Comments", lambda d: any(n.get("id") == on_page["id"] for n in
                                    d.get("specular", {}).get("annotations", []))
          or on_page["id"] in json.dumps(d), "the comments")


# --- browse verbs with no page ------------------------------------------------


@case
def browse_verbs_refuse_what_they_cannot_drive():
    a, _b, _c = fresh("Browse", notes=True)
    refused("open: blocked", "open", "https://example.com", "-f", a)
    refused("close: blocked", "close", "-f", a)
    refused("usage: specular click <ref>", "click")
    refused("usage: specular fill <ref> <text>", "fill", "@e1")
    refused("usage: specular select <ref> <value>", "select", "@e1")
    refused("not found", "snapshot", "-i", "-f", "page_nope")


def write_png(path, width, height):
    import struct
    import zlib

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    rows = b"".join(b"\x00" + b"\x30\x80\xd0" * width for _ in range(height))
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def main():
    passed, failed = 0, []
    for fn in CASES:
        if ONLY and fn.__name__ not in ONLY:
            continue
        try:
            fn()
            print(f"PASS {fn.__name__}", flush=True)
            passed += 1
        except Exception as error:  # noqa: BLE001 - every failure is reported the same way
            detail = str(error) if isinstance(error, Failed) else traceback.format_exc()
            print(f"FAIL {fn.__name__}\n  " + detail.replace("\n", "\n  "), flush=True)
            failed.append(fn.__name__)
        finally:
            # The next case starts from the canvas the space opened with.
            try:
                ok("tab", "switch", "Home")
            except Failed:
                pass
    print(f"{passed} passed, {len(failed)} failed")
    sys.exit(1 if failed else 0)


main()
