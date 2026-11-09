"""An arc drawn with chords in the boundary graph (`cad.parcels.boundaries`, docs/adr/3420; plan/map-cad.md MAP-08)
through the SDK, held by scripts/fixtures/parcel_boundaries_cases.py's own reading (mpmath; no KentOS code in the
expectations).

- A2 draws A1's arc with three corners on it: one `approxArc` problem, its corners no missing corners.
- A3's corners rounded to the centimetre: without `arc_tolerance` only the corner at the arc's top (exactly on it) is
  a missing corner; with 0.01 m the arc drawn with chords, the reading's strip and widest place.
- A negative distance is refused with `invalid_input`.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import parcel_boundaries_cases as pb  # noqa: E402


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(pb.setup(pb.APPROX))))


class ApproxArc(unittest.TestCase):
    def test_corners_on_the_arc_make_one_problem(self) -> None:
        out = cad.parcels.boundaries(drawing(), uids=[pb.uid(1), pb.uid(2)])
        want = pb.graph(pb.APPROX, [0, 1])["problems"]
        self.assertEqual([p.kind for p in out.problems], ["approxArc"])
        self.assertEqual(list(out.problems[0].boundaries), want[0]["boundaries"])
        self.assertAlmostEqual(out.problems[0].area, want[0]["area"], places=9)
        self.assertAlmostEqual(out.problems[0].gap, want[0]["gap"], places=9)

    def test_rounded_corners_need_the_distance(self) -> None:
        doc = drawing()
        plain = cad.parcels.boundaries(doc, uids=[pb.uid(1), pb.uid(3)])
        self.assertEqual([p.kind for p in plain.problems], ["missingVertex"])
        out = cad.parcels.boundaries(doc, uids=[pb.uid(1), pb.uid(3)], arc_tolerance=0.01)
        want = pb.graph(pb.APPROX, [0, 2], 0.01)["problems"]
        self.assertEqual([p.kind for p in out.problems], ["approxArc"])
        self.assertAlmostEqual(out.problems[0].area, want[0]["area"], places=9)
        self.assertAlmostEqual(out.problems[0].gap, want[0]["gap"], places=9)

    def test_a_negative_distance_is_refused(self) -> None:
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.parcels.boundaries(drawing(), arc_tolerance=-0.01)
        self.assertEqual(caught.exception.code, "invalid_input")


if __name__ == "__main__":
    unittest.main()
