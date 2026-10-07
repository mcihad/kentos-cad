"""The shared cases of the drawing extras (docs/adr/0197): the common tangents
of two circles or arcs and the one the clicks choose, a parallelogram's fourth
corner, range rings and their rays.

    python3 scripts/fixtures/drawing_extras_cases.py           # writes the file
    python3 scripts/fixtures/drawing_extras_cases.py --check   # writes nothing; compares

Writes fixtures/drawing-extras/v1/cases.json. The rules are written here from
the ADR on their own, not from an implementation's output; the geometry core
(crates/shared/geometry-core, `tools::drawing_extras`, natively and through
WASM: `commonTangents`, `chosenTangent`, `fourthCorner`, `rangeRings`) is held
to them within 1e-9.

- Two circles: centres C1, C2, radii r1, r2, d = |C2 − C1|, u = (C2 − C1)/d,
  n = (−u.y, u.x). Outer tangents when d > |r1 − r2|: c = (r1 − r2)/d, for
  k = +1 then −1, m = c·u + k·√(1 − c²)·n, A = C1 + r1·m, B = C2 + r2·m. Inner
  tangents when d > r1 + r2: c = (r1 + r2)/d, m alike, A = C1 + r1·m,
  B = C2 − r2·m. Order: outer left, outer right, inner left, inner right.
- An arc's circle gives the tangents; a solution whose point on an arc is off
  the arc (its angle from the arc's start, counter-clockwise, over the sweep
  and under a full turn, 1e-9 either way) is left out.
- The one chosen for clicks P1 (on the first) and P2 (on the second): the
  least |P1 − A| + |P2 − B|, the first in order on a tie.
- Fourth corner: D = A + C − B.
- Range rings: radii k·s for k = 1 … n; ray j ends at M + n·s·(sin θ, cos θ),
  θ = 2π·j/m, j = 0 … m − 1.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/drawing-extras/v1/cases.json"
KINDS = ["outerLeft", "outerRight", "innerLeft", "innerRight"]


def P(x, y):
    return {"x": x, "y": y}


def on_arc(c, a, arc):
    if arc is None:
        return True
    a0, a1 = arc
    sweep = (a1 - a0) % (2 * math.pi)
    if sweep < 1e-12:
        sweep = 2 * math.pi
    f = math.atan2(a[1] - c[1], a[0] - c[0])
    off = (f - a0) % (2 * math.pi)
    return off <= sweep + 1e-9 or off >= 2 * math.pi - 1e-9


def tangents(c1, r1, c2, r2, arc1=None, arc2=None):
    dx, dy = c2[0] - c1[0], c2[1] - c1[1]
    d = math.hypot(dx, dy)
    out = []
    if d == 0:
        return out
    u = (dx / d, dy / d)
    n = (-u[1], u[0])
    for kind, inner in (("outer", False), ("inner", True)):
        if (not inner and not d > abs(r1 - r2)) or (inner and not d > r1 + r2):
            continue
        c = ((r1 + r2) if inner else (r1 - r2)) / d
        s = math.sqrt(1 - c * c)
        for k, side in ((1, "Left"), (-1, "Right")):
            m = (c * u[0] + k * s * n[0], c * u[1] + k * s * n[1])
            a = (c1[0] + r1 * m[0], c1[1] + r1 * m[1])
            sign = -1 if inner else 1
            b = (c2[0] + sign * r2 * m[0], c2[1] + sign * r2 * m[1])
            if on_arc(c1, a, arc1) and on_arc(c2, b, arc2):
                out.append({"kind": kind + side, "a": P(*a), "b": P(*b)})
    return out


def chosen(sols, p1, p2):
    best = None
    for s in sols:
        cost = math.hypot(p1[0] - s["a"]["x"], p1[1] - s["a"]["y"]) + math.hypot(p2[0] - s["b"]["x"], p2[1] - s["b"]["y"])
        if best is None or cost < best[0]:
            best = (cost, s)
    return best[1] if best else None


def rings(m, s, n, rays):
    radii = [k * s for k in range(1, n + 1)]
    ends = []
    for j in range(rays):
        t = 2 * math.pi * j / rays
        ends.append(P(m[0] + n * s * math.sin(t), m[1] + n * s * math.cos(t)))
    return {"radii": radii, "rays": ends}


def circle(c, r):
    return {"kind": "circle", "c": P(*c), "r": r}


def arc(c, r, a0, a1):
    return {"kind": "arc", "c": P(*c), "r": r, "a0": a0, "a1": a1}


TANGENTS = [
    ("eşit-uzak", circle((0, 0), 5), circle((30, 0), 5)),
    ("farklı-uzak", circle((0, 0), 8), circle((25, 10), 3)),
    ("kesişen", circle((0, 0), 6), circle((8, 0), 5)),
    ("değen-dıştan", circle((0, 0), 4), circle((10, 0), 6)),
    ("iç-içe", circle((0, 0), 10), circle((2, 1), 3)),
    ("uzak-koordinat", circle((487012.5, 4420003.25), 2.5), circle((487040.75, 4419996.5), 4)),
    ("üst-yay", arc((0, 0), 5, math.radians(30), math.radians(150)), circle((30, 0), 5)),
    ("iki-yay", arc((0, 0), 5, math.radians(60), math.radians(120)), arc((30, 0), 5, math.radians(240), math.radians(300))),
]

CHOICES = [
    ("üstten-üste", 0, (0, 5.5), (30, 5.5)),
    ("alttan-alta", 0, (0, -5.5), (30, -5.5)),
    ("üstten-alta", 0, (-1, 4.8), (31, -4.8)),
    ("alttan-üste", 0, (-1, -4.8), (31, 4.8)),
    ("farklı-sağ", 1, (5, -6), (27, 8)),
]

FOURTH = [
    ("dikdörtgen", P(0, 0), P(10, 0), P(10, 5)),
    ("eğik", P(2, 1), P(9, 3), P(12, 11)),
    ("uzak", P(487000.125, 4420000.5), P(487020.25, 4420003.75), P(487018.5, 4420021.125)),
]

RINGS = [
    ("beş-halka", (0, 0), 10, 5, 0),
    ("ışınlı", (487000, 4420000), 25, 4, 8),
    ("tek-halka-üç-ışın", (5, -3), 7.5, 1, 3),
]


def centre(e):
    return (e["c"]["x"], e["c"]["y"])


def span(e):
    return (e["a0"], e["a1"]) if e["kind"] == "arc" else None


def build():
    tangent_cases = [
        {"name": n, "first": a, "second": b, "want": tangents(centre(a), a["r"], centre(b), b["r"], span(a), span(b))}
        for n, a, b in TANGENTS
    ]
    choice_cases = []
    for n, i, p1, p2 in CHOICES:
        _, a, b = TANGENTS[i]
        sols = tangents(centre(a), a["r"], centre(b), b["r"], span(a), span(b))
        choice_cases.append({"name": n, "first": a, "second": b, "p1": P(*p1), "p2": P(*p2), "want": chosen(sols, p1, p2)})
    fourth_cases = [{"name": n, "a": a, "b": b, "c": c, "want": P(a["x"] + c["x"] - b["x"], a["y"] + c["y"] - b["y"])} for n, a, b, c in FOURTH]
    ring_cases = [{"name": n, "center": P(*m), "spacing": s, "count": k, "rays": r, "want": rings(m, s, k, r)} for n, m, s, k, r in RINGS]
    return {
        "format": "kentos.drawing-extras-cases",
        "version": 1,
        "generatedBy": "scripts/fixtures/drawing_extras_cases.py",
        "title": "Çizim ekleri: iki daireye teğet doğru, dördüncü köşe, menzil halkaları (ADR 0197)",
        "note": "Teğetler sırayla dış sol, dış sağ, iç sol, iç sağ; olmayan yazılmaz; yayda teğet noktası yayın dışındaysa çözüm yoktur. Seçilen: |P1 − A| + |P2 − B| en küçük olan. D = A + C − B. Halkalar k·s; ışınlar kuzeyden saat yönünde.",
        "tangents": tangent_cases,
        "choices": choice_cases,
        "fourth": fourth_cases,
        "rings": ring_cases,
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=2) + "\n"


def main():
    want = text_of(build())
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != want:
            print(f"{OUT.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
            return 1
        print("drawing extras cases match")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(want, encoding="utf-8")
    print(f"written: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
