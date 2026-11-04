#!/usr/bin/env python3
"""A superelevation's axis of rotation and physics, shared cases (docs/adr/0931 §1, plan/road.md Y27.1), from an
independent reference: the axis's lift in exact fractions (the held edge keeping its normal-crown elevation); each
stretch's relative gradient by fitting the edge-over-axis elevation's parabola through three exact points of the
stretch (its ends and middle) and reading its slope at the ends, not by the product rule; the side friction from the
point-mass balance V²/(gR) = (e + f)/(1 − e·f) solved for f at 40 digits. No KentOS code.

    python3 scripts/fixtures/road_superelevation_physics_cases.py           (--check: fails when the file differs)
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

mp.mp.dps = 40
OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "superelevation-physics.json"
G = mp.mpf("9.80665")


def along(points, s):
    if not points:
        return F(0)
    if s <= points[0][0]:
        return F(points[0][1])
    if s >= points[-1][0]:
        return F(points[-1][1])
    for (a, va), (b, vb) in zip(points, points[1:]):
        if a <= s <= b and b > a:
            return F(va) + (F(vb) - F(va)) * (s - F(a)) / (F(b) - F(a))
    return F(points[-1][1])


def side_of(pivot, left, right):
    return {"centerline": None, "leftEdge": True, "rightEdge": False,
            "insideEdge": left <= right, "outsideEdge": left > right}[pivot]


def lift(pivot, slopes, normal, widths):
    held = side_of(pivot, F(slopes[0]), F(slopes[1]))
    if held is None:
        return F(0)
    i = 0 if held else 1
    return F(widths[i]) * (F(normal[i]) - F(slopes[i])) / 100


def gradients(way, pivot, frm, to):
    stops = {F(frm), F(to)}
    for k in ("leftSlope", "rightSlope", "leftWidth", "rightWidth"):
        stops |= {F(p[0]) for p in way.get(k, []) if F(frm) < F(p[0]) < F(to)}
    stops = sorted(stops)
    out = []
    for a, b in zip(stops, stops[1:]):
        m = (a + b) / 2
        held = side_of(pivot, along(way.get("leftSlope", []), m), along(way.get("rightSlope", []), m))

        def edge(x, is_left):
            # Elevation of an edge over the axis of rotation at x (m), from the description directly.
            wl = along(way.get("leftWidth", []), x) * along(way.get("leftSlope", []), x) / 100
            wr = along(way.get("rightWidth", []), x) * along(way.get("rightSlope", []), x) / 100
            own = wl if is_left else wr
            if held is None:
                return own
            if held == is_left:
                return F(0)
            return own - (wl if held else wr)

        def slopes_at_ends(is_left):
            # The parabola through (a, ya), (m, ym), (b, yb): its slope at a and b.
            ya, ym, yb = edge(a, is_left), edge(m, is_left), edge(b, is_left)
            h = (b - a) / 2
            c2 = (ya - 2 * ym + yb) / (2 * h * h)
            c1 = (yb - ya) / (2 * h)  # slope at the middle
            return abs(c1 - c2 * (b - a)), abs(c1 + c2 * (b - a))

        la, lb = slopes_at_ends(True)
        ra, rb = slopes_at_ends(False)
        out.append({"from": float(a), "to": float(b), "left": float(max(la, lb)), "right": float(max(ra, rb))})
    return out


def friction(speed, radius, e):
    v = mp.mpf(speed) / mp.mpf("3.6")
    a = v * v / (G * mp.mpf(radius))
    return float((a - mp.mpf(e)) / (1 + mp.mpf(e) * a))


def toward(left, right, left_turn):
    l, r = (-F(left), F(right)) if left_turn else (F(left), -F(right))
    return float(min(l, r) / 100)


# A two-lane road, 3.5 m lanes: the outside (right) turns from normal crown to full 6 % over 100–160 m, the inside
# (left) holds −2.5 % until the reverse crown then follows; the inside widens 0 → 0.8 m over the same stretch.
RUNOFF = {
    "leftSlope": [[100, -2.5], [126.47058823529412, -2.5], [160, -6]],
    "rightSlope": [[100, -2.5], [160, 6]],
    "leftWidth": [[0, 3.5]],
    "rightWidth": [[0, 3.5]],
}
WIDENED = {**RUNOFF, "leftWidth": [[100, 3.5], [160, 4.3]]}

CASES = {
    "lift": [
        {"name": name, "pivot": p, "slopes": s, "normal": [-2.5, -2.5], "widths": w,
         "lift": float(lift(p, s, [-2.5, -2.5], w))}
        for name, p, s, w in [
            ("Eksen etrafında: kalkış yok", "centerline", [-6, 6], [3.5, 3.5]),
            ("İç kenar etrafında, sola dönen kurp", "insideEdge", [-6, 6], [3.5, 3.5]),
            ("Dış kenar etrafında, sola dönen kurp", "outsideEdge", [-6, 6], [3.5, 3.5]),
            ("İç kenar etrafında, sağa dönen kurp", "insideEdge", [6, -6], [3.5, 3.5]),
            ("Sol kenar etrafında, genişlemiş sol", "leftEdge", [-6, 6], [4.3, 3.5]),
            ("Sağ kenar etrafında", "rightEdge", [-6, 6], [3.5, 3.5]),
            ("Normal eğimde iç kenar: kalkış yok", "insideEdge", [-2.5, -2.5], [3.5, 3.5]),
            ("Düz eğimli dış yan, iç kenar", "insideEdge", [-2.5, 0], [3.5, 3.5]),
        ]
    ],
    "gradients": [
        {"name": name, "pivot": p, "way": way, "from": 80.0, "to": 180.0, "rows": gradients(way, p, 80, 180)}
        for name, p, way in [
            ("Eksen etrafında", "centerline", RUNOFF),
            ("İç kenar etrafında: dış kenar bütün genişliği döner", "insideEdge", RUNOFF),
            ("Dış kenar etrafında", "outsideEdge", RUNOFF),
            ("Sol kenar etrafında", "leftEdge", RUNOFF),
            ("Genişleyen iç yan, eksen etrafında", "centerline", WIDENED),
            ("Genişleyen iç yan, dış kenar etrafında", "outsideEdge", WIDENED),
        ]
    ],
    "friction": [
        {"name": name, "speed": v, "radius": r, "e": e, "friction": friction(v, r, e),
         "approximate": v * v / (127 * r) - e}
        for name, v, r, e in [
            ("80 km/sa, R 300 m, %6 dever", 80.0, 300.0, 0.06),
            ("50 km/sa, R 80 m, %8 dever", 50.0, 80.0, 0.08),
            ("Ters eğimli dış yan (−%2,5)", 60.0, 400.0, -0.025),
            ("Deversiz", 70.0, 500.0, 0.0),
            ("Yavaş araç: sürtünme içe (eksi)", 20.0, 300.0, 0.06),
        ]
    ],
    "toward": [
        {"name": name, "left": l, "right": r, "leftTurn": t, "e": toward(l, r, t)}
        for name, l, r, t in [
            ("Sola dönen kurpta tam dever", -6, 6, True),
            ("Sağa dönen kurpta tam dever", 6, -6, False),
            ("Sola dönen kurpta normal eğim: dış yan ters", -2.5, -2.5, True),
            ("Sola dönen kurpta düz dış yan", -2.5, 0, True),
        ]
    ],
}


def main():
    text = json.dumps(CASES, ensure_ascii=False, indent=1) + "\n"
    n = sum(len(v) for v in CASES.values())
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the reference writes")
        print(f"{OUT.name}: {n} cases, the same")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({n} cases)")


if __name__ == "__main__":
    main()
