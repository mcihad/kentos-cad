"""The road overlaps' regions in İmar planı denetimi's answer (`cad.zoning.check`, docs/adr/3440; plan/map-cad.md
MAP-09) through the SDK, held by scripts/fixtures/zoning_check_cases.py's own reading (no KentOS code in the
expectations).

On the drawing of the planned roads Y1 (12 m) and Y2 with the three parcels:

- every road width and road parcel finding carries its regions, the reading's rings (counter-clockwise from the lowest
  vertex), and the regions' areas add up to the finding's area;
- the other findings carry none.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import zoning_check_cases as zc  # noqa: E402

ROADS = zc.roady(w1="12") + zc.PARCELS


def ring_area(pts) -> float:
    n = len(pts)
    return sum(pts[k].x * pts[(k + 1) % n].y - pts[(k + 1) % n].x * pts[k].y for k in range(n)) / 2


class ZoningRegions(unittest.TestCase):
    def test_road_findings_carry_their_regions(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(zc.setup(ROADS, zc.ROADS))))
        out = cad.zoning.check(doc)
        want = zc.check(ROADS, zc.ROADS)["findings"]
        self.assertEqual([f.kind for f in out.findings], [w["kind"] for w in want])
        roads = 0
        for got, w in zip(out.findings, want):
            regions = got.regions if got.regions is not cad.UNSET else []
            if got.kind not in ("roadWidth", "roadParcel"):
                self.assertEqual(list(regions), [], got.kind)
                continue
            roads += 1
            self.assertEqual([[(q.x, q.y) for q in r.outer.pts] for r in regions],
                             [[(q["x"], q["y"]) for q in r["outer"]["pts"]] for r in w["regions"]])
            # Counter-clockwise rings whose areas add up to the finding's.
            areas = [ring_area(r.outer.pts) for r in regions]
            self.assertTrue(all(a > 0 for a in areas), areas)
            self.assertAlmostEqual(sum(areas), got.area, places=6)
        self.assertGreater(roads, 0)


if __name__ == "__main__":
    unittest.main()
