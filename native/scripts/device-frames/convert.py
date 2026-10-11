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
DEFAULT_OUT = FsPath(__file__).resolve().parents[2] / "crates/specular-app/assets/device-frames"


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
    def __init__(self, frame, screen):
        self.frame = frame
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
        lines = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
            "<!-- Generated by native/scripts/device-frames/convert.py from the MIT-licensed"
            " Flutter device_frame package. Do not edit by hand. -->",
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


def convert(folder):
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
    screen_subs = subpaths(screen.cmds)
    if len(screen_subs) > 1 and screen.fillType != "evenOdd":
        raise Unsupported("multi-part screen path that is not even-odd")
    boxes = [bounds(sub) for sub in screen_subs]
    outer = max(range(len(boxes)), key=lambda i: (boxes[i][2] - boxes[i][0]) * (boxes[i][3] - boxes[i][1]))
    box = boxes[outer]

    canvas = Canvas()
    frame_src = (folder / "frame.g.dart").read_text()
    evaluate(block_after(frame_src, "void paint(", "{", "}"), {"canvas": canvas, "size": frame})

    svg = Svg(frame, screen)
    for draw in canvas.draws:
        svg.draw(draw)

    name = folder.name.lower().replace("_", "-")
    row = {
        "name": name,
        "file": f"{name}.svg",
        "frame": {"width": tidy(frame.width), "height": tidy(frame.height)},
        "screen": {
            "x": tidy(box[0], 3),
            "y": tidy(box[1], 3),
            "width": tidy(box[2] - box[0], 3),
            "height": tidy(box[3] - box[1], 3),
        },
        "screenCornerRadius": tidy(corner_radius(screen_subs[outer], box), 2),
        "logicalScreen": {"width": tidy(logical.width), "height": tidy(logical.height)},
        "pixelRatio": tidy(ratio),
        "platform": platform,
        "kind": kind,
    }
    return row, svg.text()


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
    rows, skipped = [], []
    for frame_file in sorted((args.checkout / DEVICES_DIR).glob("*/*/frame.g.dart")):
        try:
            row, svg = convert(frame_file.parent)
        except Unsupported as err:
            skipped.append((frame_file.parent.name, str(err)))
            continue
        (args.out / row["file"]).write_text(svg)
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
