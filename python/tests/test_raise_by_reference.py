"""Referanstan kot ver (`cad.entities.raise`, docs/adr/3780; plan/map-cad.md MAP-06) through the SDK, held by this
test's own arithmetic (no KentOS code in the expectations).

On the shared cases' drawing (scripts/fixtures/raise_cases.py's setup):

- the kerb's fourth vertex 2 % up from the door's threshold: the vertex takes 849.75 + 0.02·d, d the straight horizontal
  distance; every other elevation moves by as much, the one without stays without; one undo takes it back;
- the two-part area by a difference: the outer ring, its hole and the other part (its hole without elevations) move
  together, the answer counts what moved and what has none;
- the vertex standing on the reference answers no grade (`cad.UNSET`);
- a vertex without an elevation is refused with `elevation_missing`, nothing written.
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
import raise_cases as rc  # noqa: E402

X0, Y0 = 500000.0, 4400000.0
THRESHOLD = cad.RaiseReference(x=X0 - 20.0, y=Y0 + 10.0, z=849.75)


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rc.setup())))


def entity(doc: cad.Document, slot: int) -> dict:
    return doc.entity(rc.uid(slot)).entity.to_json()


class RaiseByReference(unittest.TestCase):
    def test_the_vertex_takes_the_grade_from_the_reference_and_the_rest_follow(self) -> None:
        doc = drawing()
        d = math.sqrt(100.0**2 + 30.0**2)  # (80, 40) from (−20, 10)
        new = 849.75 + 0.02 * d
        by = new - 851.25
        out = cad.entities.raise_(doc, uid=rc.uid(1), at=3, reference=THRESHOLD, grade=0.02)
        self.assertAlmostEqual(out.distance, d, places=9)
        self.assertAlmostEqual(out.after, new, places=9)
        self.assertAlmostEqual(out.by, by, places=9)
        self.assertAlmostEqual(out.grade, 0.02, places=15)
        self.assertEqual((out.moved, out.unset), (4, 1))
        zs = entity(doc, 1)["zs"]
        want = [850.0 + by, None, 850.4 + by, new, 851.0 + by]
        for got, w in zip(zs, want):
            self.assertTrue((got is None and w is None) or abs(got - w) < 1e-9, (got, w))
        # The kerb's own grades stay: the first edge's rise is the same after.
        self.assertAlmostEqual(zs[4] - zs[3], 851.0 - 851.25, places=9)
        doc.undo()
        self.assertEqual(entity(doc, 1)["zs"], [850.0, None, 850.4, 851.25, 851.0], "one step")

    def test_an_area_moves_whole_its_holes_and_parts_too(self) -> None:
        doc = drawing()
        ref = cad.RaiseReference(x=X0 + 260.0, y=Y0 - 15.0, z=120.75)
        out = cad.entities.raise_(doc, uid=rc.uid(3), at=0, reference=ref, difference=0.05)
        self.assertAlmostEqual(out.after, 120.8, places=9)
        self.assertAlmostEqual(out.by, 120.8 - 120.0, places=9)
        d = math.sqrt(60.0**2 + 15.0**2)  # (200, 0) from (260, −15)
        self.assertAlmostEqual(out.grade, 0.05 / d, places=12)
        # Outer ring 4, its hole 3, the other part 2 of 3; the other part's hole's 3 and one vertex without.
        self.assertEqual((out.moved, out.unset), (9, 4))
        e = entity(doc, 3)
        self.assertAlmostEqual(e["holes"][0]["zs"][2], 120.0 + 0.8, places=9)
        self.assertAlmostEqual(e["parts"][0]["zs"][2], 121.4 + 0.8, places=9)
        self.assertIsNone(e["parts"][0]["zs"][1])
        self.assertNotIn("zs", e["parts"][0]["holes"][0])

    def test_on_the_reference_no_grade(self) -> None:
        doc = drawing()
        on = cad.RaiseReference(x=X0 + 30.0, y=Y0 + 40.0, z=849.0)
        out = cad.entities.raise_(doc, uid=rc.uid(1), at=2, reference=on, grade=0.05)
        self.assertEqual((out.distance, out.after), (0.0, 849.0))
        self.assertIs(out.grade, cad.UNSET)

    def test_a_vertex_without_an_elevation_is_refused(self) -> None:
        doc = drawing()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.entities.raise_(doc, uid=rc.uid(1), at=1, reference=THRESHOLD, grade=0.01)
        self.assertEqual(caught.exception.code, "elevation_missing")
        self.assertEqual(entity(doc, 1)["zs"], [850.0, None, 850.4, 851.25, 851.0])


if __name__ == "__main__":
    unittest.main()
