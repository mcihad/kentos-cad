"""The selection as a command from Python (docs/adr/0275): ``kentos.cad.entities.select`` gives the persistent ids of
a region's objects, as the selection tools select them, and the answer goes to another command's ``uids`` as it is.

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


class SelectTests(unittest.TestCase):
    def test_a_window_gives_what_lies_inside_it(self) -> None:
        doc = drawing()
        got = cad.entities.select(
            doc,
            region=cad.WindowSelectRegion(a=cad.Vec2(486979.0, 4420005.0), b=cad.Vec2(486981.0, 4420019.0)),
        )
        self.assertIn(uid_of(doc, 2), got.uids)
        self.assertEqual(got.revision, doc.revision)
        self.assertEqual(got.left_out.kinds, 0)

    def test_the_kinds_filter_and_count_what_they_leave_out(self) -> None:
        doc = drawing()
        every = cad.entities.select(doc, region=cad.AllSelectRegion())
        lines = cad.entities.select(doc, region=cad.AllSelectRegion(), kinds=["line"])
        kinds = {r.uid: r.entity.to_json()["kind"] for r in doc.entities(page_size=1000)}
        self.assertTrue(all(kinds[u] == "line" for u in lines.uids))
        self.assertEqual(len(lines.uids) + lines.left_out.kinds, len(every.uids))

    def test_the_answer_goes_to_another_command(self) -> None:
        doc = drawing()
        window = cad.WindowSelectRegion(a=cad.Vec2(486979.0, 4420005.0), b=cad.Vec2(486981.0, 4420019.0))
        got = cad.entities.select(doc, region=window, kinds=["line"])
        self.assertTrue(got.uids)
        cad.entities.delete(doc, uids=got.uids)
        again = cad.entities.select(doc, region=window, kinds=["line"])
        self.assertEqual(again.uids, [])

    def test_a_polygon_crossing_itself_is_refused(self) -> None:
        doc = drawing()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.entities.select(
                doc,
                region=cad.PolygonSelectRegion(
                    points=[cad.Vec2(0, 0), cad.Vec2(10, 10), cad.Vec2(10, 0), cad.Vec2(0, 10)],
                    mode="inside",
                ),
            )
        self.assertEqual(caught.exception.code, "invalid_ring")


class PickTests(unittest.TestCase):
    """``kentos.cad.entities.pick`` (docs/adr/0276): what a click can mean, each object with its sub-object."""

    def test_a_click_on_a_line_s_end_gives_its_id_and_place(self) -> None:
        doc = drawing()
        got = cad.entities.pick(doc, at=cad.Vec2(486980.1, 4420006.1), tolerance=0.5)
        first = got.picks[0]
        self.assertEqual(first.uid, uid_of(doc, 2))
        self.assertEqual(first.place.to_json(), {"kind": "start"})
        self.assertEqual(got.revision, doc.revision)

    def test_the_sub_object_binds_a_dimension(self) -> None:
        doc = drawing()
        ends = [
            cad.entities.pick(doc, at=cad.Vec2(486980.1, y), tolerance=0.5).picks[0]
            for y in (4420006.1, 4420017.9)
        ]
        points = [cad.PlaceMeasuredPoint(uid=p.uid, place=p.place) for p in ends]
        layer = next(r.entity.layer_id for r in doc.entities(page_size=1000) if r.entity.id == 2)
        made = cad.dimensions.create(
            doc,
            measure=cad.AlignedDimensionMeasure(a=points[0], b=points[1]),
            at=cad.Vec2(486982.0, 4420012.0),
            layer_id=layer,
        )
        dim = next(r.entity for r in doc.entities(page_size=1000) if r.uid == made.created[0]).to_json()
        self.assertEqual(dim["refs"]["b"]["place"], {"kind": "end"})


if __name__ == "__main__":
    unittest.main()
