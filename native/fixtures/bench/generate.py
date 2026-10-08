#!/usr/bin/env python3
"""Writes the bench canvases next to this file. Deterministic: run it again
after changing a layout and commit what it writes.

    python3 native/fixtures/bench/generate.py

Every canvas is laid out so the bench's start camera (pan 40,40, zoom 0.25,
a 6400x4000 canvas window at 1600x1000) opens on its densest part with text
still large enough to draw.
"""

import json
import math
import random
from pathlib import Path

HERE = Path(__file__).parent
WORDS = (
    "canvas page sticky layout margin spacing colour type scale grid breakpoint "
    "header footer sidebar card button modal hover focus contrast rhythm align "
    "review ship draft idea question later maybe compare mobile desktop tablet"
).split()
URLS = [
    "https://en.wikipedia.org/wiki/Spatial_computing",
    "https://github.com/trending",
    "https://developer.mozilla.org/en-US/docs/Web/CSS",
    "https://news.ycombinator.com/",
    "https://doc.rust-lang.org/book/",
    "https://www.w3.org/TR/css-grid-1/",
    "https://react.dev/learn",
    "https://docs.python.org/3/tutorial/",
]


def words(rng, low, high):
    return " ".join(rng.choice(WORDS) for _ in range(rng.randint(low, high)))


def sticky(rng, index, x, y):
    node = {
        "id": f"sticky-{index}",
        "type": "text",
        "x": x,
        "y": y,
        "width": 200,
        "height": 200,
        "text": f"Note {index}\n{words(rng, 6, 18)}",
    }
    if index % 8:
        node["color"] = str(index % 8)
    return node


