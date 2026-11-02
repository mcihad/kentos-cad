#!/usr/bin/env python3
"""The terrain profile's shared cases (docs/adr/0902 §1, plan/road.md Y4.1), from an independent reference: the surface is
Shewchuk's Triangle's (as scripts/fixtures/road_surface_cases.py), the alignment road_alignment_cases.py's (mpmath at 40
digits); the profile is found by walking the alignment every 5 cm, finding the triangle each place is in, and refining
every change of triangle by bisection on the shared edge's side at 40 digits. No KentOS code, and not KentOS's way (it
cuts every element against every edge).

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath python3 scripts/fixtures/road_profile_cases.py
    ... --check   # fails when fixtures/road/v1/profiles.json is not what this writes

A case gives the surface's input, the alignment's PIs, the crossings (station, height) in order, whether each stretch
between the start, the crossings and the end is off the surface, and random stations' true heights (none off the
surface): a linear reading of KentOS's profile there must be within its tolerance (1 mm).
"""

import json
import math
import random
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as al  # noqa: E402
import road_surface_cases as tri  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "profiles.json"
STEP = 0.05


class Tin:
    def __init__(self, xy, z, tris):
        self.xy, self.z, self.tris = xy, z, tris
        self.cell = 25.0
        self.grid = {}
        for i, t in enumerate(tris):
            xs = [xy[k][0] for k in t]
            ys = [xy[k][1] for k in t]
            for gx in range(int(math.floor(min(xs) / self.cell)), int(math.floor(max(xs) / self.cell)) + 1):
                for gy in range(int(math.floor(min(ys) / self.cell)), int(math.floor(max(ys) / self.cell)) + 1):
                    self.grid.setdefault((gx, gy), []).append(i)

    def bary(self, i, x, y):
        (ax, ay), (bx, by), (cx, cy) = [self.xy[k] for k in self.tris[i]]
        d = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
        wa = ((bx - x) * (cy - y) - (by - y) * (cx - x)) / d
        wb = ((cx - x) * (ay - y) - (cy - y) * (ax - x)) / d
        return wa, wb, 1 - wa - wb

    def find(self, x, y):
        for i in self.grid.get((int(math.floor(x / self.cell)), int(math.floor(y / self.cell))), []):
            if min(self.bary(i, x, y)) >= -1e-12:
                return i
        return None

    def height(self, x, y):
        i = self.find(x, y)
        if i is None:
            return None
        w = self.bary(i, x, y)
        return sum(wk * self.z[k] for wk, k in zip(w, self.tris[i]))


def walk(els, length):
    """Places every STEP along the alignment by float steps of the elements (detection only)."""
    pts = al.fast_points(els, STEP)
    return pts


def shared_edge(tin, i, j):
    """The edge two triangles share, or None (they are not neighbours: the walk stepped over a corner)."""
    a, b = set(tin.tris[i]), set(tin.tris[j])
    common = a & b
    return tuple(sorted(common)) if len(common) == 2 else None


def point_at(els, s):
    e = els[0]
    for x in els:
        if x.station <= s:
            e = x
    return e.point(s - e.station)


def crossing(tin, els, s0, s1, edge):
    """The station where the alignment crosses `edge`, between s0 and s1, by bisection on its side at 40 digits."""
    (ax, ay), (bx, by) = tin.xy[edge[0]], tin.xy[edge[1]]
    ax, ay, bx, by = map(mp.mpf, (ax, ay, bx, by))

    def side(s):
        x, y = point_at(els, s)
        return (x - ax) * (by - ay) - (y - ay) * (bx - ax)

    lo, hi = mp.mpf(s0), mp.mpf(s1)
    flo = side(lo)
    for _ in range(64):
        m = (lo + hi) / 2
        fm = side(m)
        if (fm < 0) == (flo < 0):
            lo, flo = m, fm
        else:
            hi = m
    s = (lo + hi) / 2
    x, y = point_at(els, s)
    rx, ry = bx - ax, by - ay
    u = ((x - ax) * rx + (y - ay) * ry) / (rx * rx + ry * ry)
    za, zb = mp.mpf(tin.z[edge[0]]), mp.mpf(tin.z[edge[1]])
    return float(s), float(za + u * (zb - za))


def hull_edge(tin, t, p0, p1):
    """The edge of triangle t that the step p0 → p1 crosses (entering or leaving the hull)."""
    for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
        (ax, ay), (bx, by) = tin.xy[a], tin.xy[b]
        d0 = (p0[0] - ax) * (by - ay) - (p0[1] - ay) * (bx - ax)
        d1 = (p1[0] - ax) * (by - ay) - (p1[1] - ay) * (bx - ax)
        if (d0 < 0) != (d1 < 0):
            return tuple(sorted((a, b)))
    return None


