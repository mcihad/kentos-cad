"""Eğimle kot ver's high or low point (`cad.entities.grade` with `ahead` and `back`, docs/adr/3380; plan/map-cad.md
MAP-06) through the SDK, held by scripts/fixtures/grade_cases.py's own reading (no KentOS code in the expectations).

On the shared cases' polyline (a straight edge, an arc edge; its first vertex at 850 m, its fourth at 851.25 m):

- 4 % from the start and 1 % back from the end meet inside the second edge: a vertex is added there at the reading's
  place and elevation, the vertices up to it at 4 %, those after it at 1 % from the end; one undo takes it back;
- 3 % and 2 % meet on the arc: both halves keep its circle (each half's bulge the reading's);
- grades meeting on the middle vertex add none (`inserted` false);
- a plain grade answers without a point (`cad.UNSET`);
- parallel grades are refused with `no_intersection`, nothing written.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import grade_cases as gc  # noqa: E402
import kcad_v2_reference as kr  # noqa: E402

POLY = gc.uid(1)


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(gc.setup())))


def polyline(doc: cad.Document) -> dict:
    return doc.entity(POLY).entity.to_json()


class GradePeak(unittest.TestCase):
    def test_a_high_point_inside_a_straight_edge_adds_a_vertex(self) -> None:
        doc = drawing()
        after, want = gc.peaked(gc.POLY, 0, 3, ahead=0.04, back=0.01)
        out = cad.entities.grade(doc, uid=POLY, from_=0, to=3, ahead=0.04, back=0.01)
        p = out.point
        self.assertEqual((p.index, p.inserted, p.kind), (2, True, cad.GradePointKind.HIGH))
        self.assertAlmostEqual(p.along, want["point"]["along"], places=9)
        self.assertAlmostEqual(p.z, want["point"]["z"], places=9)
        self.assertAlmostEqual(out.back, 0.01, places=15)
        e = polyline(doc)
        self.assertEqual(len(e["pts"]), 6)
        for got, w in zip(e["pts"], after["pts"]):
            self.assertLess(math.dist((got["x"], got["y"]), (w["x"], w["y"])), 1e-6)
        for got, w in zip(e["zs"], after["zs"]):
            self.assertTrue((got is None and w is None) or abs(got - w) < 1e-9, (got, w))
        # Up to the point 4 % from the start; after it 1 % from the end.
        self.assertAlmostEqual(e["zs"][1], 850.0 + 0.04 * 30.0, places=9)
        self.assertAlmostEqual(e["zs"][3], 851.25 + 0.01 * (want["length"] - 70.0), places=9)
        doc.undo()
        self.assertEqual(len(polyline(doc)["pts"]), 5, "one step")

    def test_an_arc_split_keeps_its_circle(self) -> None:
        doc = drawing()
        after, _ = gc.peaked(gc.POLY, 0, 3, ahead=0.03, back=0.02)
        out = cad.entities.grade(doc, uid=POLY, from_=0, to=3, ahead=0.03, back=0.02)
        self.assertEqual((out.point.index, out.point.inserted), (3, True))
        e = polyline(doc)
        for got, w in zip(e["bulges"], after["bulges"]):
            self.assertAlmostEqual(got, w, places=12)
        # The new vertex on the arc's circle: as far from its centre as the arc's ends.
        centre, _ = gc.arc_centre(gc.POLY["pts"][2], gc.POLY["pts"][3], 0.5)
        r = math.dist((centre["x"], centre["y"]), (gc.POLY["pts"][2]["x"], gc.POLY["pts"][2]["y"]))
        self.assertAlmostEqual(math.dist((centre["x"], centre["y"]), (out.point.x, out.point.y)), r, places=6)

    def test_meeting_on_a_vertex_adds_none(self) -> None:
        doc = drawing()
        out = cad.entities.grade(doc, uid=POLY, from_=0, to=2, end=850.625, ahead=0.0625, back=0.03125)
        self.assertEqual((out.point.index, out.point.inserted, out.point.z), (1, False, 851.875))
        self.assertEqual(len(polyline(doc)["pts"]), 5)

    def test_a_plain_grade_has_no_point(self) -> None:
        out = cad.entities.grade(drawing(), uid=POLY, from_=0, to=3, grade=0.02)
        self.assertIs(out.point, cad.UNSET)
        self.assertIs(out.back, cad.UNSET)

    def test_parallel_grades_are_refused(self) -> None:
        doc = drawing()
        before = doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.entities.grade(doc, uid=POLY, from_=0, to=3, ahead=0.02, back=-0.02)
        self.assertEqual(caught.exception.code, "no_intersection")
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
