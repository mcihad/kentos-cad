"""Kesit kontrolü (`road.terrains.sectioncheck`, docs/adr/5120; BÖHHBÜY m. 93(1)), held by the test's own reading of
the drawing (no KentOS code in the expectations): the shared cases' contours run north–south at x = 0, 15, 35 (a
zigzag through (35, 0)), 50, 72 (a zigzag) and 95 with levels 100 … 105; the section runs along y = 0 from A (−5, 0) to
B (100, 0). A point at x = 25 lies between the 101 contour (x = 15) and the 102 one (x = 35): its height from the
contours is 101.5; at x = 42 between 102 (35) and 103 (50): 102 + 7/15.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import section_check_cases as sc  # noqa: E402  (the drawing only)


class SectionCheck(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(sc.setup())))

    def run_check(self, points, **more) -> tuple[dict, list]:
        input_ = {"from": sc.U("A"), "to": sc.U("B"), "points": [sc.U(p) for p in points], "contours": ["esyukselti"],
                  **more}
        r = cad.road.terrains.sectioncheck.run(self.doc, cad.TerrainsSectionCheck.from_json(input_))
        return r.unwrap().to_json(), [w.code for w in r.warnings]

    def test_heights_come_from_the_two_contours_around_each_point(self) -> None:
        out, warns = self.run_check(["K2", "K3"])
        rows = {round(p["at"][0]): p for p in out["points"]}
        self.assertAlmostEqual(rows[25]["contour"], 101.5, places=12)
        self.assertAlmostEqual(rows[42]["contour"], 102 + 7 / 15, places=12)
        self.assertEqual(rows[42]["between"], [102.0, 103.0])
        diffs = [rows[25]["measured"] - 101.5, rows[42]["measured"] - (102 + 7 / 15)]
        self.assertAlmostEqual(out["rms"], math.sqrt(sum(d * d for d in diffs) / 2), places=12)
        # KOH_Z at 1/1000: 1.33 · (−1.665e-6 · 10⁶ + 17.45 − 1.166) cm; the limit 1.5 times it.
        koh = 1.33 * (-1.665 + 17.45 - 1.166) / 100
        self.assertAlmostEqual(out["limit"], 1.5 * koh, places=12)
        # Three places along 105 m: −5 → 25 → 42 → 100: two gaps past 20 m.
        self.assertEqual([round(g["length"], 9) for g in out["gaps"]], [30.0, 58.0])
        self.assertIn("section_spacing_exceeded", warns)

    def test_a_point_past_the_end_is_outside(self) -> None:
        out, _ = self.run_check(["K1", "K9"])
        self.assertEqual([p["state"] for p in out["points"]], ["compared", "outside"])


if __name__ == "__main__":
    unittest.main()
