#!/usr/bin/env python3
"""Tags drawn (docs/adr/0344 §1), from the ADR's rules, without KentOS code.

Writes fixtures/arch/v1/tags.json; `--check` compares and writes nothing. The words' widths are the drawing typeface's
measured advances (the first face's, `paragraph_cases.py` reads them from the data); the core's test compares within
1e-9.

- Its words `h` = size (2.5 mm unless said) × scale / 1000 high; lines `1.5 h` apart; each line centred on the tag's
  middle in x; the block of n lines `h (1 + 1.5 (n − 1))` high, its middle on the tag's; an empty line drawn not.
- Its box: half width the widest line's (its advances / 1000 × h; an empty line 0; never under h) / 2; the pad 0.4 h.
- Its frame: a circle of radius hypot(half width, half height) + pad; a rectangle ±(w/2 + pad), ±(b + pad); a hexagon
  that rectangle with a point each side as deep as (b + pad) tan 30°; a rounded rectangle, its corners quarter circles
  of half the lesser half-size (8 points, bulge tan 22.5° on the corners); none.
- Its leader: from where the line to the point it ties to leaves the frame (a circle its radius; else the nearest
  crossing with the frame's sides, a rounded frame's and none's its rectangle's) to that point; none when the point is
  inside.
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import paragraph_cases as pc  # noqa: E402  (the typefaces' measured advances)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/tags.json"
PITCH, PAD = 1.5, 0.4
TAN30 = math.tan(math.pi / 6)
QUARTER = math.tan(math.pi / 8)
FONT = pc.FONTS[0]


def width(line, h):
    if not line:
        return 0.0
    return sum(pc.advance(FONT, c, False) for c in line) / 1000 * h


def draw(at, text, shape, size, scale, leader, anchor):
    h = (size if size is not None else 2.5) * scale / 1000
    lines = text.split("\n")
    w = max([h] + [width(ln, h) for ln in lines])
    b = h * (1 + PITCH * (len(lines) - 1)) / 2
    pad = PAD * h
    a, bb = w / 2 + pad, b + pad
    x, y = at
    frame = None
    ring = None
    if shape == "circle":
        frame = {"kind": "circle", "c": [x, y], "r": math.hypot(w / 2, b) + pad}
    elif shape == "rectangle":
        ring = [(-a, -bb), (a, -bb), (a, bb), (-a, bb)]
        frame = {"kind": "polygon", "pts": [[x + p, y + q] for p, q in ring], "bulges": None}
    elif shape == "hexagon":
        k = bb * TAN30
        ring = [(-a, -bb), (a, -bb), (a + k, 0), (a, bb), (-a, bb), (-a - k, 0)]
        frame = {"kind": "polygon", "pts": [[x + p, y + q] for p, q in ring], "bulges": None}
    elif shape == "rounded":
        r = min(a, bb) / 2
        pts = [(-a + r, -bb), (a - r, -bb), (a, -bb + r), (a, bb - r), (a - r, bb), (-a + r, bb), (-a, bb - r), (-a, -bb + r)]
        frame = {"kind": "polygon", "pts": [[x + p, y + q] for p, q in pts], "bulges": [0.0, QUARTER] * 4}
    texts = [{"p": [x, y + b - h / 2 - i * PITCH * h], "text": ln, "height": h} for i, ln in enumerate(lines) if ln]
    lead = None
    if leader and anchor is not None:
        dx, dy = anchor[0] - x, anchor[1] - y
        l = math.hypot(dx, dy)
        if l > 0:
            d = (dx / l, dy / l)
            if shape == "circle":
                t = frame["r"]
            else:
                side = ring if shape == "hexagon" else [(-a, -bb), (a, -bb), (a, bb), (-a, bb)]
                t = math.inf
                for i in range(len(side)):
                    p, q = side[i], side[(i + 1) % len(side)]
                    e = (q[0] - p[0], q[1] - p[1])
                    den = d[0] * e[1] - d[1] * e[0]
                    if den == 0:
                        continue
                    t1 = (p[0] * e[1] - p[1] * e[0]) / den
                    u = (p[0] * d[1] - p[1] * d[0]) / den
                    if t1 >= 0 and 0 <= u <= 1 and t1 < t:
                        t = t1
            if t < l:
                lead = {"a": [x + d[0] * t, y + d[1] * t], "b": list(anchor)}
    return {"frame": frame, "texts": texts, "leader": lead}


CASES = [
    ("kapı numarası dairede, 1:50", (2.0, 0.5), "K01", "circle", None, 50, False, (2.0, 0.0)),
    ("mahal etiketi dikdörtgende, üç satır, 1:100", (22.0, 1.5), "101\nSalon\n12.00 m²", "rectangle", None, 100, False, (22.0, 1.5)),
    ("pencere numarası altıgende, kol aşağı", (6.0, 0.5), "P01", "hexagon", None, 50, True, (6.0, 0.0)),
    ("boş yazı: çerçeve yazı yüksekliği genişliğinde", (0.0, 0.0), "", "circle", None, 50, False, None),
    ("yuvarlak köşeli, kol sağa", (40.0, 0.5), "S1", "rounded", None, 50, True, (42.0, 0.5)),
    ("çerçevesiz, kol eğik", (50.0, 3.5), "A", "none", None, 100, True, (50.0, 3.0)),
    ("altıgen, iki satır, 3.5 mm", (3.0, 7.0), "K01\nAhşap", "hexagon", 3.5, 50, True, (3.0, 5.0)),
    ("bağ noktası çerçevenin içindeyse kol yok", (0.0, 0.0), "D1", "circle", None, 100, True, (0.05, 0.05)),
    ("dikdörtgen, kol köşeden", (10.0, 10.0), "101", "rectangle", 2.0, 200, True, (13.0, 6.0)),
    ("yuvarlak köşeli, iki satır, boş ikinci satır yazılmaz", (5.0, 5.0), "M1\n", "rounded", None, 50, False, None),
]


def build():
    out = []
    for name, at, text, shape, size, scale, leader, anchor in CASES:
        spec = {"kind": "tag", "at": {"x": at[0], "y": at[1]}}
        if text:
            spec["text"] = text
        if shape != "circle":
            spec["shape"] = shape
        if size is not None:
            spec["size"] = size
        if leader:
            spec["leader"] = True
        if anchor is not None:
            spec["anchor"] = {"x": anchor[0], "y": anchor[1]}
        out.append({"name": name, "shape": spec, "scale": scale, **draw(at, text, shape, size, scale, leader, anchor)})
    return {"format": "kentos.tag-cases", "version": 1,
            "note": "ADR 0344 §1. scripts/fixtures/tag_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan.",
            "cases": out}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"etiketler: {len(CASES)} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
