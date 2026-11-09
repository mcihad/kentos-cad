"""Köşelerden üçgenlenmiş taban (`road.terrains.stockpile` base ``triangulated``; docs/adr/3940; plan/map-cad.md
MAP-15) through the SDK, held by this test's own arithmetic (no KentOS code in the expectations).

On the shared cases' drawing (scripts/fixtures/road_terrain_stockpile_cases.py's setup: a pile on a sloping ground, the
pentagon round it):

- the base's triangles cover the pentagon (their plan areas add up to its area) and each corner's height is the
  surface's height at that corner of the boundary;
- the net volume (cut − fill) over the triangulated base is the fitted plane's plus, over each base triangle, the
  plane's integral less the triangle's (its area times its corners' mean height): the two bases differ only there;
- a boundary whose corners are on a line is refused (``invalid_stockpile``).
"""

from __future__ import annotations

import json
import sys
import unittest
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.terrains.stockpile.json").read_text("utf-8"))


def case_input(name: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"] == name)["steps"][0]["input"]


PENTAGON = case_input("Beşgen, köşelere oturan eğimli düzlem (en küçük kareler)")["boundary"]
LINE = case_input("Köşeleri bir doğru üstünde: eğimli düzlem çizilemez")["boundary"]


def area(t) -> F:
    (ax, ay, _), (bx, by, _), (cx, cy, _) = [[F(v) for v in c] for c in t]
    return ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)) / 2


class StockpileTriangulated(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))

    def test_the_triangulated_base_differs_from_the_plane_by_its_triangles(self) -> None:
        tri = cad.road.terrains.stockpile(self.doc, surface="t-1", boundary=PENTAGON, base="triangulated")
        fit = cad.road.terrains.stockpile(self.doc, surface="t-1", boundary=PENTAGON, base="fit")
        self.assertIs(tri.plane, cad.UNSET)
        self.assertEqual(len(tri.triangles), 3)
        self.assertAlmostEqual(float(sum(area(t) for t in tri.triangles)), tri.area, places=6)
        heights = sorted(round(h, 9) for h in tri.corners)
        self.assertEqual(sorted({round(c[2], 9) for t in tri.triangles for c in t}), heights)
        a, b, c = (F(v) for v in fit.plane)
        shift = sum(
            area(t) * ((a * sum(F(v[0]) for v in t) + b * sum(F(v[1]) for v in t)) / 3 + c - sum(F(v[2]) for v in t) / 3)
            for t in tri.triangles
        )
        self.assertAlmostEqual(tri.cut - tri.fill, fit.cut - fit.fill + float(shift), places=6)

    def test_corners_on_a_line_are_not_triangulated(self) -> None:
        with self.assertRaises(cad.CommandError) as refused:
            cad.road.terrains.stockpile(self.doc, surface="t-1", boundary=LINE, base="triangulated")
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_stockpile", "boundary"))


if __name__ == "__main__":
    unittest.main()
