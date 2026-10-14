"""The object snap as a command from Python (docs/adr/0272): ``kentos.cad.snap.find`` reads what a click would
snap to, with the object's persistent id and its place, and the answer binds a dimension as it is
(``cad.dimensions.create``'s ``place`` points), as the desktop's dimension tool binds the snap it took.

The drawing is the interaction traces' (fixtures/interaction/v1/edits.kcad): line 2 runs north from
(486980, 4420006) to (486980, 4420018).
"""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"


def drawing() -> cad.Document:
    return cad.Document.from_bytes(EDITS.read_bytes())


def uid_of(doc: cad.Document, slot: int) -> str:
    return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == slot)


class SnapTests(unittest.TestCase):
    def test_a_line_end_is_read_with_its_object_and_place(self) -> None:
        doc = drawing()
        found = cad.snap.find(doc, at=cad.Vec2(486980.1, 4420006.1), aperture=0.5)
        self.assertEqual(len(found.snaps), 1)
        s = found.snaps[0]
        self.assertEqual(s.kind, "endpoint")
        self.assertEqual((s.point.x, s.point.y), (486980.0, 4420006.0))
        self.assertEqual(s.uid, uid_of(doc, 2))
        self.assertEqual(s.place.to_json(), {"kind": "start"})
        self.assertEqual(found.revision, doc.revision)

    def test_the_answer_binds_a_dimension(self) -> None:
        doc = drawing()
        ends = [
            cad.snap.find(doc, at=cad.Vec2(486980.1, y), aperture=0.5).snaps[0]
            for y in (4420006.1, 4420017.9)
        ]
        points = [cad.PlaceMeasuredPoint(uid=s.uid, place=s.place) for s in ends]
        layer = next(r.entity.layer_id for r in doc.entities(page_size=1000) if r.entity.id == 2)
        made = cad.dimensions.create(
            doc,
            measure=cad.AlignedDimensionMeasure(a=points[0], b=points[1]),
            at=cad.Vec2(486982.0, 4420012.0),
            layer_id=layer,
        )
        dim = next(r.entity for r in doc.entities(page_size=1000) if r.uid == made.created[0]).to_json()
        self.assertEqual(dim["refs"]["a"]["uid"], uid_of(doc, 2))
        self.assertEqual(dim["refs"]["a"]["place"], {"kind": "start"})
        self.assertEqual(dim["refs"]["b"]["place"], {"kind": "end"})

    def test_every_candidate_best_first(self) -> None:
        doc = drawing()
        found = cad.snap.find(doc, at=cad.Vec2(486980.1, 4420006.1), aperture=0.5, all=True)
        self.assertGreaterEqual(len(found.snaps), 1)
        self.assertEqual(found.snaps[0].kind, "endpoint")


if __name__ == "__main__":
    unittest.main()
