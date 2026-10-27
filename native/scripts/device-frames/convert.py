#!/usr/bin/env python3
"""Convert the Flutter `device_frame` painters to SVG.

Usage: convert.py <flutter_device_preview checkout> [--out DIR] [--commit SHA]

The package ships no SVG, only Dart `CustomPainter` code. This script runs the
`paint()` body of every `frame.g.dart` through a small interpreter for the
subset of Dart those files use, records the draw calls, and writes one SVG per
device plus `manifest.json` and the upstream licence. See README.md.
"""

import argparse
import json
import math
import re
import subprocess
import sys
from pathlib import Path as FsPath

REPO = "github.com/aloisdeniel/flutter_device_preview"
DEVICES_DIR = "device_frame/lib/src/devices"
DEFAULT_OUT = FsPath(__file__).resolve().parents[2] / "crates/specular-shell/assets/device-frames"


class Unsupported(Exception):
    pass


# ---------------------------------------------------------------- tokens

TOKEN = re.compile(
    r"""
    (?P<ws>\s+|//[^\n]*|/\*.*?\*/)
  | (?P<num>0[xX][0-9a-fA-F]+|(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)
  | (?P<str>'[^']*'|"[^"]*")
  | (?P<id>[A-Za-z_]\w*)
  | (?P<op>\.\.|[.,()\[\]{}=*/+\-;:])
    """,
    re.X | re.S,
)


def tokenize(src):
    out, pos = [], 0
    while pos < len(src):
        m = TOKEN.match(src, pos)
        if not m:
            raise Unsupported(f"cannot tokenize at: {src[pos:pos + 40]!r}")
        pos = m.end()
        if m.lastgroup != "ws":
            out.append((m.lastgroup, m.group()))
    return out


# ---------------------------------------------------------------- parser

class Parser:
    """Recursive descent over the Dart subset, producing tuple AST nodes."""

    def __init__(self, tokens):
        self.t = tokens
        self.i = 0

    def peek(self, k=0):
        return self.t[self.i + k] if self.i + k < len(self.t) else ("eof", "")

    def at(self, value):
        return self.peek()[1] == value and self.peek()[0] in ("op", "id")

    def take(self, value=None):
        tok = self.peek()
        if value is not None and tok[1] != value:
            raise Unsupported(f"expected {value!r}, got {tok[1]!r}")
        self.i += 1
        return tok

    def statements(self):
        out = []
        while self.peek()[0] != "eof":
            out.append(self.statement())
        return out

    def statement(self):
        if self.at("final") or self.at("var"):
            self.take()
        # `Type name = expr;` or `name = expr;` after `final`.
        if self.peek()[0] == "id" and self.peek(1)[0] == "id" and self.peek(2)[1] == "=":
            self.take()
        if self.peek()[0] == "id" and self.peek(1)[1] == "=":
            name = self.take()[1]
            self.take("=")
            value = self.expr()
            self.take(";")
            return ("let", name, value)
        target = self.expr()
        if self.at("="):
            self.take()
            value = self.expr()
            self.take(";")
            return ("assign", target, value)
        self.take(";")
        return ("do", target)

    def expr(self):
        node = self.additive()
        sections = []
        while self.at(".."):
            self.take()
            name = self.take()[1]
            if self.at("("):
                args, kwargs = self.arguments()
                sections.append(("call", name, args, kwargs))
            else:
                self.take("=")
                sections.append(("set", name, self.additive()))
        return ("cascade", node, sections) if sections else node

    def additive(self):
        node = self.multiplicative()
        while self.at("+") or self.at("-"):
            op = self.take()[1]
            node = ("bin", op, node, self.multiplicative())
        return node

    def multiplicative(self):
        node = self.unary()
        while self.at("*") or self.at("/"):
            op = self.take()[1]
            node = ("bin", op, node, self.unary())
        return node

    def unary(self):
        if self.at("-"):
            self.take()
            return ("neg", self.unary())
        return self.postfix()

    def postfix(self):
        node = self.primary()
        while True:
            if self.at("."):
                self.take()
                node = ("get", node, self.take()[1])
            elif self.at("("):
                args, kwargs = self.arguments()
                node = ("call", node, args, kwargs)
            else:
                return node

    def primary(self):
        kind, text = self.take()
        if kind == "id" and text == "const":
            return self.primary()
        if kind == "num":
            return ("lit", int(text, 16) if text[:2].lower() == "0x" else float(text))
        if kind == "str":
            return ("lit", text[1:-1])
        if kind == "id":
            return ("name", text)
        if text == "(":
            node = self.expr()
            self.take(")")
            return node
        if text == "[":
            items = []
            while not self.at("]"):
                items.append(self.expr())
                if self.at(","):
                    self.take()
            self.take("]")
            return ("list", items)
        raise Unsupported(f"unexpected token {text!r}")

    def arguments(self):
        self.take("(")
        args, kwargs = [], {}
        while not self.at(")"):
            if self.peek()[0] == "id" and self.peek(1)[1] == ":":
                name = self.take()[1]
                self.take(":")
                kwargs[name] = self.expr()
            else:
                args.append(self.expr())
            if self.at(","):
                self.take()
        self.take(")")
        return args, kwargs


# ---------------------------------------------------------------- runtime

class Color:
    def __init__(self, argb):
        self.argb = int(argb) & 0xFFFFFFFF

    def withAlpha(self, alpha):
        return Color((int(alpha) << 24) | (self.argb & 0xFFFFFF))

    def withOpacity(self, opacity):
        return self.withAlpha(dart_round(opacity * 255))

    @property
    def hex(self):
        return f"#{self.argb & 0xFFFFFF:06X}"

    @property
    def opacity(self):
        return (self.argb >> 24) / 255


