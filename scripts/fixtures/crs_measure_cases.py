#!/usr/bin/env python3
"""Independent reference of lengths and areas in the second system's plane (docs/adr/0167 §2).

Writes fixtures/geodesy/v1/measure.json from PROJ (pyproj) and the ADR's rule, no KentOS code: a path or the rings of
an area, given in the project's system, measured in another projected system's plane. Each straight segment's ends,
and each arc segment as straight pieces whose sagitta is at most 0.1 mm (docs/adr/0149's bound), are taken into the
second system the way PROJ takes them (the EPSG paths of crs_transform_cases.py); the lengths are the sums of the
straight pieces there, the areas the shoelace areas there about each ring's first point, an area's holes taken from its
outer ring.

An arc segment is the DXF bulge form: bulge = tan(θ/4), θ the included angle, positive counter-clockwise; its centre is
on the chord's left normal at chord·(1 − b²)/(4b), its radius chord·(1 + b²)/(4|b|). It is cut into
n = max(1, ⌈|θ| / (2·acos(1 − 0.0001/r))⌉) equal pieces (one when r ≤ 0.0001), its ends exactly the vertices.

A geographic second system has no plane: none is measured (`why`: `geographic`). Neither has the Pseudo-Mercator,
whose scale is another at every latitude (`mercator`).

The geometry core (crates/shared/geometry-core/tests/crs_measure.rs) and the web through its WASM must give the same:
lengths within 1e-6 m, areas within 1e-6 m² for every 100 m of their perimeter (the points agree with PROJ's within
nanometres, and an area moves by its perimeter times that).
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as reference  # noqa: E402  (the EPSG paths and the registry's systems)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "geodesy" / "v1" / "measure.json"

# The largest sagitta of an arc's straight pieces (m).
CHORD = 1e-4


def arc_points(a, b, bulge):
    """The arc segment from `a` to `b` as points, `a` first and `b` last; a straight one is its ends."""
    if abs(bulge) <= 1e-12:
        return [a, b]
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    if chord < 1e-12:
        return [a, b]
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = chord * (1 + bulge * bulge) / (4 * abs(bulge))
    sweep = 4 * math.atan(bulge)
    n = 1 if r <= CHORD else max(1, math.ceil(abs(sweep) / (2 * math.acos(1 - CHORD / r))))
    a0 = math.atan2(a[1] - c[1], a[0] - c[0])
    inner = [(c[0] + r * math.cos(a0 + sweep * i / n), c[1] + r * math.sin(a0 + sweep * i / n)) for i in range(1, n)]
    return [a, *inner, b]


def ring_points(pts, bulges, closed):
    """Every point of a path or a ring, arcs cut; a ring's first point is not repeated."""
    out = [tuple(pts[0])]
    count = len(pts) if closed else len(pts) - 1
    for i in range(count):
        a, b = tuple(pts[i]), tuple(pts[(i + 1) % len(pts)])
        bulge = bulges[i] if bulges and i < len(bulges) else 0.0
        out.extend(arc_points(a, b, bulge)[1:])
    return out[:-1] if closed else out


def length(points, closed):
    pieces = [math.hypot(q[0] - p[0], q[1] - p[1]) for p, q in zip(points, points[1:])]
    if closed:
        pieces.append(math.hypot(points[0][0] - points[-1][0], points[0][1] - points[-1][1]))
    return math.fsum(pieces)


def area(points):
    """The shoelace area about the ring's first point: the grid's millions do not take the area's digits."""
    o = points[0]
    rel = [(x - o[0], y - o[1]) for x, y in points]
    return abs(math.fsum(p[0] * q[1] - q[0] * p[1] for p, q in zip(rel, rel[1:] + rel[:1]))) / 2


def measure(src, dst, rings, closed):
    """The path's length, or the rings' perimeter and net area, in `dst`'s plane; None and why when there is none."""
    if dst["kind"] == "geographic":
        return None, "geographic"
    if dst.get("projection") == "Pseudo-Mercator":
        return None, "mercator"
    t, _, _ = reference.transformer(src, dst)
    total, net = 0.0, 0.0
    for i, ring in enumerate(rings):
        points = ring_points(ring["pts"], ring.get("bulges"), closed)
        moved = [t.transform(x, y) for x, y in points]
        if any(not (math.isfinite(x) and math.isfinite(y)) for x, y in moved):
            return None, "unreachable"
        total += length(moved, closed)
        if closed:
            net += area(moved) if i == 0 else -area(moved)
    return ({"length": total, "area": net} if closed else {"length": total}), None


def shapes():
    """The shapes measured, in their project's coordinates (TM30 at İstanbul, TM36 at Ankara)."""
    ist, ank = (414000.0, 4540000.0), (486500.0, 4420200.0)
    at = lambda o, pts: [[o[0] + x, o[1] + y] for x, y in pts]  # noqa: E731
    return [
        ("açık yol, üç köşe", 5254, at(ist, [(0, 0), (60.25, 12.5), (84.75, -30.125)]), None, False),
        ("açık yol, yaylı kenarla", 5254, at(ist, [(0, 0), (40, 0), (40, 30)]), [0.0, 0.4], False),
        ("kapalı alan, 40 × 25 m", 5254, at(ist, [(0, 0), (40, 0), (40, 25), (0, 25)]), None, True),
        ("kapalı alan, yaylı kenarlı ve delikli", 5256, at(ank, [(0, 0), (120, 0), (120, 80), (0, 80)]), [0.0, 0.25, 0.0, -0.1], True),
        ("büyük parsel, 1 km", 5256, at(ank, [(0, 0), (1000, 0), (1000, 1000), (0, 1000)]), None, True),
    ]


HOLE = [[486540.0, 4420230.0], [486560.0, 4420230.0], [486560.0, 4420250.0], [486540.0, 4420250.0]]

# The second systems each shape is measured in.
SECONDS = {
    5254: [2320, 5255, 32635, 5252, 3857],
    5256: [23036, 32636, 5257, 2322, 4326],
}


def build():
    registry = json.loads(reference.REGISTRY.read_text(encoding="utf-8"))
    entries = {e["srid"]: e for e in registry["systems"] if e["kind"] != "local"}
    cases = []
    for name, srid, pts, bulges, closed in shapes():
        rings = [{"pts": pts, **({"bulges": bulges} if bulges else {})}]
        if "delikli" in name:
            rings.append({"pts": HOLE})
        for second in SECONDS[srid]:
            src, dst = entries[srid], entries[second]
            expect, why = measure(src, dst, rings, closed)
            case = {"name": f"{src['name']} → {dst['name']}, {name}", "from": reference.system(src), "to": reference.system(dst),
                    "fromSrid": srid, "toSrid": second, "rings": rings, "closed": closed, "expect": expect}
            if why:
                case["why"] = why
            cases.append(case)
    pieces = [{"a": [0, 0], "b": [40, 0], "bulge": b, "points": len(arc_points((0, 0), (40, 0), b))} for b in (0.0, 0.1, 0.4, 1.0, -0.25)]
    return {"format": "kentos.crs-measure", "version": 1, "proj": __import__("pyproj").proj_version_str, "chord": CHORD,
            "measure": cases, "pieces": pieces}


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what PROJ and the rule give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kuraldan çıkan değil; yeniden yazın: python3 {Path(__file__).relative_to(ROOT)}",
                  file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} PROJ'dan ve kuraldan çıkanla aynı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    data = json.loads(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(data['measure'])} ölçü, {len(data['pieces'])} yay bölüşü.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
