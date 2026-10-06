#!/usr/bin/env python3
"""Resim nesnesi (docs/adr/0192 §1, §5): the shared cases of the picture
tools' computations, written from the ADR's rules without KentOS code.

- Resim ekle's frame: its lower left corner p and a second point q give
  the width |q − p| and the turn atan2 of q − p; the height is the width
  times the picture's height over its width. The same point twice gives
  no frame.
- Resmi kırp's clip: a boundary drawn in the world, its corners in the
  picture's own fractions (0,0 its lower left, 1,1 its upper right; a
  mirrored picture is upside down in its frame, so the picture's own
  fraction up is one less the frame's), cut to the unit square, turned
  counter-clockwise, starting at its lowest corner (the leftmost of the
  lowest). A boundary that leaves nothing of the picture (no area) is
  refused.

An unturned frame's fractions are exact fractions here, and the cut with
them; a turned frame's are within 1e-12. The core runs the cases natively
and through WASM (crates/shared/geometry-core/tests/all/image.rs,
apps/web/src/tools/image.wasm.test.ts).

    python3 scripts/fixtures/image_cases.py          # write
    python3 scripts/fixtures/image_cases.py --check  # compare
"""

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "image" / "v1" / "cases.json"
SOURCE = "scripts/fixtures/image_cases.py (docs/adr/0192)"

NO_SIZE = "İkinci nokta birinciyle aynı yerde; resmin genişliği için başka bir noktaya tıklayın."
CLIP_OUTSIDE = "Kırpma sınırı resmin dışında kalıyor; resmin üzerinde bir sınır çizin."


def placed(p, q, aspect):
    dx, dy = q[0] - p[0], q[1] - p[1]
    width = math.hypot(dx, dy)
    if not (width > 0):
        return None
    return {"width": width, "height": width * aspect, "rotation": math.atan2(dy, dx)}


def fractions(frame, q):
    """A world point's fractions along and up an unturned frame, exactly; the picture's own up when mirrored."""
    (px, py), w, h, mirror = frame
    s = (F(q[0]) - F(px)) / F(w)
    t = (F(q[1]) - F(py)) / F(h)
    return (s, 1 - t if mirror else t)


def turned_fractions(frame, rotation, q):
    """The same for a turned frame, in floats (the cases compare within 1e-12)."""
    (px, py), w, h, mirror = frame
    c, s = math.cos(rotation), math.sin(rotation)
    dx, dy = q[0] - px, q[1] - py
    a = (dx * c + dy * s) / w
    b = (-dx * s + dy * c) / h
    return (a, 1 - b if mirror else b)


def twice_area(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1]))


def cut(ring):
    """The ring cut to the unit square, side by side (each side a half-plane the square lies in), counter-clockwise,
    from its lowest corner; None when nothing with an area is left."""
    sides = [
        (lambda p: p[0], lambda a, b: (F(0), a[1] + (b[1] - a[1]) * (0 - a[0]) / (b[0] - a[0]))),
        (lambda p: 1 - p[0], lambda a, b: (F(1), a[1] + (b[1] - a[1]) * (1 - a[0]) / (b[0] - a[0]))),
        (lambda p: p[1], lambda a, b: (a[0] + (b[0] - a[0]) * (0 - a[1]) / (b[1] - a[1]), F(0))),
        (lambda p: 1 - p[1], lambda a, b: (a[0] + (b[0] - a[0]) * (1 - a[1]) / (b[1] - a[1]), F(1))),
    ]
    out = list(ring)
    for inside, meet in sides:
        given, out = out, []
        for i, a in enumerate(given):
            b = given[(i + 1) % len(given)]
            if inside(a) >= 0:
                out.append(a)
            if (inside(a) >= 0) != (inside(b) >= 0):
                out.append(meet(a, b))
    # A corner met twice is one corner.
    kept = []
    for p in out:
        if not kept or kept[-1] != p:
            kept.append(p)
    if len(kept) > 1 and kept[0] == kept[-1]:
        kept.pop()
    if len(kept) < 3 or twice_area(kept) == 0:
        return None
    if twice_area(kept) < 0:
        kept.reverse()
    first = min(range(len(kept)), key=lambda i: (kept[i][1], kept[i][0]))
    return kept[first:] + kept[:first]