class DartPath:
    def __init__(self):
        self.cmds = []
        self.fillType = "nonZero"

    def moveTo(self, x, y):
        self.cmds.append(("M", x, y))

    def lineTo(self, x, y):
        self.cmds.append(("L", x, y))

    def cubicTo(self, x1, y1, x2, y2, x, y):
        self.cmds.append(("C", x1, y1, x2, y2, x, y))

    def quadraticBezierTo(self, x1, y1, x, y):
        self.cmds.append(("Q", x1, y1, x, y))

    def arcToPoint(self, end, radius=(0.0, 0.0), rotation=0.0, largeArc=False, clockwise=True):
        if radius[0] == 0 or radius[1] == 0:
            self.cmds.append(("L", end[0], end[1]))
        else:
            self.cmds.append(("A", radius[0], radius[1], rotation, int(largeArc), int(clockwise), end[0], end[1]))

    def close(self):
        self.cmds.append(("Z",))

    def d(self):
        return " ".join(c[0] + " ".join(num(v) for v in c[1:]) for c in self.cmds)


class Paint:
    def __init__(self):
        self.style = "fill"
        self.color = Color(0xFF000000)
        self.strokeWidth = 0.0
        self.shader = None
        self.maskFilter = None

    def snapshot(self):
        copy = Paint()
        copy.__dict__.update(self.__dict__)
        return copy


class Canvas:
    def __init__(self):
        self.draws = []

    def drawPath(self, path, paint):
        if path.fillType != "nonZero" and paint.style == "fill":
            raise Unsupported("frame path with a non-default fillType")
        self.draws.append(("path", list(path.cmds), paint.snapshot()))

    def drawRect(self, rect, paint):
        self.draws.append(("rrect", rect, (0.0,) * 4, paint.snapshot()))

    def drawRRect(self, rrect, paint):
        self.draws.append(("rrect", rrect[0], rrect[1], paint.snapshot()))

    def drawCircle(self, center, radius, paint):
        self.draws.append(("circle", center, radius, paint.snapshot()))


class Size:
    def __init__(self, width, height):
        self.width = float(width)
        self.height = float(height)


def rrect_from_rect_and_corners(rect, topLeft=(0.0, 0.0), topRight=(0.0, 0.0), bottomRight=(0.0, 0.0), bottomLeft=(0.0, 0.0)):
    corners = (topLeft, topRight, bottomRight, bottomLeft)
    if any(rx != ry for rx, ry in corners):
        raise Unsupported("elliptical RRect corner")
    return (rect, tuple(rx for rx, _ in corners))


def linear_gradient(p0, p1, colors, stops=None, *rest):
    if rest:
        raise Unsupported("gradient tile mode or matrix")
    if stops is None:
        stops = [0.0, 1.0]
    if len(stops) != len(colors):
        raise Unsupported("gradient stops do not match colors")
    return ("linear", p0, p1, tuple(colors), tuple(stops))


STATICS = {
    "Path": DartPath,
    "Paint": Paint,
    "Color": Color,
    "Size": Size,
    "Offset": lambda x, y: (x, y),
    "Radius.elliptical": lambda x, y: (x, y),
    "Radius.circular": lambda r: (r, r),
    "Rect.fromLTWH": lambda l, t, w, h: (l, t, w, h),
    "RRect.fromRectAndCorners": rrect_from_rect_and_corners,
    "ui.Gradient.linear": linear_gradient,
    "MaskFilter.blur": lambda style, sigma: (style, sigma),
}

CONSTANTS = {
    "PaintingStyle.fill": "fill",
    "PaintingStyle.stroke": "stroke",
    "BlurStyle.normal": "normal",
    "BlurStyle.inner": "inner",
    "PathFillType.evenOdd": "evenOdd",
    "PathFillType.nonZero": "nonZero",
    "Colors.black": Color(0xFF000000),
    "Colors.white": Color(0xFFFFFFFF),
    "true": True,
    "false": False,
}

PAINT_FIELDS = {"style", "color", "strokeWidth", "shader", "maskFilter"}


def dart_round(value):
    return int(math.floor(abs(value) + 0.5)) * (1 if value >= 0 else -1)


def dotted(node):
    if node[0] == "name":
        return node[1]
    if node[0] == "get":
        head = dotted(node[1])
        return head and f"{head}.{node[2]}"
    return None


class Interpreter:
    def __init__(self, env):
        self.env = dict(env)

    def run(self, statements):
        for stmt in statements:
            if stmt[0] == "let":
                self.env[stmt[1]] = self.eval(stmt[2])
            elif stmt[0] == "assign":
                target = stmt[1]
                if target[0] != "get":
                    raise Unsupported("assignment to a non-member")
                self.set(self.eval(target[1]), target[2], self.eval(stmt[2]))
            else:
                self.eval(stmt[1])

    def set(self, obj, name, value):
        if isinstance(obj, Paint) and name not in PAINT_FIELDS:
            raise Unsupported(f"Paint.{name}")
        if isinstance(obj, DartPath) and name != "fillType":
            raise Unsupported(f"Path.{name}")
        setattr(obj, name, value)

    def call(self, func, args, kwargs):
        return func(*[self.eval(a) for a in args], **{k: self.eval(v) for k, v in kwargs.items()})

    def eval(self, node):
        kind = node[0]
        if kind == "lit":
            return node[1]
        if kind == "list":
            return [self.eval(n) for n in node[1]]
        if kind == "neg":
            return -self.eval(node[1])
        if kind == "bin":
            a, b = self.eval(node[2]), self.eval(node[3])
            if node[1] == "+":
                return a + b
            if node[1] == "-":
                return a - b
            return a * b if node[1] == "*" else a / b
        if kind in ("name", "get"):
            name = dotted(node)
            if name in CONSTANTS:
                return CONSTANTS[name]
            if kind == "name":
                if name not in self.env:
                    raise Unsupported(f"unknown name {name}")
                return self.env[name]
            return getattr(self.eval(node[1]), node[2])
        if kind == "call":
            name = dotted(node[1])
            if name in STATICS:
                return self.call(STATICS[name], node[2], node[3])
            if node[1][0] != "get":
                raise Unsupported(f"unknown function {name}")
            receiver, method = self.eval(node[1][1]), node[1][2]
            if isinstance(receiver, (int, float)) and method == "round":
                return dart_round(receiver)
            if not hasattr(receiver, method):
                raise Unsupported(f"{type(receiver).__name__}.{method}()")
            return self.call(getattr(receiver, method), node[2], node[3])
        if kind == "cascade":
            target = self.eval(node[1])
            for section in node[2]:
                if section[0] == "set":
                    self.set(target, section[1], self.eval(section[2]))
                elif not hasattr(target, section[1]):
                    raise Unsupported(f"{type(target).__name__}.{section[1]}()")
                else:
                    self.call(getattr(target, section[1]), section[2], section[3])
            return target
        raise Unsupported(f"node {kind}")


