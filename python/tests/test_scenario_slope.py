"""Senaryo ölçüleri'nin arazi eğimi (`cad.scenarios.impact`'in `slope`'u, docs/adr/4260; plan/map-cad.md MAP-10), held by
the test's own computation (no KentOS code in the expectations).

The drawing is the shared case's (fixtures/commands/v1/cad.scenarios.impact.json, “eğim: Zemin yüzeyinde …”): the
terrain surface “Zemin” of four points A (−10, −10, 100), B (125, −10, 103), C (125, 70, 121), D (−10, 80, 106), the
field state's decision areas and Alternatif A's. The test works the surface out on its own: the Delaunay diagonal is
the one whose triangles' circles hold no other corner; each triangle's slope is the steepest rise of its plane. It then
integrates the slope over each konut area on a 10 cm grid (each cell's centre in its triangle) and checks:

- the konut areas' mean slope on each side, within the grid's error;
- Alternatif A's konut east of x = 125 lies half off the surface: 50 m² off it, said as a gap of the scenario only;
- the decision changed in the alternative (K2 ticaret-konut → konut) changes the konut slope measure.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.scenarios.impact.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("eğim: Zemin yüzeyinde"))
X0, Y0 = 487000.0, 4420000.0
GROUND = {"A": (-10.0, -10.0, 100.0), "B": (125.0, -10.0, 103.0), "C": (125.0, 70.0, 121.0), "D": (-10.0, 80.0, 106.0)}


def circle_holds(a, b, c, d) -> bool:
    """Whether d lies inside the circle through a, b and c (their circumcentre, by the perpendicular bisectors)."""
    ax, ay, bx, by, cx, cy = a[0], a[1], b[0], b[1], c[0], c[1]
    det = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / det
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / det
    return math.hypot(d[0] - ux, d[1] - uy) < math.hypot(ax - ux, ay - uy)


def triangles():
    a, b, c, d = (GROUND[k] for k in "ABCD")
    return [(a, b, c), (a, c, d)] if not circle_holds(a, b, c, d) else [(a, b, d), (b, c, d)]


def slope(t) -> float:
    """The steepest rise of the plane through the triangle, percent."""
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = t
    u, v = (x2 - x1, y2 - y1, z2 - z1), (x3 - x1, y3 - y1, z3 - z1)
    n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    return math.hypot(n[0], n[1]) / abs(n[2]) * 100


def inside(t, x, y) -> bool:
    (x1, y1, _), (x2, y2, _), (x3, y3, _) = t
    d1 = (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1)
    d2 = (x3 - x2) * (y - y2) - (y3 - y2) * (x - x2)
    d3 = (x1 - x3) * (y - y3) - (y1 - y3) * (x - x3)
    return (d1 >= 0 and d2 >= 0 and d3 >= 0) or (d1 <= 0 and d2 <= 0 and d3 <= 0)


def integrate(rects, step=0.1):
    """Σ slope · cell over the cells on the surface, the area on it, the area off it."""
    tris = [(t, slope(t)) for t in triangles()]
    weighted = on = off = 0.0
    for x0, y0, x1, y1 in rects:
        nx, ny = round((x1 - x0) / step), round((y1 - y0) / step)
        for i in range(nx):
            for j in range(ny):
                x, y = x0 + (i + 0.5) * step, y0 + (j + 0.5) * step
                hit = next((s for t, s in tris if inside(t, x, y)), None)
                if hit is None:
                    off += step * step
                else:
                    weighted += hit * step * step
                    on += step * step
    return weighted, on, off


def konut(layer: str):
    """The konut rectangles of a layer, local."""
    out = []
    for e in CASE["setup"]["entities"]:
        if e["layerId"] == layer and e["attrs"].get("Kullanim") == "konut":
            xs = [p["x"] - X0 for p in e["pts"]]
            ys = [p["y"] - Y0 for p in e["pts"]]
            out.append((min(xs), min(ys), max(xs), max(ys)))
    return out


class ScenarioSlope(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASE["setup"])))

    def measured(self, **slope) -> dict:
        out = cad.scenarios.impact(self.doc, scenario="alt-a", slope=cad.ImpactSlope.from_json(slope)).to_json()
        return {m["id"]: m for m in out["measures"]} | {"gaps": out["gaps"]}

    def test_the_konut_mean_slope_on_each_side(self) -> None:
        m = self.measured(surface="Zemin", threshold=10.0, **{"from": ["konut"]})
        for side, layer in (("base", "karar"), ("alternative", "alt-a-karar")):
            weighted, on, _ = integrate(konut(layer))
            self.assertAlmostEqual(m["slopeMean"][side], weighted / on, delta=0.01, msg=side)

    def test_land_off_the_surface_is_a_gap_of_the_scenario(self) -> None:
        m = self.measured(surface="Zemin", threshold=10.0, **{"from": ["konut"]})
        _, _, off = integrate(konut("alt-a-karar"))
        self.assertAlmostEqual(off, 50.0, delta=1e-6)
        self.assertAlmostEqual(m["slopeOff"]["alternative"], 50.0, delta=1e-6)
        self.assertEqual(m["slopeOff"]["base"], 0.0)
        said = [g for g in m["gaps"] if "yüzeyinin dışında" in g]
        self.assertEqual(len(said), 1)
        self.assertIn("senaryoda", said[0])

    def test_the_changed_decision_changes_the_konut_measure(self) -> None:
        m = self.measured(surface="t-1", threshold=10.0, **{"from": ["konut"]})
        # K2 is konut only in the alternative: its slope enters there.
        self.assertNotAlmostEqual(m["slopeMean"]["base"], m["slopeMean"]["alternative"], places=3)
        self.assertGreater(m["slopeSteep"]["alternative"], m["slopeSteep"]["base"])


if __name__ == "__main__":
    unittest.main()
