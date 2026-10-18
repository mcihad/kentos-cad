#!/usr/bin/env python3
"""View marks drawn (docs/adr/0345 §1), from the ADR's rules, without KentOS code.

Writes fixtures/arch/v1/viewmarks.json; `--check` compares and writes nothing. The words' widths are the drawing
typeface's measured advances (the first face's, `paragraph_cases.py` reads them from the data); the core's test compares
piece by piece, in order, within 1e-9.

- Its words `h` = size (2.5 mm unless said) × scale / 1000 high. A bubble's radius `R` = max(1.5 h, half the wider of
  its two words + 0.5 h); its circle, a line across its middle, its top words' middle 0.75 h above the middle, its
  bottom words' 0.75 h below (empty words drawn not).
- A look arrow from a bubble towards `u`: a triangle, its tip 1.7 R from the middle along `u`, then the bubble's points
  at +60° and −60° from `u`.
- Section: its cut line part by part; then, at its first end and then at its last, the end part (as long as the part,
  at most 3 h, from the end inward) drawn again 0.15 h to the look side, a line from the end to the bubble 1.2 R past
  it along the end part, and the bubble with its arrow to the look side: the left of the first part's direction (the
  right when flipped).
- Elevation: a bubble at its point, its arrow at `angle` (degrees from east, counter-clockwise; 90 unless said).
- Detail: its circle (dashed) round its point, a leader from the circle to the bubble when they do not touch, then a
  bubble without an arrow at `at` (else at twice its radius up and to the right: c + 2r (√½, √½)).
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import paragraph_cases as pc  # noqa: E402  (the typefaces' measured advances)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/viewmarks.json"
FONT = pc.FONTS[0]


def width(t, h):
    return sum(pc.advance(FONT, c, False) for c in t) / 1000 * h if t else 0.0


def unit(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l = math.hypot(dx, dy)
    return (dx / l, dy / l) if l > 0 else None


def draw(form, pts, top, bottom, scale, size=None, radius=None, at=None, angle=None, flip=False):
    h = (size if size is not None else 2.5) * scale / 1000
    R = max(1.5 * h, max(width(top, h), width(bottom, h)) / 2 + 0.5 * h)
    out = []

    def line(a, b):
        out.append({"kind": "line", "a": list(a), "b": list(b)})

    def bubble(c, u):
        out.append({"kind": "circle", "c": list(c), "r": R})
        line((c[0] - R, c[1]), (c[0] + R, c[1]))
        if top:
            out.append({"kind": "text", "p": [c[0], c[1] + 0.75 * h], "text": top, "height": h})
        if bottom:
            out.append({"kind": "text", "p": [c[0], c[1] - 0.75 * h], "text": bottom, "height": h})
        if u is not None:
            a = math.atan2(u[1], u[0])
            on = lambda t: [c[0] + R * math.cos(t), c[1] + R * math.sin(t)]  # noqa: E731
            out.append({"kind": "polygon", "pts": [[c[0] + 1.7 * R * u[0], c[1] + 1.7 * R * u[1]],
                                                   on(a + math.radians(60)), on(a - math.radians(60))]})

    if form == "section":
        for k in range(len(pts) - 1):
            line(pts[k], pts[k + 1])
        n = len(pts)
        along = unit(pts[0], pts[1])
        side = -1.0 if flip else 1.0
        look = (-along[1] * side, along[0] * side)
        for end, inner in ((pts[0], pts[1]), (pts[n - 1], pts[n - 2])):
            d = unit(inner, end)
            ln = min(math.hypot(end[0] - inner[0], end[1] - inner[1]), 3 * h)
            start = (end[0] - d[0] * ln, end[1] - d[1] * ln)
            off = 0.15 * h
            line((start[0] + look[0] * off, start[1] + look[1] * off), (end[0] + look[0] * off, end[1] + look[1] * off))
            c = (end[0] + d[0] * 1.2 * R, end[1] + d[1] * 1.2 * R)
            line(end, (c[0] - d[0] * R, c[1] - d[1] * R))
            bubble(c, look)
    elif form == "elevation":
        a = math.radians(angle if angle is not None else 90.0)
        bubble(pts[0], (math.cos(a), math.sin(a)))
    else:
        c = pts[0]
        b = at if at is not None else (c[0] + 2 * radius * math.sqrt(0.5), c[1] + 2 * radius * math.sqrt(0.5))
        out.append({"kind": "circle", "c": list(c), "r": radius, "dashed": True})
        u = unit(c, b)
        if u is not None and math.hypot(b[0] - c[0], b[1] - c[1]) > radius + R:
            line((c[0] + u[0] * radius, c[1] + u[1] * radius), (b[0] - u[0] * R, b[1] - u[1] * R))
        bubble(b, None)
    return out


CASES = [
    ("düz kesit, 1:50, A ve A-101", "section", [(0.0, 0.0), (10.0, 0.0)], "A", "A-101", 50, {}),
    ("kesit çevrilmiş: bakış sağa", "section", [(0.0, 0.0), (10.0, 0.0)], "A", "A-101", 50, {"flip": True}),
    ("kırık kesme çizgisi, 1:100", "section", [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (9.0, 3.0)], "B", "A-102", 100, {}),
    ("eğik kesit, uzun pafta adı, 3.5 mm", "section", [(2.0, 1.0), (8.0, 5.0)], "C", "Kesitler 1", 50, {"size": 3.5}),
    ("kısa kesit: uç parçası kendi boyu", "section", [(0.0, 0.0), (0.05, 0.0)], "1", "—", 50, {}),
    ("görünüş kuzeye", "elevation", [(3.0, -1.0)], "2", "A-201", 50, {}),
    ("görünüş doğuya, 1:200", "elevation", [(0.0, 0.0)], "1", "Kesit A-A", 200, {"angle": 0.0}),
    ("görünüş güneybatıya, numarasız", "elevation", [(5.0, 5.0)], "", "—", 50, {"angle": 225.0}),
    ("detay, balon sağ üstte", "detail", [(6.0, 1.0)], "2", "A-101", 50, {"radius": 0.8}),
    ("detay, balon verilen yerde", "detail", [(0.0, 0.0)], "D1", "Detay 1", 50, {"radius": 0.5, "at": (-3.0, 2.0)}),
    ("detay, balon çemberin üstünde: kol yok", "detail", [(0.0, 0.0)], "3", "—", 50, {"radius": 2.0, "at": (0.5, 0.5)}),
    ("detay, yazısız", "detail", [(1.0, 1.0)], "", "", 100, {"radius": 1.0}),
]


def build():
    out = []
    for name, form, pts, top, bottom, scale, kw in CASES:
        spec = {"kind": "viewmark", "form": form, "points": [{"x": x, "y": y} for x, y in pts]}
        for k in ("radius", "angle", "size"):
            if k in kw:
                spec[k] = kw[k]
        if "at" in kw:
            spec["at"] = {"x": kw["at"][0], "y": kw["at"][1]}
        if kw.get("flip"):
            spec["flip"] = True
        if top or bottom:
            spec["words"] = {"top": top, "bottom": bottom}
        out.append({"name": name, "shape": spec, "scale": scale, "pieces": draw(form, pts, top, bottom, scale, **kw)})
    return {"format": "kentos.viewmark-cases", "version": 1,
            "note": "ADR 0345 §1. scripts/fixtures/viewmark_cases.py'nin ADR'den yazılmış kurallarından, KentOS kodu olmadan.",
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
        print(f"görünüm işaretleri: {len(CASES)} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