# ---------------------------------------------------------------- geometry

def arc_to_cubics(x0, y0, rx, ry, rotation, large, sweep, x, y):
    """SVG endpoint arc to cubic segments (SVG 1.1 implementation notes F.6)."""
    if (x0, y0) == (x, y):
        return []
    phi = math.radians(rotation)
    cos_p, sin_p = math.cos(phi), math.sin(phi)
    dx, dy = (x0 - x) / 2, (y0 - y) / 2
    x1p, y1p = cos_p * dx + sin_p * dy, -sin_p * dx + cos_p * dy
    rx, ry = abs(rx), abs(ry)
    scale = (x1p / rx) ** 2 + (y1p / ry) ** 2
    if scale > 1:
        rx, ry = rx * math.sqrt(scale), ry * math.sqrt(scale)
    denom = (rx * y1p) ** 2 + (ry * x1p) ** 2
    factor = math.sqrt(max(0.0, ((rx * ry) ** 2 - denom) / denom))
    if large == sweep:
        factor = -factor
    cxp, cyp = factor * rx * y1p / ry, -factor * ry * x1p / rx
    cx = cos_p * cxp - sin_p * cyp + (x0 + x) / 2
    cy = sin_p * cxp + cos_p * cyp + (y0 + y) / 2
    theta = math.atan2((y1p - cyp) / ry, (x1p - cxp) / rx)
    delta = math.atan2((-y1p - cyp) / ry, (-x1p - cxp) / rx) - theta
    if sweep and delta < 0:
        delta += 2 * math.pi
    elif not sweep and delta > 0:
        delta -= 2 * math.pi

    def point(angle):
        ex, ey = rx * math.cos(angle), ry * math.sin(angle)
        return cx + cos_p * ex - sin_p * ey, cy + sin_p * ex + cos_p * ey

    def tangent(angle):
        ex, ey = -rx * math.sin(angle), ry * math.cos(angle)
        return cos_p * ex - sin_p * ey, sin_p * ex + cos_p * ey

    count = max(1, math.ceil(abs(delta) / (math.pi / 2) - 1e-9))
    step = delta / count
    k = 4 / 3 * math.tan(step / 4)
    out = []
    for n in range(count):
        a0, a1 = theta + n * step, theta + (n + 1) * step
        p0, p3 = point(a0), point(a1)
        t0, t3 = tangent(a0), tangent(a1)
        out.append((p0, (p0[0] + k * t0[0], p0[1] + k * t0[1]), (p3[0] - k * t3[0], p3[1] - k * t3[1]), p3))
    return out


def subpaths(cmds):
    """Path commands as subpaths of cubic segments (p0, c1, c2, p3)."""
    out, current, start, pen = [], [], None, (0.0, 0.0)

    def line(to):
        current.append((pen, pen, to, to))

    for c in cmds:
        if c[0] == "M":
            if current:
                out.append(current)
            current, start, pen = [], (c[1], c[2]), (c[1], c[2])
        elif c[0] == "L":
            line((c[1], c[2]))
            pen = (c[1], c[2])
        elif c[0] == "C":
            current.append((pen, (c[1], c[2]), (c[3], c[4]), (c[5], c[6])))
            pen = (c[5], c[6])
        elif c[0] == "Q":
            q, to = (c[1], c[2]), (c[3], c[4])
            c1 = (pen[0] + 2 / 3 * (q[0] - pen[0]), pen[1] + 2 / 3 * (q[1] - pen[1]))
            c2 = (to[0] + 2 / 3 * (q[0] - to[0]), to[1] + 2 / 3 * (q[1] - to[1]))
            current.append((pen, c1, c2, to))
            pen = to
        elif c[0] == "A":
            current.extend(arc_to_cubics(pen[0], pen[1], *c[1:]))
            pen = (c[6], c[7])
        elif c[0] == "Z" and start is not None:
            if pen != start:
                line(start)
            pen = start
    if current:
        out.append(current)
    return out


def cubic_at(seg, t):
    u = 1 - t
    return tuple(
        u * u * u * seg[0][i] + 3 * u * u * t * seg[1][i] + 3 * u * t * t * seg[2][i] + t * t * t * seg[3][i]
        for i in (0, 1)
    )


def cubic_extrema(seg, axis):
    p0, p1, p2, p3 = (p[axis] for p in seg)
    a, b, c = -p0 + 3 * p1 - 3 * p2 + p3, 2 * (p0 - 2 * p1 + p2), p1 - p0
    if abs(a) < 1e-12:
        roots = [-c / b] if abs(b) > 1e-12 else []
    else:
        disc = b * b - 4 * a * c
        roots = [] if disc < 0 else [(-b + s * math.sqrt(disc)) / (2 * a) for s in (1, -1)]
    return [t for t in roots if 0 < t < 1]


def bounds(segments):
    """Tight bounding box (x0, y0, x1, y1) of cubic segments."""
    xs, ys = [], []
    for seg in segments:
        ts = [0.0, 1.0] + cubic_extrema(seg, 0) + cubic_extrema(seg, 1)
        for t in ts:
            x, y = cubic_at(seg, t)
            xs.append(x)
            ys.append(y)
    return min(xs), min(ys), max(xs), max(ys)


def corner_radius(segments, box):
    """Radius of the circle that meets the outline where each corner's 45°
    diagonal crosses it; the median of the four corners."""
    points = [cubic_at(seg, n / 64) for seg in segments for n in range(65)]
    radii = []
    for cx, sx in ((box[0], 1), (box[2], -1)):
        for cy, sy in ((box[1], 1), (box[3], -1)):
            best = None
            for a, b in zip(points, points[1:]):
                fa = (a[0] - cx) * sx - (a[1] - cy) * sy
                fb = (b[0] - cx) * sx - (b[1] - cy) * sy
                if fa == fb or fa * fb > 0:
                    continue
                t = fa / (fa - fb)
                inset = (a[0] + t * (b[0] - a[0]) - cx) * sx
                if best is None or inset < best:
                    best = inset
            radii.append((best or 0.0) / (1 - math.sqrt(0.5)))
    radii.sort()
    return (radii[1] + radii[2]) / 2


