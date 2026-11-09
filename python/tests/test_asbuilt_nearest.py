"""Yüzeye en yakın uzaklık (`cad.asbuilt.check`'s `nearest`; docs/adr/4020; plan/map-cad.md MAP-19) through the SDK,
held by this test's own geometry (no KentOS code in the expectations).

On the shared cases' drawing (scripts/fixtures/asbuilt_cases.py: the design surface the plane
z = 100 + 0.02 x + 0.01 y over [0, 100]² from the project's origin), the surface alone:

- each point's nearest place is on the plane, the point less it is along the plane's normal (−0.02, −0.01, 1), and its
  length is the nearest distance, signed as the height difference;
- the nearest distance is the height difference over √(1 + 0.02² + 0.01²);
- held to the nearest distance (`surfaceBy: nearest`), a limit between P3's two measures leaves P3 within.
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

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.asbuilt.check.json").read_text("utf-8"))
ORIGIN = (CASES["setup"]["origin"]["x"], CASES["setup"]["origin"]["y"])
K = math.sqrt(1 + 0.02**2 + 0.01**2)


def plane(x: float, y: float) -> float:
    return 100.0 + 0.02 * (x - ORIGIN[0]) + 0.01 * (y - ORIGIN[1])


class AsbuiltNearest(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))

    def test_the_nearest_place_is_along_the_plane_s_normal(self) -> None:
        out = cad.asbuilt.check(self.doc, measured="olcum", surface="Tasarım")
        seen = 0
        for p in out.points:
            if p.surface is cad.UNSET:
                continue
            q = p.nearest_at
            self.assertAlmostEqual(q[2], plane(q[0], q[1]), places=9, msg=p.name)
            self.assertAlmostEqual(p.nearest, p.surface / K, places=9, msg=p.name)
            # The point less its nearest place, along the normal, as long as the distance.
            point = self.doc.entity(p.uid).entity.to_json()
            v = (point["p"]["x"] - q[0], point["p"]["y"] - q[1], point["z"] - q[2])
            self.assertAlmostEqual(math.hypot(*v), abs(p.nearest), places=9, msg=p.name)
            self.assertAlmostEqual(v[0] * 1.0 + v[2] * 0.02, 0.0, places=9, msg=p.name)
            self.assertAlmostEqual(v[1] * 1.0 + v[2] * 0.01, 0.0, places=9, msg=p.name)
            seen += 1
        self.assertGreater(seen, 5)

    def test_the_limit_held_to_the_nearest_distance(self) -> None:
        out = cad.asbuilt.check(self.doc, measured="olcum", surface="Tasarım")
        p3 = next(p for p in out.points if p.name == "P3")
        limit = (abs(p3.surface) + abs(p3.nearest)) / 2
        flags = lambda by: next(p.flags for p in cad.asbuilt.check(  # noqa: E731
            self.doc, measured="olcum", surface="Tasarım", vertical=limit, surface_by=by).points if p.name == "P3")
        self.assertIn("outsideSurface", flags(cad.UNSET))
        self.assertNotIn("outsideSurface", flags(cad.AsbuiltSurfaceBy.NEAREST))


if __name__ == "__main__":
    unittest.main()
