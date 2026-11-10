"""Eğimle kot ver's range on any part and ring (`cad.entities.grade` with `part` and `ring`, docs/adr/4980;
plan/map-cad.md MAP-06) through the SDK, held by the test's own arithmetic and scripts/fixtures/grade_cases.py's
reading, read back with the independent `.kcad` reader (tools/kcad/kcad.py; no KentOS code in the expectations).

On the shared cases' two-part area (each part with a hole) and two-part polyline:

- the second part's hole (410, 10), (430, 10), (430, 20), (410, 20) from vertex 1 to 3, 101 m to 101.6 m: 30 m along
  it, 2 %, the vertices 101, 101.2, 101.6; its first vertex, the first part, its hole and the second part's outer ring
  are left;
- 5 % ahead and 3 % back on the polyline's second part meet on its arc: a vertex goes in there (the reading's place and
  elevation), the first part is left;
- a part or a hole the object has not is refused (`wrong_place` at `part` or `ring`), nothing written.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import grade_cases as gc  # noqa: E402
import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

ISLAND, TWO = gc.uid(6), gc.uid(7)


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(gc.setup())))


def saved(doc: cad.Document, uid: str) -> dict:
    _, d = kcad.read(doc.to_bytes())
    return dict(zip(d["uids"], d["entities"]))[uid]


class GradePartRing(unittest.TestCase):
    def test_a_further_part_s_hole(self) -> None:
        doc = drawing()
        before = saved(doc, ISLAND)
        out = cad.entities.grade(doc, uid=ISLAND, from_=1, to=3, part=1, ring=1, start=101.0, end=101.6)
        self.assertAlmostEqual(out.length, 30.0, places=9)
        self.assertAlmostEqual(out.grade, 0.6 / 30.0, places=12)
        e = saved(doc, ISLAND)
        zs = e["parts"][0]["holes"][0]["zs"]
        self.assertIsNone(zs[0], "the first vertex is not in the range")
        for got, want in zip(zs[1:], (101.0, 101.0 + 0.02 * 10.0, 101.6)):
            self.assertAlmostEqual(got, want, places=9)
        self.assertEqual((e["pts"], e.get("zs"), e["holes"]), (before["pts"], before.get("zs"), before["holes"]))
        self.assertIsNone(e["parts"][0].get("zs"))
        doc.undo()
        self.assertIsNone(saved(doc, ISLAND)["parts"][0]["holes"][0].get("zs"), "one step")

    def test_a_high_point_on_a_further_part_s_arc(self) -> None:
        doc = drawing()
        before = saved(doc, TWO)
        after, want = gc.peaked(gc.TWO, 0, 2, ahead=0.05, back=0.03, at=(1, 0))
        out = cad.entities.grade(doc, uid=TWO, from_=0, to=2, part=1, ahead=0.05, back=0.03)
        p = out.point
        self.assertEqual((p.index, p.inserted, p.kind), (2, True, cad.GradePointKind.HIGH))
        self.assertAlmostEqual(p.z, want["point"]["z"], places=9)
        e = saved(doc, TWO)
        part = e["parts"][0]
        self.assertEqual(len(part["pts"]), 4)
        for got, w in zip(part["pts"], after["parts"][0]["pts"]):
            self.assertLess(math.dist((got["x"], got["y"]), (w["x"], w["y"])), 1e-6)
        for got, w in zip(part["bulges"], after["parts"][0]["bulges"]):
            self.assertAlmostEqual(got, w, places=9)
        self.assertEqual((e["pts"], e.get("zs")), (before["pts"], before.get("zs")), "the first part is left")

    def test_refusals(self) -> None:
        command = cad.command("cad.entities.grade")
        doc = drawing()
        for extra, path in [({"part": 2}, "part"), ({"part": 1, "ring": 2}, "ring")]:
            input_ = dict({"uid": ISLAND, "from": 0, "to": 1, "start": 100.0, "grade": 0.01}, **extra)
            answer = command.run(doc, command.input_type.from_json(input_), op="plan")
            self.assertEqual((answer.error.code, answer.error.path), ("wrong_place", path), extra)


if __name__ == "__main__":
    unittest.main()