# ---------------------------------------------------------------- SVG

def num(value, places=3):
    text = f"{round(float(value), places):.{places}f}".rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def rrect_cmds(rect, radii):
    l, t, w, h = rect
    tl, tr, br, bl = (min(r, w / 2, h / 2) for r in radii)
    path = DartPath()
    path.moveTo(l + tl, t)
    path.lineTo(l + w - tr, t)
    path.arcToPoint((l + w, t + tr), radius=(tr, tr))
    path.lineTo(l + w, t + h - br)
    path.arcToPoint((l + w - br, t + h), radius=(br, br))
    path.lineTo(l + bl, t + h)
    path.arcToPoint((l, t + h - bl), radius=(bl, bl))
    path.lineTo(l, t + tl)
    path.arcToPoint((l + tl, t), radius=(tl, tl))
    path.close()
    return path.cmds


class Svg:
    def __init__(self, frame, screen, source=None):
        self.frame = frame
        self.source = source
        self.defs = []
        self.ids = {}
        self.body = []
        w, h = num(frame.width), num(frame.height)
        # The frame rectangle and the screen path under even-odd leave
        # everything but the screen, and keep the screen path's own holes.
        self.defs.append(
            f'<clipPath id="outside-screen"><path clip-rule="evenodd" d="M0 0H{w}V{h}H0Z {screen.d()}"/></clipPath>'
        )

    def define(self, prefix, body):
        """Add a def once; `body` holds `{id}` where its id goes."""
        if body not in self.ids:
            self.ids[body] = f"{prefix}{sum(1 for v in self.ids.values() if v.startswith(prefix))}"
            self.defs.append(body.replace("{id}", self.ids[body]))
        return self.ids[body]

    def paint_attrs(self, paint, box):
        role = paint.style
        if paint.shader is not None:
            _, p0, p1, colors, stops = paint.shader
            stop_tags = "".join(
                f'<stop offset="{num(s, 4)}" stop-color="{c.hex}"'
                + (f' stop-opacity="{num(c.opacity, 4)}"' if c.opacity < 1 else "")
                + "/>"
                for c, s in zip(colors, stops)
            )
            gid = self.define(
                "g",
                f'<linearGradient id="{{id}}" gradientUnits="userSpaceOnUse" x1="{num(p0[0])}" y1="{num(p0[1])}"'
                f' x2="{num(p1[0])}" y2="{num(p1[1])}">{stop_tags}</linearGradient>',
            )
            attrs = [f'{role}="url(#{gid})"']
        else:
            attrs = [f'{role}="{paint.color.hex}"']
            if paint.color.opacity < 1:
                attrs.append(f'{role}-opacity="{num(paint.color.opacity, 4)}"')
        grow = 0.0
        if role == "stroke":
            if paint.strokeWidth <= 0:
                raise Unsupported("hairline stroke")
            attrs = ['fill="none"'] + attrs + [f'stroke-width="{num(paint.strokeWidth)}"']
            grow = paint.strokeWidth / 2
        if paint.maskFilter is not None:
            style, sigma = paint.maskFilter
            # Explicit user-space region: a stroked straight line has an
            # empty bounding box, which would switch the filter off.
            grow += 3 * sigma + 1
            x0, y0 = math.floor(box[0] - grow), math.floor(box[1] - grow)
            x1, y1 = math.ceil(box[2] + grow), math.ceil(box[3] + grow)
            inner = '<feComposite operator="in" in2="SourceAlpha"/>' if style == "inner" else ""
            fid = self.define(
                "f",
                f'<filter id="{{id}}" filterUnits="userSpaceOnUse" x="{x0}" y="{y0}" width="{x1 - x0}"'
                f' height="{y1 - y0}" color-interpolation-filters="sRGB">'
                f'<feGaussianBlur stdDeviation="{num(sigma)}"/>{inner}</filter>',
            )
            attrs.append(f'filter="url(#{fid})"')
        return " ".join(attrs)

    def draw(self, draw):
        paint = draw[-1]
        if draw[0] == "circle":
            (cx, cy), r = draw[1], draw[2]
            box = (cx - r, cy - r, cx + r, cy + r)
            shape = f'<circle cx="{num(cx)}" cy="{num(cy)}" r="{num(r)}"'
        elif draw[0] == "rrect" and len(set(draw[2])) == 1:
            (l, t, w, h), r = draw[1], draw[2][0]
            box = (l, t, l + w, t + h)
            shape = f'<rect x="{num(l)}" y="{num(t)}" width="{num(w)}" height="{num(h)}"'
            if r:
                shape += f' rx="{num(r)}"'
        else:
            path = DartPath()
            path.cmds = rrect_cmds(draw[1], draw[2]) if draw[0] == "rrect" else draw[1]
            segments = [seg for sub in subpaths(path.cmds) for seg in sub]
            if not segments:
                return
            box = bounds(segments)
            shape = f'<path d="{path.d()}"'
        self.body.append(f"{shape} {self.paint_attrs(paint, box)}/>")

    def text(self):
        w, h = num(self.frame.width), num(self.frame.height)
        origin = (
            "Specular's own drawing, built by native/scripts/device-frames/convert.py from parts of"
            " the MIT-licensed Flutter device_frame package."
            if self.source
            else "Generated by native/scripts/device-frames/convert.py from the MIT-licensed"
            " Flutter device_frame package."
        )
        lines = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
            f"<!-- {origin} Do not edit by hand. -->",
            "<defs>",
            *self.defs,
            "</defs>",
            '<g clip-path="url(#outside-screen)">',
            *self.body,
            "</g>",
            "</svg>",
        ]
        return "\n".join(lines) + "\n"


# ---------------------------------------------------------------- devices