def pt(x, y):
    return {"x": float(x), "y": float(y)}


def picture(p, w, h, rotation=0.0, mirror=False):
    out = {"kind": "image", "p": pt(*p), "width": float(w), "height": float(h), "rotation": float(rotation), "asset": "resim-0011223344556677"}
    if mirror:
        out["mirror"] = True
    return out


def clip_case(name, frame, world, rotation=0.0):
    p, w, h, mirror = frame
    if rotation == 0.0:
        ring = [fractions(frame, q) for q in world]
        got = cut(ring)
        clip = None if got is None else [pt(x, y) for x, y in got]
    else:
        ring = [tuple(F(v) for v in turned_fractions(frame, rotation, q)) for q in world]
        got = cut(ring)
        clip = None if got is None else [pt(x, y) for x, y in got]
    case = {"name": name, "shape": picture(p, w, h, rotation, mirror), "world": [pt(*q) for q in world]}
    if clip is None:
        case["problem"] = CLIP_OUTSIDE
    else:
        case["clip"] = clip
    if rotation != 0.0:
        case["tolerance"] = 1e-12
    return case


def build():
    placements = []
    for name, p, q, aspect in [
        ("doğuya: genişlik uzaklık, dönüşsüz", (10.0, 20.0), (26.0, 20.0), 0.75),
        ("3-4-5 doğrultusunda", (0.0, 0.0), (30.0, 40.0), 0.75),
        ("kuzeye: çeyrek tur", (0.0, 0.0), (0.0, 30.0), 0.5),
        ("batıya: yarım tur, uzun resim", (5.0, 5.0), (-7.0, 5.0), 2.0),
        ("güneybatıya: eksi açı", (100.0, 100.0), (97.0, 96.0), 1.0),
    ]:
        got = placed(p, q, aspect)
        placements.append({"name": name, "p": pt(*p), "q": pt(*q), "aspect": aspect, "placed": got})
    placements.append({"name": "aynı nokta: çerçeve yok", "p": pt(3, 4), "q": pt(3, 4), "aspect": 1.0, "problem": NO_SIZE})
    frame = ((100.0, 200.0), 40.0, 20.0, False)
    mirrored = ((100.0, 200.0), 40.0, 20.0, True)
    clips = [
        clip_case("içte dikdörtgen", frame, [(110, 205), (130, 205), (130, 215), (110, 215)]),
        clip_case("saat yönünde verilen sınır ters döner", frame, [(110, 205), (110, 215), (130, 215), (130, 205)]),
        clip_case("taşan sınır resmin kenarlarında kesilir", frame, [(90, 190), (120, 190), (120, 230), (90, 230)]),
        clip_case("resmi saran sınır bütün resimdir", frame, [(0, 0), (500, 0), (500, 500), (0, 500)]),
        clip_case("köşeyi kesen üçgen", frame, [(130, 195), (150, 195), (150, 225)]),
        clip_case("en alttaki köşeden: eşit alttakilerin en solu", frame, [(130, 205), (110, 215), (110, 205)]),
        clip_case("aynalı resim: kesir yukarı ters", mirrored, [(100, 200), (140, 200), (140, 205), (100, 205)]),
        clip_case("dışarıda: söylenir", frame, [(300, 300), (310, 300), (310, 310)]),
        clip_case("yalnız kenara değen: alansız, söylenir", frame, [(140, 205), (150, 205), (150, 210), (140, 210)]),
        clip_case("dönük resim", ((100.0, 200.0), 40.0, 20.0, False), [(95, 205), (105, 205), (105, 230), (95, 230)], rotation=math.pi / 2),
    ]
    return {"format": "kentos.image-cases", "version": 1, "source": SOURCE, "placements": placements, "clips": clips}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"resim durumları tutarlı: {OUT.relative_to(ROOT)}")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
