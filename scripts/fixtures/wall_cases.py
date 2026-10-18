#!/usr/bin/env python3
"""A wall's drawing (docs/adr/0339 §1), from the ADR's rules, without KentOS code.

Writes fixtures/walls/v1/cases.json; `--check` compares and writes nothing. The shared core's test
(`kentos-geometry-core`, tests/all/walls.rs) reads it and must give the same lines, areas and pieces.

The rules, as ADR 0339 §1 gives them:
- the justification line is the path moved `offset` to its left; the left face lies 0 (left), width/2 (center) or
  width (right) further left of it, the right face a width to the right of the left face, the components' boundaries
  between, from the left face, their thicknesses the given ones and the rest for the one without;
- a line of the wall is the path offset `d` to its left, edge by edge: a straight edge moved along its left normal, an
  arc about its centre (the left of a counter-clockwise arc is its centre's side: radius r − d, else r + d; at or
  below 1e-9 the wall cannot be drawn); where two offset edges do not meet, the crossings of their supports (line and
  line, line and circle, circle and circle), the nearest to the path's vertex, joins them when it is at most 4·|d|
  from it, else a straight bevel goes between their ends; a closed path joins its last edge to its first;
- the body's area: an open wall's ring (its left face, then across, then its right face back), a closed wall's band
  (the left face's ring less the right face's, absolute), each ring's area its vertices' shoelace and each arc's
  segment, r²/2·(θ − sin θ) with θ its signed sweep;
- the pieces at a detail: schematic, the fill (when the wall has one) then the two faces and an open wall's two caps
  (unless its ends are open); standard and detailed, each component shown at that detail (its least detail at most the
  view's, never schematic) its hatch, then the faces, the caps, and each boundary between two shown components a line;
  a hatch's pattern is the library's at `scale × plot scale / 1000` metres per paper millimetre, turned with the path's
  first edge plus its angle; SOLID a solid fill.
"""

import argparse
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/walls/v1/cases.json"
MITER = 4.0
TOL = 1e-9