def block_after(src, marker, open_ch, close_ch):
    start = src.index(open_ch, src.index(marker))
    depth = 0
    for i in range(start, len(src)):
        if src[i] == open_ch:
            depth += 1
        elif src[i] == close_ch:
            depth -= 1
            if depth == 0:
                return src[start + 1:i]
    raise Unsupported(f"unbalanced {open_ch} after {marker}")


def evaluate(src, env=None):
    interp = Interpreter(env or {})
    interp.run(Parser(tokenize(src)).statements())
    return interp.env


def tidy(value, places=4):
    value = round(float(value), places)
    return int(value) if value == int(value) else value


def body_outline(draws, frame):
    """The device's body: the largest filled outline, without the buttons
    that stick out of it. Returns its box cut to the frame and its corner
    radius, or None when no fill stands out as the body.

    A fill that covers the whole frame is a backdrop or a cut-out, not the
    body, and so is left out."""
    width, height = frame.width, frame.height
    found = []
    for draw in draws:
        paint = draw[-1]
        if paint.style != "fill":
            continue
        if draw[0] == "path":
            outlines = subpaths(draw[1])
        elif draw[0] == "rrect":
            outlines = subpaths(rrect_cmds(draw[1], draw[2]))
        else:
            continue
        for outline in outlines:
            box = bounds(outline)
            cut = (max(box[0], 0.0), max(box[1], 0.0), min(box[2], width), min(box[3], height))
            if cut[2] <= cut[0] or cut[3] <= cut[1]:
                continue
            if cut[2] - cut[0] >= width - 1e-6 and cut[3] - cut[1] >= height - 1e-6:
                continue
            area = (cut[2] - cut[0]) * (cut[3] - cut[1])
            found.append((area, cut, corner_radius(outline, box), draw))
    if not found:
        return None
    area, cut, radius, draw = max(found, key=lambda entry: entry[0])
    # A body fills most of its frame; anything smaller is a part of one. A
    # laptop's lid is the least of them, at 0.78 of a frame as wide as its
    # base.
    if area < 0.75 * width * height:
        return None
    return cut, radius, draw


def body_fields(draws, frame):
    body = body_outline(draws, frame)
    if body is None:
        return {}
    cut, radius, _ = body
    return {
        "body": {
            "x": tidy(cut[0], 3),
            "y": tidy(cut[1], 3),
            "width": tidy(cut[2] - cut[0], 3),
            "height": tidy(cut[3] - cut[1], 3),
        },
        "bodyCornerRadius": tidy(radius, 2),
    }


# ---------------------------------------------------------------- light

# The devices that also get `<name>-light.svg`. The drawings are one dark
# colourway; the light one is the same draws with some fills changed.
LIGHT_DEVICES = (
    "google-pixel-9",
    "google-pixel-9-pro-xl",
    "ipad-air-4",
    "ipad-mini",
    "ipad-pro-11-inches-m4",
    "ipad-pro-11inches",
    "ipad-pro-12inches-gen4",
    "ipad-pro-13-inches-m4",
    "iphone-13",
    "iphone-15-pro",
    "iphone-15-pro-max",
    "iphone-16",
    "iphone-16-plus",
    "iphone-16-pro",
    "iphone-16-pro-max",
    "iphone-air",
    "iphone-se",
    "macbook-pro",
    "samsung-galaxy-s25",
)

# (what the fill is, its dark colour) -> its light colour. Anything not
# listed keeps its colour: glass, the island, cameras, sensors, shading.
#
# body    the device's main outline, which shows as its edge
# button  a fill that sticks out of the body
# face    the front, on a device with a forehead and a chin
# ring    a fill with a hole: the home button's ring, or a highlight band
#         round the front
# edge    a small fill on the body's edge, which is an antenna break
#
# A body that shows as a wide band takes the lighter grey and one that
# shows as a thin line the darker, so the line still reads on white.
LIGHT_FILLS = {
    ("body", "#213744"): "#B4B6B9",
    ("body", "#272727"): "#E3E4E5",
    ("body", "#3A4245"): "#C9CACC",
    ("body", "#3B393B"): "#C9CACC",
    ("body", "#434343"): "#E3E4E5",
    ("body", "#454C60"): "#E3E4E5",
    ("body", "#4E4E4E"): "#E3E4E5",
    ("body", "#4F5359"): "#E3E4E5",
    ("button", "#121515"): "#C9CACC",
    ("button", "#1C3343"): "#C9CACC",
    ("button", "#213744"): "#C9CACC",
    ("button", "#272727"): "#C9CACC",
    ("button", "#3B393B"): "#C9CACC",
    ("button", "#434343"): "#C9CACC",
    ("button", "#454C60"): "#C9CACC",
    ("button", "#4E4E4E"): "#C9CACC",
    ("button", "#4F5359"): "#C9CACC",
    ("edge", "#36454C"): "#A9ABAE",
    ("face", "#121515"): "#F4F4F5",
    ("ring", "#262C2D"): "#C9CACC",
    ("ring", "#8EADC1"): "#DADBDD",
    # The thumb scoop in a laptop's base.
    (None, "#4E4F53"): "#C4C5C8",
}

# A gradient stop's dark colour -> its light colour: the aluminium of a
# laptop's base, and the shade under it.
LIGHT_STOPS = {
    "#08090D": "#8E8F93",
    "#303135": "#B9BABD",
    "#333438": "#BDBEC1",
    "#35363A": "#C0C1C4",
    "#4D4E52": "#D2D3D5",
    "#4E4F53": "#D4D5D7",
    "#535458": "#D9DADC",
    "#818286": "#E8E9EA",
}

# A front the screen covers this much of is glass to its edge, and stays
# dark. Less, and it is a painted face. The iPad mini's glass is the widest
# share of a front at 0.78, and the iPhone SE's face is 0.72.
GLASS_FRONT = 0.75


def box_area(box):
    return max(box[2] - box[0], 0.0) * max(box[3] - box[1], 0.0)


