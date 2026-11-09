"""Yüzeyden kot ver (`cad.entities.drape`, docs/adr/4180; plan/map-cad.md MAP-06), held by the test's own computation
(no KentOS code in the expectations).

The drawing is the shared cases' (fixtures/commands/v1/cad.entities.drape.json): the surface “Zemin” of four corners
A (0, 0, 100), B (100, 0, 104), C (100, 80, 112), D (0, 100, 106), from X0, Y0. The test works its triangles out on
its own: the circle through A, B, C leaves D out, so they are ABC and ACD, split by AC (y = 0.8 x). It then checks:

- a point inside takes the height of its triangle's plane (worked out here from the three corners);
- a point outside keeps its own height;
- with break points the polyline from (10, 50) to (60, 30) gets a vertex where it crosses AC, at the height AC has
  there, its other vertices unmoved;
- one undo puts every height back.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.entities.drape.json").read_text("utf-8"))
X0, Y0 = 500000.0, 4400000.0
A, B, C, D = (0.0, 0.0, 100.0), (100.0, 0.0, 104.0), (100.0, 80.0, 112.0), (0.0, 100.0, 106.0)


def plane(t, x, y):
    (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = t
    det = (x2 - x1) * (y3 - y1) - (x3 - x1) * (y2 - y1)
    a = ((z2 - z1) * (y3 - y1) - (z3 - z1) * (y2 - y1)) / det
    b = ((x2 - x1) * (z3 - z1) - (x3 - x1) * (z2 - z1)) / det
    return z1 + a * (x - x1) + b * (y - y1)


def ground(x, y):
    """The surface's height at a local place: ABC under AC, ACD over it."""
    return plane((A, B, C) if y <= 0.8 * x else (A, C, D), x, y)


def uid(slot: int) -> str:
    return CASES["setup"]["uids"][slot - 1]


class Drape(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))

    def entities(self) -> dict:
        _, saved = kcad.read(self.doc.to_bytes())
        return dict(zip(saved["uids"], saved["entities"]))

    def test_points_take_their_triangle_s_height_or_keep_their_own(self) -> None:
        inside, outside = uid(5), uid(6)
        out = cad.entities.drape(self.doc, uids=[inside, outside], surface="Zemin").to_json()
        self.assertEqual((out["set"], out["outside"]), (1, 1))
        e = self.entities()
        self.assertAlmostEqual(e[inside]["z"], ground(60.0, 20.0), delta=1e-9)
        self.assertEqual(e[outside]["z"], 99.0)

    def test_a_break_point_goes_where_the_line_crosses_ac(self) -> None:
        line = uid(8)
        before = self.entities()[line]
        out = cad.entities.drape(self.doc, uids=[line], surface="t-1", breaks=True).to_json()
        self.assertEqual(out["inserted"], 1)
        pts = self.entities()[line]["pts"]
        zs = self.entities()[line]["zs"]
        self.assertEqual(len(pts), 4)
        # (10 + 50 t, 50 − 20 t) on y = 0.8 x: t = 0.7, (45, 36).
        x, y = pts[1]["x"] - X0, pts[1]["y"] - Y0
        self.assertAlmostEqual(x, 45.0, delta=1e-6)
        self.assertAlmostEqual(y, 36.0, delta=1e-6)
        self.assertAlmostEqual(zs[1], A[2] + (C[2] - A[2]) * 0.45, delta=1e-6)
        for got, want in zip([pts[0], pts[2], pts[3]], before["pts"]):
            self.assertEqual((got["x"], got["y"]), (want["x"], want["y"]))
        for k, (xl, yl) in ((0, (10.0, 50.0)), (2, (60.0, 30.0)), (3, (90.0, 10.0))):
            self.assertAlmostEqual(zs[k], ground(xl, yl), delta=1e-9)

    def test_one_undo_puts_the_heights_back(self) -> None:
        before = self.entities()
        cad.entities.drape(self.doc, uids=[uid(5), uid(8), uid(9)], surface="t-1", breaks=True)
        self.doc.undo()
        self.assertEqual(self.entities(), before)


if __name__ == "__main__":
    unittest.main()