def stickies(count, columns, seed, origin=(0, 0), prefix=0):
    rng = random.Random(seed)
    return [
        sticky(rng, prefix + i, origin[0] + (i % columns) * 240, origin[1] + (i // columns) * 240)
        for i in range(count)
    ]


def drawing(rng, index, x, y, width=280, height=180):
    """One pen or highlighter stroke: a wobbling line of 60 to 140 points."""
    points = []
    steps = rng.randint(60, 140)
    phase, waves = rng.random() * math.tau, rng.uniform(1.5, 5.0)
    for step in range(steps):
        t = step / (steps - 1)
        points.append(
            {
                "x": round(x + t * width, 2),
                "y": round(y + height / 2 + math.sin(phase + t * waves * math.tau) * height * 0.4 * (0.4 + 0.6 * t), 2),
            }
        )
    stroke = {"id": f"s-draw-{index}", "color": str(1 + index % 7), "width": [2, 4, 8][index % 3], "points": points}
    stroke["brushType"] = "pen"
    if index % 5 == 4:
        stroke["brushType"] = "highlight"
        stroke["width"] = 16
    xs, ys = [p["x"] for p in points], [p["y"] for p in points]
    return {
        "id": f"draw-{index}",
        "type": "drawing",
        "x": min(xs) - 1,
        "y": round(min(ys) - 1, 2),
        "width": max(xs) - min(xs) + 2,
        "height": round(max(ys) - min(ys) + 2, 2),
        "strokes": [stroke],
    }


def drawings(count, columns, seed, origin=(0, 0)):
    rng = random.Random(seed)
    return [
        drawing(rng, i, origin[0] + (i % columns) * 320, origin[1] + (i // columns) * 220) for i in range(count)
    ]


def shape(index, x, y, prefix="shape"):
    kinds = ["rectangle", "ellipse", "diamond", "triangle", "rounded"]
    return {
        "id": f"{prefix}-{index}",
        "type": "shape",
        "x": x,
        "y": y,
        "width": 160,
        "height": 100,
        "shapeKind": kinds[index % len(kinds)],
        "text": f"Step {index}",
        "color": str(1 + index % 7),
        "strokeWidth": 2,
    }


def edges(count, columns, origin=(0, 0), prefix="edge"):
    """`count` edges, each between its own pair of shapes."""
    nodes, links = [], []
    for i in range(count):
        x = origin[0] + (i % columns) * 520
        y = origin[1] + (i // columns) * 200
        nodes.append(shape(2 * i, x, y, f"{prefix}-shape"))
        nodes.append(shape(2 * i + 1, x + 300, y + (40 if i % 2 else -20), f"{prefix}-shape"))
        link = {"id": f"{prefix}-{i}", "fromNode": nodes[-2]["id"], "toNode": nodes[-1]["id"], "toEnd": "arrow"}
        if i % 4 == 1:
            link["lineStyle"] = "dashed"
        if i % 4 == 2:
            link["label"] = f"link {i}"
        if i % 4 == 3:
            link["fromEnd"] = "arrow"
            link["strokeWidth"] = 4
        links.append(link)
    return nodes, links


def note_text(rng, index):
    lines = [f"# Document {index}", "", words(rng, 20, 40), ""]
    for section in range(4):
        lines += [f"## Section {section + 1}", "", f"Some **bold {words(rng, 1, 2)}** and *italic* text with `code`. {words(rng, 25, 60)}", ""]
        lines += [f"- {words(rng, 3, 9)}" for _ in range(rng.randint(3, 6))] + [""]
        if section % 2:
            lines += [f"> {words(rng, 8, 20)}", "", "```", f"let {rng.choice(WORDS)} = {rng.randint(1, 99)};", "```", ""]
    return "\n".join(lines)


def documents(count, columns, seed, origin=(0, 0)):
    """`count` Documents over ten markdown files in `notes/`."""
    rng = random.Random(seed)
    (HERE / "notes").mkdir(exist_ok=True)
    for index in range(10):
        (HERE / "notes" / f"doc-{index}.md").write_text(note_text(rng, index))
    return [
        {
            "id": f"doc-{i}",
            "type": "file",
            "x": origin[0] + (i % columns) * 560,
            "y": origin[1] + (i // columns) * 680,
            "width": 520,
            "height": 640,
            "file": f"notes/doc-{i % 10}.md",
        }
        for i in range(count)
    ]


def pages(count, columns, origin=(0, 0)):
    return [
        {
            "id": f"page-{i:02}",
            "type": "link",
            "x": origin[0] + (i % columns) * 1360,
            "y": origin[1] + (i // columns) * 880,
            "width": 1280,
            "height": 800,
            "url": URLS[i % len(URLS)],
            "metadata": {"pageSizeMode": "custom", "customSize": {"width": 1280, "height": 800}},
        }
        for i in range(count)
    ]


def mixed():
    """Twenty pages with three hundred items around and over them."""
    nodes = pages(20, 5)
    # A sticky on each page's corner, so z-order breaks batches as a real
    # annotated board does.
    rng = random.Random(7)
    nodes += [sticky(rng, i, (i % 5) * 1360 + 1000, (i // 5) * 880 + 40) for i in range(20)]
    nodes += stickies(130, 26, 8, origin=(0, 3560), prefix=100)
    nodes += drawings(50, 10, 9, origin=(0, 4800))
    edge_nodes, links = edges(30, 6, origin=(3400, 4800))
    nodes += edge_nodes
    nodes += documents(10, 10, 10, origin=(0, 6200))
    return nodes, links


def whole(value):
    """Whole floats as integers, the way the app writes them back."""
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, list):
        return [whole(item) for item in value]
    if isinstance(value, dict):
        return {key: whole(item) for key, item in value.items()}
    return value


def write(name, nodes, links=()):
    path = HERE / f"{name}.canvas"
    path.write_text(json.dumps(whole({"nodes": nodes, "edges": list(links)}), separators=(",", ":")) + "\n")
    print(f"{path.name}: {len(nodes)} nodes, {len(links)} edges")


def main():
    write("stickies-500", stickies(500, 25, 1))
    write("stickies-2000", stickies(2000, 50, 2))
    write("drawings-300", drawings(300, 20, 3))
    write("edges-200", *edges(200, 12))
    write("documents-50", documents(50, 10, 5))
    write("mixed", *mixed())


if __name__ == "__main__":
    main()