def fill_role(draw, body_draw, body_box, screen_box):
    """What a solid fill is, as LIGHT_FILLS names it, or None."""
    paint = draw[-1]
    if paint.style != "fill" or paint.shader is not None or paint.maskFilter is not None:
        return None
    if draw is body_draw:
        return "body"
    if draw[0] == "path":
        outlines = subpaths(draw[1])
    elif draw[0] == "rrect":
        outlines = subpaths(rrect_cmds(draw[1], draw[2]))
    else:
        return None
    boxes = [bounds(outline) for outline in outlines]
    if not boxes:
        return None
    box = (min(b[0] for b in boxes), min(b[1] for b in boxes), max(b[2] for b in boxes), max(b[3] for b in boxes))
    reach = 0.5
    if (
        box[0] < body_box[0] - reach
        or box[1] < body_box[1] - reach
        or box[2] > body_box[2] + reach
        or box[3] > body_box[3] + reach
    ):
        return "button"
    if len(boxes) == 2:
        outer, inner = sorted(boxes, key=box_area, reverse=True)
        if outer[0] < inner[0] and outer[1] < inner[1] and outer[2] > inner[2] and outer[3] > inner[3]:
            return "ring"
    if box_area(box) >= 0.8 * box_area(body_box):
        return "glass" if box_area(screen_box) >= GLASS_FRONT * box_area(box) else "face"
    if (
        box[0] < body_box[0] + reach
        or box[1] < body_box[1] + reach
        or box[2] > body_box[2] - reach
        or box[3] > body_box[3] - reach
    ):
        return "edge"
    return None


def light_draws(draws, frame, screen_box):
    """The draws in the light colourway."""
    body = body_outline(draws, frame)
    if body is None:
        raise Unsupported("no body outline to colour")
    body_box, _, body_draw = body
    out = []
    for draw in draws:
        paint = draw[-1]
        role = fill_role(draw, body_draw, body_box, screen_box)
        light = LIGHT_FILLS.get((role, paint.color.hex))
        if light is not None:
            paint = paint.snapshot()
            paint.color = Color((paint.color.argb & 0xFF000000) | int(light[1:], 16))
            draw = (*draw[:-1], paint)
        elif paint.shader is not None and paint.style == "fill" and paint.maskFilter is None:
            kind, p0, p1, colors, stops = paint.shader
            lit = tuple(
                Color((c.argb & 0xFF000000) | int(LIGHT_STOPS[c.hex][1:], 16)) if c.hex in LIGHT_STOPS else c
                for c in colors
            )
            if lit != colors:
                paint = paint.snapshot()
                paint.shader = (kind, p0, p1, lit, stops)
                draw = (*draw[:-1], paint)
        out.append(draw)
    return out


class Device:
    """One drawing: its draws in frame units and what the manifest says."""

    def __init__(self, name, frame, screen, draws, logical, ratio, platform, kind, source=None):
        self.name = name
        self.frame = frame
        self.screen = screen
        self.draws = draws
        self.logical = logical
        self.ratio = ratio
        self.platform = platform
        self.kind = kind
        self.source = source
        subs = subpaths(screen.cmds)
        if len(subs) > 1 and screen.fillType != "evenOdd":
            raise Unsupported("multi-part screen path that is not even-odd")
        self.screen_outline = max(subs, key=lambda sub: box_area(bounds(sub)))
        self.screen_box = bounds(self.screen_outline)


def read_device(folder):
    device_src = (folder / "device.dart").read_text()
    info = block_after(device_src, "DeviceInfo(", "(", ")")

    def field(name):
        match = re.search(rf"\b{name}:\s*([^\n]*?),?\s*\n", info)
        if not match:
            raise Unsupported(f"device.dart has no {name}")
        return evaluate(f"v = {match.group(1)};")["v"]

    frame, logical, ratio = field("frameSize"), field("screenSize"), field("pixelRatio")
    platform = re.search(r"TargetPlatform\.(\w+)", info).group(1).lower()
    kind = re.search(r"DeviceType\.(\w+)", info).group(1).lower()

    screen_src = (folder / "screen.g.dart").read_text()
    screen = evaluate(screen_src[screen_src.index("_screenPath"):])["_screenPath"]

    canvas = Canvas()
    frame_src = (folder / "frame.g.dart").read_text()
    evaluate(block_after(frame_src, "void paint(", "{", "}"), {"canvas": canvas, "size": frame})

    name = folder.name.lower().replace("_", "-")
    return Device(name, frame, screen, canvas.draws, logical, ratio, platform, kind)


# Painters written by hand, which have no `screen.g.dart`. Upstream lays a
# wallpaper and a window over the panel and calls the window the screen.
# Here the screen is the whole panel, `screen` in frame units, and the paint
# body is read up to `until`, where the wallpaper starts. `folder` is under
# DEVICES_DIR.
HAND_WRITTEN = (
    {
        "folder": "macos/macbook_pro",
        "screen": (346.68, 98.2, 2298.82, 1437.32),
        "until": "// Generic custom",
    },
)


def read_hand_written(spec, devices_dir):
    folder = devices_dir / spec["folder"]
    info = block_after((folder / "device.dart").read_text(), "DeviceInfo(", "(", ")")

    def field(name):
        match = re.search(rf"\b{name}:\s*([^\n]*?),?\s*\n", info)
        if not match:
            raise Unsupported(f"device.dart has no {name}")
        return evaluate(f"v = {match.group(1)};")["v"]

    frame, ratio = field("frameSize"), field("pixelRatio")
    platform = re.search(r"TargetPlatform\.(\w+)", info).group(1).lower()
    kind = re.search(r"DeviceType\.(\w+)", info).group(1).lower()

    left, top, width, height = spec["screen"]
    screen = DartPath()
    screen.moveTo(left, top)
    screen.lineTo(left + width, top)
    screen.lineTo(left + width, top + height)
    screen.lineTo(left, top + height)
    screen.close()

    canvas = Canvas()
    paint = block_after((folder / "frame.dart").read_text(), "void paint(", "{", "}")
    evaluate(paint[:paint.index(spec["until"])], {"canvas": canvas, "size": frame})

    name = folder.name.lower().replace("_", "-")
    logical = Size(width / ratio, height / ratio)
    return Device(name, frame, screen, canvas.draws, logical, ratio, platform, kind)