def arc_of(a, b, bulge):
    """The arc an edge with this bulge draws: centre, radius, start angle, signed sweep; None when straight."""
    if abs(bulge) < 1e-12:
        return None
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    sweep = 4.0 * math.atan(bulge)
    r = chord / (2.0 * math.sin(abs(sweep) / 2.0))
    # The centre lies on the chord's bisector, to the left for a counter-clockwise arc of less than half a turn.
    mx, my = (a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0
    h = r * math.cos(abs(sweep) / 2.0)
    nx, ny = -dy / chord, dx / chord
    side = 1.0 if (bulge > 0) == (abs(sweep) < math.pi) else -1.0
    c = (mx + nx * h * side, my + ny * h * side)
    a0 = math.atan2(a[1] - c[1], a[0] - c[0])
    return c, r, a0, sweep


def line_hits(p1, p2, q1, q2):
    d = (p2[0] - p1[0]) * (q2[1] - q1[1]) - (p2[1] - p1[1]) * (q2[0] - q1[0])
    if abs(d) < 1e-15:
        return []
    t = ((q1[0] - p1[0]) * (q2[1] - q1[1]) - (q1[1] - p1[1]) * (q2[0] - q1[0])) / d
    return [(p1[0] + t * (p2[0] - p1[0]), p1[1] + t * (p2[1] - p1[1]))]


def circle_line_hits(c, r, p, q):
    dx, dy = q[0] - p[0], q[1] - p[1]
    fx, fy = p[0] - c[0], p[1] - c[1]
    A = dx * dx + dy * dy
    B = 2 * (fx * dx + fy * dy)
    C = fx * fx + fy * fy - r * r
    disc = B * B - 4 * A * C
    if disc < 0 or A == 0:
        return []
    s = math.sqrt(disc)
    return [(p[0] + t * dx, p[1] + t * dy) for t in ((-B - s) / (2 * A), (-B + s) / (2 * A))]


def circle_hits(c1, r1, c2, r2):
    dx, dy = c2[0] - c1[0], c2[1] - c1[1]
    d = math.hypot(dx, dy)
    if d < 1e-15 or d > r1 + r2 or d < abs(r1 - r2):
        return []
    a = (r1 * r1 - r2 * r2 + d * d) / (2 * d)
    h = math.sqrt(max(r1 * r1 - a * a, 0.0))
    mx, my = c1[0] + a * dx / d, c1[1] + a * dy / d
    return [(mx + h * dy / d, my - h * dx / d), (mx - h * dy / d, my + h * dx / d)]


def offset(pts, bulges, closed, d):
    """The path offset `d` to its left: (points, bulges one per vertex), or the reason it cannot be drawn."""
    n = len(pts)
    edges = n if closed else n - 1
    pieces, corners = [], []
    for i in range(edges):
        a, b = pts[i], pts[(i + 1) % n]
        bulge = bulges[i] if bulges else 0.0
        arc = arc_of(a, b, bulge)
        if arc:
            c, r, a0, sweep = arc
            ccw = sweep > 0
            r2 = r - d if ccw else r + d
            if r2 <= 1e-9:
                return None, "collapse"
            k = r2 / r
            sc = lambda p: (c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k)  # noqa: E731
            pieces.append({"a": sc(a), "b": sc(b), "c": c, "r": r2, "ccw": ccw})
        else:
            dx, dy = b[0] - a[0], b[1] - a[1]
            length = math.hypot(dx, dy)
            if length < 1e-12:
                continue
            nx, ny = -dy / length * d, dx / length * d
            pieces.append({"a": (a[0] + nx, a[1] + ny), "b": (b[0] + nx, b[1] + ny), "c": None})
        corners.append(b)
    m = len(pieces)
    bevel = [False] * m
    for k in range(m if closed else m - 1):
        p1, p2 = pieces[k], pieces[(k + 1) % m]
        if math.hypot(p1["b"][0] - p2["a"][0], p1["b"][1] - p2["a"][1]) < 1e-9:
            continue
        corner = corners[k]
        if p1["c"] is None and p2["c"] is None:
            hits = line_hits(p1["a"], p1["b"], p2["a"], p2["b"])
        elif p1["c"] is None:
            hits = circle_line_hits(p2["c"], p2["r"], p1["a"], p1["b"])
        elif p2["c"] is None:
            hits = circle_line_hits(p1["c"], p1["r"], p2["a"], p2["b"])
        else:
            hits = circle_hits(p1["c"], p1["r"], p2["c"], p2["r"])
        hits.sort(key=lambda h: math.hypot(h[0] - corner[0], h[1] - corner[1]))
        if hits and math.hypot(hits[0][0] - corner[0], hits[0][1] - corner[1]) <= MITER * abs(d):
            p1["b"] = hits[0]
            p2["a"] = hits[0]
        else:
            bevel[k] = True
    out_p, out_b = [pieces[0]["a"]], []
    for k, p in enumerate(pieces):
        if p["c"] is None:
            out_b.append(0.0)
        else:
            s = math.atan2(p["a"][1] - p["c"][1], p["a"][0] - p["c"][0])
            e = math.atan2(p["b"][1] - p["c"][1], p["b"][0] - p["c"][0])
            sweep = (e - s) % (2 * math.pi) if p["ccw"] else -((s - e) % (2 * math.pi))
            if abs(sweep) < 1e-12:
                sweep = 2 * math.pi if p["ccw"] else -2 * math.pi
            out_b.append(math.tan(sweep / 4.0))
        out_p.append(p["b"])
        if bevel[k] and (closed or k < m - 1):
            out_b.append(0.0)
            out_p.append(pieces[(k + 1) % m]["a"])
    if closed:
        out_p.pop()
    else:
        out_b.append(0.0)
    # Points no farther apart than 1e-9: one, keeping the later edge's bulge.
    cp, cb = [], []
    for p, b in zip(out_p, out_b):
        if cp and math.hypot(p[0] - cp[-1][0], p[1] - cp[-1][1]) <= 1e-9:
            cb[-1] = b
            continue
        cp.append(p)
        cb.append(b)
    if closed and len(cp) > 1 and math.hypot(cp[0][0] - cp[-1][0], cp[0][1] - cp[-1][1]) <= 1e-9:
        cp.pop()
        cb.pop()
    return (cp, cb), None


def thicknesses(width, comps):
    fixed = sum(c["thickness"] for c in comps if c.get("thickness") is not None)
    rest = [c for c in comps if c.get("thickness") is None]
    slack = 1e-9 * max(width, 1.0)
    if not rest and abs(fixed - width) <= slack:
        return [c["thickness"] for c in comps]
    if len(rest) == 1 and fixed < width - slack:
        return [c["thickness"] if c.get("thickness") is not None else width - fixed for c in comps]
    return None


def boundaries(w):
    left = w.get("offset", 0.0) + {"left": 0.0, "center": w["width"] / 2.0, "right": w["width"]}[w["justify"]]
    out = [left]
    comps = w.get("components", [])
    t = thicknesses(w["width"], comps) if comps else None
    if t:
        at = left
        for x in t[:-1]:
            at -= x
            out.append(at)
    out.append(left - w["width"])
    return out


def lines(w):
    out = []
    for d in boundaries(w):
        line, why = offset([tuple(p) for p in w["pts"]], w.get("bulges"), w.get("closed", False), d)
        if why:
            return None
        out.append(line)
    return out


def ring_area(pts, bulges):
    """The ring's signed area, its vertices taken from its first (map coordinates lose no digits)."""
    n = len(pts)
    ox, oy = pts[0]
    a = 0.0
    for i in range(n):
        x1, y1 = pts[i][0] - ox, pts[i][1] - oy
        x2, y2 = pts[(i + 1) % n][0] - ox, pts[(i + 1) % n][1] - oy
        a += x1 * y2 - x2 * y1
    a /= 2.0
    for i in range(n):
        arc = arc_of(pts[i], pts[(i + 1) % n], bulges[i])
        if arc:
            _, r, _, sweep = arc
            a += r * r / 2.0 * (sweep - math.sin(sweep))
    return a


def area(w, ls):
    left, right = ls[0], ls[-1]
    if w.get("closed"):
        return abs(abs(ring_area(*left)) - abs(ring_area(*right)))
    lp, lb = left
    rp, rb = right
    # The right face back: its vertices reversed, each edge's bulge negated.
    back_p = list(reversed(rp))
    back_b = [-rb[len(rp) - 2 - i] for i in range(len(rp) - 1)] + [0.0]
    ring = list(lp) + back_p
    b = list(lb[:-1]) + [0.0] + back_b
    return abs(ring_area(ring, b))


def pieces(w, detail, scale):
    ls = lines(w)
    comps = w.get("components", [])
    order = {"schematic": 0, "standard": 1, "detailed": 2}
    shown = lambda c: order[detail] >= 1 and order[detail] >= order[c["detail"]]  # noqa: E731
    out = []
    if detail == "schematic":
        if w.get("fill"):
            out.append({"kind": "fill", "color": w["fill"]})
    else:
        first = math.degrees(math.atan2(w["pts"][1][1] - w["pts"][0][1], w["pts"][1][0] - w["pts"][0][0]))
        for c in comps:
            h = c.get("hatch")
            if not h or not shown(c):
                continue
            if h["pattern"].upper() == "SOLID":
                out.append({"kind": "hatch", "pattern": "SOLID"})
            else:
                out.append({"kind": "hatch", "pattern": h["pattern"].upper(), "angle": first + h["angle"], "scale": h["scale"] * scale / 1000.0})
    out += [{"kind": "face"}, {"kind": "face"}]
    if not w.get("closed") and w.get("caps") != "open":
        out += [{"kind": "cap"}, {"kind": "cap"}]
    for k in range(1, len(comps)):
        if shown(comps[k - 1]) and shown(comps[k]):
            out.append({"kind": "boundary", "at": k})
    return out


def P(x, y):
    return [float(x), float(y)]


EXTERIOR = [
    {"name": "İç sıva", "thickness": 0.02, "detail": "detailed"},
    {"name": "Tuğla", "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0}, "detail": "standard"},
    {"name": "Isı yalıtımı", "thickness": 0.05, "hatch": {"pattern": "ANSI37", "scale": 0.5, "angle": 0.0}, "detail": "standard"},
    {"name": "Dış sıva", "thickness": 0.03, "detail": "detailed"},
]

CASES = [
    ("düz duvar, ortası yolda", {"pts": [P(0, 0), P(4, 0)], "width": 0.2, "justify": "center"}),
    ("L köşe, iki yüz gönyeli", {"pts": [P(0, 0), P(4, 0), P(4, 3)], "width": 0.2, "justify": "center"}),
    ("U yol, sol yüz yolda, ötelemeyle", {"pts": [P(0, 0), P(5, 0), P(5, 4), P(0, 4)], "width": 0.25, "justify": "left", "offset": 0.05}),
    ("yaylı kenarın iki yanı", {"pts": [P(0, 0), P(6, 0), P(9, 3)], "bulges": [0.0, 0.25, 0.0], "width": 0.2, "justify": "center"}),
    ("yarım daire duvar, sağ yüz yolda", {"pts": [P(-5, 0), P(5, 0)], "bulges": [-1.0, 0.0], "width": 0.2, "justify": "right"}),
    ("kapalı dikdörtgen, ortası yolda", {"pts": [P(0, 0), P(4, 0), P(4, 3), P(0, 3)], "closed": True, "width": 0.2, "justify": "center"}),
    ("yuvarlak oda: iki yarım daire, kapalı", {"pts": [P(3, 0), P(-3, 0)], "bulges": [1.0, 1.0], "closed": True, "width": 0.3, "justify": "center"}),
    ("sivri köşe pahlanır", {"pts": [P(0, 0), P(10, 0), P(0, 1)], "width": 0.2, "justify": "center"}),
    ("yaydan sonra teğet düz kenar", {"pts": [P(0, 0), P(4, 4), P(8, 4)], "bulges": [0.41421356237309503, 0.0, 0.0], "width": 0.2, "justify": "left"}),
    ("yay ile düz kenar teğetsiz birleşir", {"pts": [P(0, 0), P(4, 0), P(4, 5)], "bulges": [0.3, 0.0, 0.0], "width": 0.24, "justify": "center"}),
    ("dış duvarın bileşenleri", {"pts": [P(0, 0), P(6, 0), P(6, 4)], "width": 0.3, "justify": "left", "components": EXTERIOR, "fill": "#5A5A5A"}),
    ("uçları açık duvar", {"pts": [P(0, 0), P(3, 0)], "width": 0.1, "justify": "center", "caps": "open"}),
    ("kapalı, sağ yüz yolda, ötelemeyle", {"pts": [P(0, 0), P(6, 0), P(6, 5), P(0, 5)], "closed": True, "width": 0.25, "justify": "right", "offset": -0.03}),
    ("harita koordinatlarında", {"pts": [P(487000, 4420000), P(487012.5, 4420003.75)], "width": 0.25, "justify": "center"}),
    ("yay duvardan dar: çizilemez", {"pts": [P(0, 0), P(0.1, 0)], "bulges": [1.0, 0.0], "width": 0.2, "justify": "center"}),
    ("katı betonarme bileşen", {"pts": [P(0, 0), P(5, 0)], "width": 0.25, "justify": "center",
                                  "components": [{"name": "Betonarme", "hatch": {"pattern": "SOLID", "scale": 1.0, "angle": 0.0, "color": "#A6A6A6"}, "detail": "standard"}],
                                  "fill": "#3C3C3C"}),
]


def bulges_of(w):
    b = w.get("bulges")
    if b is None:
        return None
    return b


def case(name, w):
    ls = lines(w)
    out = {"name": name, "wall": w}
    if ls is None:
        out["collapses"] = True
        return out
    out["lines"] = [{"pts": [list(p) for p in pts], "bulges": b} for pts, b in ls]
    out["area"] = area(w, ls)
    out["pieces"] = {d: pieces(w, d, s) for d, s in (("schematic", 100.0), ("standard", 50.0), ("detailed", 20.0))}
    return out


def document():
    return {
        "format": "kentos.wall-cases",
        "version": 1,
        "note": "ADR 0339 §1: a wall's lines (the path offset to the boundaries' distances: straight edges along their normal, arcs about their centres, joins at the supports' crossing nearest the vertex within 4·|d|, else a bevel), its body's area (exact on arcs) and its pieces at the three details (plot scales 100, 50 and 20). Written by scripts/fixtures/wall_cases.py without KentOS code; numbers within 1e-9.",
        "cases": [case(n, w) for n, w in CASES],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"Duvarın çizimi: {len(CASES)} durum başvuruyla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
