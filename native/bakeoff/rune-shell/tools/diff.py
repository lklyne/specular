"""Pixel difference between two PNGs of one size, outside the rows a
toolbar covers and, with --mask, outside given rectangles.

    python3 tools/diff.py A.png B.png [--skip-top N] [--mask x,y,w,h ...] [--out diff.png]

Prints how many pixels differ at all, by more than 2 and by more than 16 of
255 on any channel, the largest difference, and the box holding every pixel
over 16. --out writes the differing pixels in red over a faded A.
"""

import argparse

from PIL import Image, ImageChops


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("a")
    parser.add_argument("b")
    parser.add_argument("--skip-top", type=int, default=0)
    parser.add_argument("--mask", action="append", default=[])
    parser.add_argument("--out")
    args = parser.parse_args()

    a = Image.open(args.a).convert("RGBA")
    b = Image.open(args.b).convert("RGBA")
    if a.size != b.size:
        raise SystemExit(f"sizes differ: {a.size} and {b.size}")
    width, height = a.size
    # The largest difference of any channel, per pixel.
    bands = ImageChops.difference(a, b).split()
    worst = bands[0]
    for band in bands[1:]:
        worst = ImageChops.lighter(worst, band)
    pixels = worst.load()
    masks = [tuple(int(v) for v in m.split(",")) for m in args.mask]

    def masked(x, y):
        return y < args.skip_top or any(
            mx <= x < mx + mw and my <= y < my + mh for mx, my, mw, mh in masks
        )

    compared = any_diff = over2 = over16 = largest = 0
    box = None
    marks = Image.new("RGBA", a.size, (0, 0, 0, 0)) if args.out else None
    for y in range(height):
        for x in range(width):
            if masked(x, y):
                continue
            compared += 1
            d = pixels[x, y]
            if d == 0:
                continue
            any_diff += 1
            largest = max(largest, d)
            if d > 2:
                over2 += 1
            if d > 16:
                over16 += 1
                box = (
                    (x, y, x, y)
                    if box is None
                    else (min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y))
                )
                if marks:
                    marks.putpixel((x, y), (255, 0, 0, 255))

    def share(count):
        return f"{count} ({100.0 * count / compared:.4f}%)"

    print(f"compared {compared} of {width * height} pixels")
    print(f"differ at all: {share(any_diff)}")
    print(f"differ by > 2: {share(over2)}")
    print(f"differ by > 16: {share(over16)}")
    print(f"largest difference: {largest}")
    print(f"box of > 16: {box}")
    if marks:
        faded = Image.blend(a, Image.new("RGBA", a.size, (255, 255, 255, 255)), 0.6)
        Image.alpha_composite(faded, marks).save(args.out)


if __name__ == "__main__":
    main()
