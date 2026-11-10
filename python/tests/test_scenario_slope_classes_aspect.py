"""Senaryo ölçüleri'nin eğim sınıfları ve bakısı (`cad.scenarios.impact`'in `slope.classes` ve `slope.aspect`'i,
docs/adr/4660; plan/map-cad.md MAP-10), held by the test's own computation (no KentOS code in the expectations).

The drawing is the shared case's (fixtures/commands/v1/cad.scenarios.impact.json, “eğim: Zemin yüzeyinde …”): the
terrain surface “Zemin” of four points A (−10, −10, 100), B (125, −10, 103), C (125, 70, 121), D (−10, 80, 106). The test
works the surface out on its own (Delaunay by the empty circle; each triangle's slope the steepest rise of its plane, its
aspect the direction the plane falls, clockwise from north, in ArcGIS's eight directions) and integrates the konut areas
on a 10 cm grid (each cell's centre in its triangle):

- the area of each slope class (0–20 %, at least 20 %) on each side, within the grid's error;
- the area of each aspect: one triangle falls south, the other south-west; the other directions and Düz have none;
- the classes' areas add up to the area on the surface;
- refused: breaks not rising, a break not above 0.
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
DIRECTIONS = ["aspectN", "aspectNE", "aspectE", "aspectSE", "aspectS", "aspectSW", "aspectW", "aspectNW"]


def circle_holds(a, b, c, d) -> bool:
    ax, ay, bx, by, cx, cy = a[0], a[1], b[0], b[1], c[0], c[1]
    det = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / det
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / det
    return math.hypot(d[0] - ux, d[1] - uy) < math.hypot(ax - ux, ay - uy)


def triangles():
    a, b, c, d = (GROUND[k] for k in "ABCD")
    return [(a, b, c), (a, c, d)] if not circle_holds(a, b, c, d) else [(a, b, d), (b, c, d)]


def plane(t) -> tuple[float, float]:
    """The plane z = c + a·x + b·y through the triangle: (a, b)."""
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = t
    det = (x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1)
    a = ((z2 - z1) * (y3 - y1) - (z3 - z1) * (y2 - y1)) / det
    b = ((x2 - x1) * (z3 - z1) - (x3 - x1) * (z2 - z1)) / det
    return a, b


def slope(t) -> float:
    a, b = plane(t)
    return 100 * math.hypot(a, b)


def aspect(t) -> str:
    """Where the plane falls (−a, −b), clockwise from north, in the eight directions, each from its lower bound."""
    a, b = plane(t)
    if a == 0 and b == 0:
        return "aspectFlat"
    direction = math.degrees(math.atan2(-a, -b)) % 360
    return DIRECTIONS[int((direction + 22.5) // 45) % 8]


def inside(t, x, y) -> bool:
    (x1, y1, _), (x2, y2, _), (x3, y3, _) = t
    d1 = (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1)
    d2 = (x3 - x2) * (y - y2) - (y3 - y2) * (x - x2)
    d3 = (x1 - x3) * (y - y3) - (y1 - y3) * (x - x3)
    return (d1 >= 0 and d2 >= 0 and d3 >= 0) or (d1 <= 0 and d2 <= 0 and d3 <= 0)


def integrate(rects, breaks, step=0.1):
    """The area of each slope class and of each aspect over the cells on the surface, and the area on it."""
    tris = [(t, slope(t), aspect(t)) for t in triangles()]
    classes = [0.0] * (len(breaks) + 1)
    aspects: dict[str, float] = {}
    on = 0.0
    for x0, y0, x1, y1 in rects:
        nx, ny = round((x1 - x0) / step), round((y1 - y0) / step)
        for i in range(nx):
            for j in range(ny):
                x, y = x0 + (i + 0.5) * step, y0 + (j + 0.5) * step
                hit = next(((s, w) for t, s, w in tris if inside(t, x, y)), None)
                if hit is None:
                    continue
                cell = step * step
                on += cell
                classes[sum(1 for b in breaks if hit[0] >= b)] += cell
                aspects[hit[1]] = aspects.get(hit[1], 0.0) + cell
    return classes, aspects, on


def konut(layer: str):
    out = []
    for e in CASE["setup"]["entities"]:
        if e["layerId"] == layer and e["attrs"].get("Kullanim") == "konut":
            xs = [p["x"] - X0 for p in e["pts"]]
            ys = [p["y"] - Y0 for p in e["pts"]]
            out.append((min(xs), min(ys), max(xs), max(ys)))
    return out


class ScenarioSlopeClassesAspect(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASE["setup"])))

    def measured(self, **slope) -> dict:
        out = cad.scenarios.impact(self.doc, scenario="alt-a", slope=cad.ImpactSlope.from_json(slope)).to_json()
        return {m["id"]: m for m in out["measures"]}

    def test_the_classes_and_aspects_of_the_konut_areas(self) -> None:
        self.assertEqual(sorted(aspect(t) for t in triangles()), ["aspectS", "aspectSW"])
        m = self.measured(surface="Zemin", threshold=10.0, classes=[20.0], aspect=True, **{"from": ["konut"]})
        for side, layer in (("base", "karar"), ("alternative", "alt-a-karar")):
            classes, aspects, on = integrate(konut(layer), [20.0])
            # The grid's error: a cell straddling the diagonal is counted whole on one side (≈ 0.1 m × the diagonal).
            for k, want in enumerate(classes):
                self.assertAlmostEqual(m[f"slopeClass{k + 1}"][side], want, delta=15.0, msg=(side, k))
            for name in ["aspectFlat"] + DIRECTIONS:
                self.assertAlmostEqual(m[name][side], aspects.get(name, 0.0), delta=15.0, msg=(side, name))
            self.assertAlmostEqual(m["slopeClass1"][side] + m["slopeClass2"][side], on, delta=1e-6 + 0.01 * on)

    def test_refusals(self) -> None:
        command = cad.command("cad.scenarios.impact")
        for classes in ([15.0, 5.0], [0.0, 5.0]):
            input_ = {"scenario": "alt-a", "slope": {"surface": "t-1", "threshold": 10.0, "classes": classes}}
            answer = command.run(self.doc, command.input_type.from_json(input_), op="plan")
            self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", "slope.classes"), classes)


if __name__ == "__main__":
    unittest.main()