def emit(device):
    """A device's manifest row and its files, as {file name: text}."""
    name, frame, box = device.name, device.frame, device.screen_box
    svg = Svg(frame, device.screen, device.source)
    for draw in device.draws:
        svg.draw(draw)
    files = {f"{name}.svg": svg.text()}
    if name in LIGHT_DEVICES:
        light = Svg(frame, device.screen, device.source)
        for draw in light_draws(device.draws, frame, box):
            light.draw(draw)
        files[f"{name}-light.svg"] = light.text()
    row = {
        "name": name,
        "file": f"{name}.svg",
        **({"lightFile": f"{name}-light.svg"} if name in LIGHT_DEVICES else {}),
        "frame": {"width": tidy(frame.width), "height": tidy(frame.height)},
        "screen": {
            "x": tidy(box[0], 3),
            "y": tidy(box[1], 3),
            "width": tidy(box[2] - box[0], 3),
            "height": tidy(box[3] - box[1], 3),
        },
        "screenCornerRadius": tidy(corner_radius(device.screen_outline, box), 2),
        **body_fields(device.draws, frame),
        "logicalScreen": {"width": tidy(device.logical.width), "height": tidy(device.logical.height)},
        "pixelRatio": tidy(device.ratio),
        "platform": device.platform,
        "kind": device.kind,
        **({"source": device.source} if device.source else {}),
    }
    return row, files


# ---------------------------------------------------------------- our own

# Drawings the package does not have, made from one it does. `from` is the
# drawing whose bezel, corners, edge and buttons are kept at their real
# size; the screen between them is resized to `logical`. `points_per_inch`
# is each device's logical points to the inch (pixels per inch over the
# pixel ratio), which is what puts the two at the same real scale.
# `camera` borrows another drawing's lens for the bezel of one that has none.
OWN_DEVICES = (
    {
        "name": "ipad-mini",
        "from": "ipad-air-4",
        "logical": (744, 1133),
        "pixelRatio": 2,
        "points_per_inch": 163,
        "from_points_per_inch": 132,
        "camera": {"from": "ipad-pro-11-inches-m4", "points_per_inch": 132, "colour": "#131423"},
    },
    {
        "name": "iphone-air",
        "from": "iphone-16-pro",
        "logical": (420, 912),
        "pixelRatio": 3,
        "points_per_inch": 460 / 3,
        "from_points_per_inch": 460 / 3,
    },
)

# A part this close to the middle of the frame, as a share of the frame,
# stays in the middle when the frame is resized: an island, a speaker.
CENTRED = 0.05


def draw_cmds(draw):
    if draw[0] == "path":
        return draw[1]
    if draw[0] == "rrect":
        return rrect_cmds(draw[1], draw[2])
    (cx, cy), r = draw[1], draw[2]
    path = DartPath()
    path.moveTo(cx - r, cy)
    path.arcToPoint((cx + r, cy), radius=(r, r))
    path.arcToPoint((cx - r, cy), radius=(r, r))
    path.close()
    return path.cmds


def split_subpaths(cmds):
    out = []
    for c in cmds:
        if c[0] == "M" or not out:
            out.append([])
        out[-1].append(c)
    return out


def cmd_points(c):
    """A command's points as (index of x in the command) for each."""
    return {"M": (1,), "L": (1,), "C": (1, 3, 5), "Q": (1, 3), "A": (6,), "Z": ()}[c[0]]


def map_paint(paint, point, scale=1.0):
    paint = paint.snapshot()
    paint.strokeWidth *= scale
    if paint.shader is not None:
        kind, p0, p1, colors, stops = paint.shader
        paint.shader = (kind, point(p0), point(p1), colors, stops)
    if paint.maskFilter is not None:
        paint.maskFilter = (paint.maskFilter[0], paint.maskFilter[1] * scale)
    return paint


class Resize:
    """Moves a drawing's parts for a screen that is `grow` larger.

    The frame is cut in four at the middle of the screen. A part keeps its
    shape and its distance from its own corner. A part that spans the cut is
    stretched across it, which only ever lengthens straight edges. A part in
    the middle stays in the middle, with whatever sits inside it."""

    def __init__(self, device, grow):
        self.grow = grow
        self.extent = (device.frame.width, device.frame.height)
        box = device.screen_box
        self.cut = ((box[0] + box[2]) / 2, (box[1] + box[3]) / 2)
        self.middles = ([], [])
        outlines = [sub for draw in device.draws for sub in split_subpaths(draw_cmds(draw))]
        outlines += split_subpaths(device.screen.cmds)
        for sub in outlines:
            box = bounds([seg for part in subpaths(sub) for seg in part])
            for axis in (0, 1):
                if self.spans(box, axis) is False and self.in_middle(box, axis):
                    self.middles[axis].append(box)

    def spans(self, box, axis):
        return box[axis + 2] - box[axis] >= self.extent[axis] / 2

    def in_middle(self, box, axis):
        centre = (box[axis] + box[axis + 2]) / 2
        return box[axis] < self.cut[axis] < box[axis + 2] and abs(centre - self.cut[axis]) < CENTRED * self.extent[axis]

    def shift(self, box, axis):
        """How far a part with this box moves on an axis, or None when it
        is stretched across the cut."""
        if self.spans(box, axis):
            return None
        inside = any(
            m[0] <= box[0] and m[1] <= box[1] and m[2] >= box[2] and m[3] >= box[3] for m in self.middles[axis]
        )
        if inside:
            return self.grow[axis] / 2
        return self.grow[axis] if (box[axis] + box[axis + 2]) / 2 > self.cut[axis] else 0.0

    def cmds(self, cmds):
        out = []
        for sub in split_subpaths(cmds):
            segments = [seg for part in subpaths(sub) for seg in part]
            if not segments:
                out.extend(sub)
                continue
            shifts = [self.shift(bounds(segments), axis) for axis in (0, 1)]
            pen = None
            for c in sub:
                moved = list(c)
                points = [(c[i], c[i + 1]) for i in cmd_points(c)]
                for axis in (0, 1):
                    if shifts[axis] is None and c[0] in "CQA":
                        sides = {p[axis] > self.cut[axis] for p in [pen, *points]}
                        if len(sides) > 1:
                            raise Unsupported("a curve crosses the middle of the screen")
                for i in cmd_points(c):
                    for axis in (0, 1):
                        value = c[i + axis]
                        by = shifts[axis]
                        if by is None:
                            by = self.grow[axis] if value > self.cut[axis] else 0.0
                        moved[i + axis] = value + by
                if points:
                    pen = points[-1]
                out.append(tuple(moved))
        return out

    def draw(self, draw):
        cmds = draw_cmds(draw)
        segments = [seg for part in subpaths(cmds) for seg in part]
        box = bounds(segments)
        shifts = [self.shift(box, axis) for axis in (0, 1)]

        def point(p):
            return tuple(
                p[axis] + (shifts[axis] if shifts[axis] is not None else self.grow[axis] if p[axis] > self.cut[axis] else 0.0)
                for axis in (0, 1)
            )

        return ("path", self.cmds(cmds), map_paint(draw[-1], point))


