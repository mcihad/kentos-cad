"""Puts a usage scenario's desktop and web pictures side by side.

    python3 scripts/usage/compare.py <iz> [--dizin .run/shots/kullanim]

`kentos-cad kullan <iz>` writes `masaustu-<iz>-<nn>-<ad>-<G>x<Y>[-acik].png`,
`pnpm -C apps/web e2e:use <iz>` writes the same names with `web-`. Each pair
becomes `karsilastir-<iz>-<nn>-<ad>-<G>x<Y>[-acik].png`: the desktop on the
left, the web on the right, each under its name. A picture one platform
lacks is said and left out of the pair (the other stands alone).
"""
import argparse
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

BAND = 40
GAP = 16
INK = (230, 232, 236)
PAPER = (24, 26, 31)


def font(size):
    for name in ("/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", "/usr/share/fonts/noto/NotoSans-Regular.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def pair(left, right, out):
    shown = [(name, Image.open(path).convert("RGB")) for name, path in (("Masaüstü", left), ("Web", right)) if path]
    width = sum(img.width for _, img in shown) + GAP * (len(shown) - 1)
    height = BAND + max(img.height for _, img in shown)
    sheet = Image.new("RGB", (width, height), PAPER)
    draw = ImageDraw.Draw(sheet)
    x = 0
    for name, img in shown:
        draw.text((x + 12, 8), name, fill=INK, font=font(22))
        sheet.paste(img, (x, BAND))
        x += img.width + GAP
    sheet.save(out)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("iz")
    parser.add_argument("--dizin", default=".run/shots/kullanim")
    args = parser.parse_args(argv)
    folder = Path(args.dizin)
    pattern = re.compile(rf"^(masaustu|web)-{re.escape(args.iz)}-(\d+-.+)\.png$")
    found = {}
    for path in sorted(folder.glob(f"*-{args.iz}-*.png")):
        m = pattern.match(path.name)
        if m:
            found.setdefault(m.group(2), {})[m.group(1)] = path
    if not found:
        sys.exit(f"{folder}: {args.iz} için resim yok; önce kentos-cad kullan {args.iz} ve pnpm -C apps/web e2e:use {args.iz}")
    for key, sides in sorted(found.items()):
        for side in ("masaustu", "web"):
            if side not in sides:
                print(f"Uyarı: {key}: {side} resmi yok", file=sys.stderr)
        out = folder / f"karsilastir-{args.iz}-{key}.png"
        pair(sides.get("masaustu"), sides.get("web"), out)
        print(out)


if __name__ == "__main__":
    main()
