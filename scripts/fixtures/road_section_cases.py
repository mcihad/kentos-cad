#!/usr/bin/env python3
"""The sample lines' and terrain sections' shared cases (docs/adr/0903 §1, plan/road.md Y5.1), from an independent
reference: the surface is Shewchuk's Triangle's (as scripts/fixtures/road_surface_cases.py), the alignment
road_alignment_cases.py's (mpmath at 40 digits); a sample line's ends are the alignment's place moved square to its
direction, and the terrain along it is found by cutting the line against every triangle's edge at 40 digits, a
crossing's height the edge's own, an end's its triangle's plane. No KentOS code.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath python3 scripts/fixtures/road_section_cases.py
    ... --check   # fails when fixtures/road/v1/sections.json is not what this writes
"""

import json
import random
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as al  # noqa: E402
import road_profile_cases as pref  # noqa: E402
import road_surface_cases as tri  # noqa: E402

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "sections.json"


def edges(tin):
    out = set()
    for t in tin.tris:
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            out.add((min(a, b), max(a, b)))
    return sorted(out)


def section(tin, ends):
    (px, py), (qx, qy) = [(mp.mpf(v[0]), mp.mpf(v[1])) for v in ends]
    dx, dy = qx - px, qy - py
    length = mp.sqrt(dx * dx + dy * dy)
    at = []
    for a, b in edges(tin):
        (ax, ay), (bx, by) = [(mp.mpf(c[0]), mp.mpf(c[1])) for c in (tin.xy[a], tin.xy[b])]
        rx, ry = bx - ax, by - ay
        den = dx * ry - dy * rx
        if den == 0:
            continue
        apx, apy = ax - px, ay - py
        t = (apx * ry - apy * rx) / den
        u = (apx * dy - apy * dx) / den
        if 0 <= t <= 1 and 0 <= u <= 1:
            za, zb = mp.mpf(tin.z[a]), mp.mpf(tin.z[b])
            at.append((t, za + u * (zb - za)))
    at.sort(key=lambda v: v[0])
    out = []
    for i, (t, z) in enumerate([(mp.mpf(0), None)] + at + [(mp.mpf(1), None)]):
        if out and (t - out[-1][0]) * length < mp.mpf("1e-9"):
            continue
        out.append((t, z))
    rows = []
    for i, (t, z) in enumerate(out):
        x, y = px + t * dx, py + t * dy
        if z is None:
            h = tin.height(float(x), float(y))
            z = None if h is None else mp.mpf(h)
        gap = False
        if i:
            m = (t + out[i - 1][0]) / 2
            gap = tin.height(float(px + m * dx), float(py + m * dy)) is None
        rows.append({"offset": float(t * length), "z": None if z is None else float(z), "gap": gap,
                     "crossing": 0 < i < len(out) - 1})
    return rows


def case(name, seed, n, box, pis, lines):
    rng = random.Random(seed)
    x0, y0, w, h = box
    pts = []
    for _ in range(n):
        x = round(x0 + rng.random() * w, 3)
        y = round(y0 + rng.random() * h, 3)
        z = round(100 + 0.03 * (x - x0) - 0.01 * (y - y0) + 2 * rng.random(), 3)
        pts.append((x, y, z))
    xy = [(x, y) for x, y, _ in pts]
    tin = pref.Tin(xy, [z for *_, z in pts], tri.triangulate(xy, []))
    els, length, _, _ = al.build(pis)
    out = []
    for station, left, right in lines:
        e, (x, y) = al.place_at(els, mp.mpf(station))
        th = e.theta(mp.mpf(station) - e.station)
        rx, ry = mp.sin(th), -mp.cos(th)
        ends = [[x - rx * left, y - ry * left], [x + rx * right, y + ry * right]]
        rows = section(tin, ends)
        for r in rows:
            r["offset"] -= left
        out.append({"station": station, "left": left, "right": right,
                    "ends": [[float(v) for v in p] for p in ends], "ground": rows})
    return {"name": name, "input": {"points": [{"x": x, "y": y, "z": z} for x, y, z in pts]}, "pis": pis,
            "length": float(length), "lines": out}


P = al.pi
CASES = [
    case("Düz aks, kesitler yüzeyde", 11, 80, (0, -100, 400, 200), [P("a", 10, -5), P("b", 390, 20)],
         [(50.0, 30.0, 30.0), (200.0, 25.0, 40.0), (350.0, 15.0, 15.0)]),
    case("Kurplu aks, kurpta ve klotoidde kesit", 12, 150, (0, 0, 600, 400),
         [P("a", 20, 50), P("p1", 300, 60, 150, 40, 40), P("b", 560, 360)],
         [(100.0, 30.0, 30.0), (210.0, 40.0, 25.0), (276.0, 35.0, 35.0), (330.0, 20.0, 50.0), (500.0, 30.0, 30.0)]),
    case("Kesit yüzeyin dışına taşıyor", 13, 60, (100, 100, 200, 150),
         [P("a", 60, 170), P("b", 360, 180)], [(60.0, 80.0, 80.0), (150.0, 120.0, 40.0), (290.0, 30.0, 30.0)]),
    case("Büyük koordinatlarda", 14, 120, (485000, 4420000, 500, 300),
         [P("a", 485020, 4420050), P("p1", 485250, 4420100, 200, 40, 40), P("b", 485480, 4420260)],
         [(40.0, 25.0, 25.0), (240.0, 30.0, 30.0), (400.0, 20.0, 35.0)]),
]


def main():
    text = json.dumps({"tolerance": 1e-6, "cases": CASES}, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {len(CASES)} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(CASES)} cases, {sum(len(l['ground']) for c in CASES for l in c['lines'])} places)")


if __name__ == "__main__":
    main()