def lens_draws(device, colour):
    """The draws of a camera lens: the first fill of `colour` and every
    draw inside it."""
    lens = None
    out = []
    for draw in device.draws:
        segments = [seg for part in subpaths(draw_cmds(draw)) for seg in part]
        box = bounds(segments)
        if lens is None:
            if draw[-1].style != "fill" or draw[-1].shader is not None or draw[-1].color.hex != colour:
                continue
            lens = box
        elif not (lens[0] <= box[0] and lens[1] <= box[1] and lens[2] >= box[2] and lens[3] >= box[3]):
            continue
        out.append(draw)
    if lens is None:
        raise Unsupported(f"{device.name} has no lens of {colour}")
    return out, lens


def placed(draw, scale, origin, to):
    """A draw scaled about `origin` and moved so that `origin` lands on `to`."""

    def point(p):
        return tuple((p[axis] - origin[axis]) * scale + to[axis] for axis in (0, 1))

    cmds = []
    for c in draw_cmds(draw):
        moved = list(c)
        for i in cmd_points(c):
            moved[i], moved[i + 1] = point((c[i], c[i + 1]))
        if c[0] == "A":
            moved[1], moved[2] = c[1] * scale, c[2] * scale
        cmds.append(tuple(moved))
    return ("path", cmds, map_paint(draw[-1], point, scale))


def units_per_inch(device, points_per_inch):
    box = device.screen_box
    return (box[2] - box[0]) / device.logical.width * points_per_inch


def synthesise(spec, devices):
    base = devices[spec["from"]]
    box = base.screen_box
    # Frame units to the logical point at this device's real size, from the
    # base drawing's screen width.
    per_point = (box[2] - box[0]) / base.logical.width * spec["from_points_per_inch"] / spec["points_per_inch"]
    logical = Size(*spec["logical"])
    grow = (
        logical.width * per_point - (box[2] - box[0]),
        logical.height * per_point - (box[3] - box[1]),
    )
    resize = Resize(base, grow)
    frame = Size(base.frame.width + grow[0], base.frame.height + grow[1])
    screen = DartPath()
    screen.cmds = resize.cmds(base.screen.cmds)
    screen.fillType = base.screen.fillType
    draws = [resize.draw(draw) for draw in base.draws]
    device = Device(spec["name"], frame, screen, draws, logical, spec["pixelRatio"], base.platform, base.kind, "specular")

    camera = spec.get("camera")
    if camera:
        donor = devices[camera["from"]]
        lens, lens_box = lens_draws(donor, camera["colour"])
        scale = units_per_inch(base, spec["from_points_per_inch"]) / units_per_inch(donor, camera["points_per_inch"])
        body = body_outline(draws, frame)
        if body is None:
            raise Unsupported("no body to put a camera on")
        # On the short edge at the top, midway between the body's edge and
        # the screen's.
        to = ((device.screen_box[0] + device.screen_box[2]) / 2, (body[0][1] + device.screen_box[1]) / 2)
        origin = ((lens_box[0] + lens_box[2]) / 2, (lens_box[1] + lens_box[3]) / 2)
        device.draws = draws + [placed(draw, scale, origin, to) for draw in lens]
    return device


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("checkout", type=FsPath, help="a checkout of flutter_device_preview")
    parser.add_argument("--out", type=FsPath, default=DEFAULT_OUT)
    parser.add_argument("--commit", help="upstream commit, when the checkout is not a git repository")
    args = parser.parse_args()

    commit = args.commit or subprocess.run(
        ["git", "-C", str(args.checkout), "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()

    args.out.mkdir(parents=True, exist_ok=True)
    devices, skipped = {}, []
    for frame_file in sorted((args.checkout / DEVICES_DIR).glob("*/*/frame.g.dart")):
        try:
            device = read_device(frame_file.parent)
        except Unsupported as err:
            skipped.append((frame_file.parent.name, str(err)))
            continue
        devices[device.name] = device
    for spec in HAND_WRITTEN:
        try:
            device = read_hand_written(spec, args.checkout / DEVICES_DIR)
        except Unsupported as err:
            skipped.append((spec["folder"], str(err)))
            continue
        devices[device.name] = device
    for spec in OWN_DEVICES:
        try:
            devices[spec["name"]] = synthesise(spec, devices)
        except (Unsupported, KeyError) as err:
            skipped.append((spec["name"], str(err)))

    rows = []
    for device in devices.values():
        try:
            row, files = emit(device)
        except Unsupported as err:
            skipped.append((device.name, str(err)))
            continue
        for file, svg in files.items():
            (args.out / file).write_text(svg)
        rows.append(row)

    rows.sort(key=lambda r: r["name"])
    (args.out / "manifest.json").write_text(json.dumps(rows, indent=2) + "\n")
    licence = (args.checkout / "device_frame/LICENSE").read_text().strip()
    (args.out / "LICENSE-device_frame").write_text(
        f"The device frame SVGs in this folder are converted from the Flutter package\n"
        f"device_frame, https://{REPO} at commit\n{commit}, by native/scripts/device-frames/convert.py.\n\n"
        f"{licence}\n"
    )

    print(f"{len(rows)} devices written to {args.out}")
    for name, why in skipped:
        print(f"skipped {name}: {why}", file=sys.stderr)
    return 1 if skipped else 0


if __name__ == "__main__":
    sys.exit(main())