def changes(tin, els, s0, s1, i, j, depth=0):
    """The crossings between s0 (in triangle i) and s1 (in j): a shared edge's, else the step halved."""
    if i == j:
        return []
    if i is not None and j is not None:
        edge = shared_edge(tin, i, j)
        if edge is not None:
            return [crossing(tin, els, s0, s1, edge)]
    if i is None or j is None:
        p0 = [float(v) for v in point_at(els, mp.mpf(s0))]
        p1 = [float(v) for v in point_at(els, mp.mpf(s1))]
        edge = hull_edge(tin, tin.tris[i if i is not None else j], p0, p1)
        if edge is not None and depth > 0:
            return [crossing(tin, els, s0, s1, edge)]
    assert depth < 60, "the alignment runs through a corner"
    m = (s0 + s1) / 2
    x, y = point_at(els, mp.mpf(m))
    k = tin.find(float(x), float(y))
    return changes(tin, els, s0, m, i, k, depth + 1) + changes(tin, els, m, s1, k, j, depth + 1)


def profile(tin, els, length):
    pts = walk(els, length)
    where = [tin.find(x, y) for _, x, y in pts]
    out = []
    for k in range(1, len(pts)):
        out += changes(tin, els, pts[k - 1][0], pts[k][0], where[k - 1], where[k])
    return out


def case(name, seed, n, box, pis, samples=60):
    rng = random.Random(seed)
    x0, y0, w, h = box
    pts = []
    for _ in range(n):
        x = round(x0 + rng.random() * w, 3)
        y = round(y0 + rng.random() * h, 3)
        z = round(100 + 0.02 * (x - x0) + 0.01 * (y - y0) + 4 * math.sin((x - x0) / 37) + rng.random(), 3)
        pts.append((x, y, z))
    xy = [(x, y) for x, y, _ in pts]
    z = [h for _, _, h in pts]
    tris = tri.triangulate(xy, [])
    tin = Tin(xy, z, tris)
    els, length, _, _ = al.build(pis)
    cross = profile(tin, els, length)
    marks = [0.0] + [s for s, _ in cross] + [float(length)]
    off = []
    for a, b in zip(marks, marks[1:]):
        x, y = point_at(els, mp.mpf((a + b) / 2))
        off.append(tin.height(float(x), float(y)) is None)
    srng = random.Random(seed + 1)
    checks = []
    for _ in range(samples):
        s = srng.random() * float(length)
        x, y = point_at(els, mp.mpf(s))
        checks.append({"station": s, "z": tin.height(float(x), float(y))})
    return {"name": name, "input": {"points": [{"x": x, "y": y, "z": h} for x, y, h in pts]},
            "pis": pis, "length": float(length), "crossings": [{"station": s, "z": v} for s, v in cross],
            "off": off, "checks": checks}


P = al.pi
CASES = [
    case("Düz aks, her yanı yüzeyde", 1, 60, (0, -100, 400, 200), [P("a", 10, -5), P("b", 390, 20)]),
    case("Kurplu ve klotoidli aks", 2, 120, (0, 0, 600, 400),
         [P("a", 20, 50), P("p1", 300, 60, 150, 40, 40), P("b", 560, 360)]),
    case("Yüzeye dışarıdan girip çıkan aks", 3, 80, (100, 100, 300, 300),
         [P("a", 40, 150), P("p1", 250, 260, 120, 30, 30), P("b", 470, 230)]),
    case("Sıkı kurp, dik yüzey", 4, 150, (0, 0, 300, 300),
         [P("a", 20, 20), P("p1", 150, 40, 60, 20, 20), P("b", 160, 280)]),
    case("Büyük koordinatlarda iki kurp", 5, 150, (485000, 4420000, 900, 500),
         [P("a", 485020, 4420050), P("p1", 485300, 4420100, 200, 50, 50), P("p2", 485600, 4420350, 250, 40, 60),
          P("b", 485880, 4420420)]),
    case("Kırık noktalı aks", 6, 70, (0, 0, 300, 300), [P("a", 30, 40), P("p1", 150, 60), P("b", 260, 250)]),
]


def main():
    text = json.dumps({"tolerance": 1e-3, "cases": CASES}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases, {sum(len(c['crossings']) for c in CASES)} crossings)")


if __name__ == "__main__":
    main()
