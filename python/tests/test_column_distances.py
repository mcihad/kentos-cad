"""Kolonun geçici ölçüleri (docs/adr/0712) through the public SDK: a column's four clear distances read, the column moved
by one with cad.arch.edit's distance, the room that deducts it following in the same undo step."""
import json
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing() -> cad.Document:
    """A 6 × 4 m room of 20 cm walls on level zemin, a 40 × 40 cm column at (2, 1.5); the head of
    cad.spaces.columns.json's drawing (its levels and layers)."""
    snapshot = json.loads((ROOT / "fixtures/commands/v1/cad.spaces.columns.json").read_text())["setup"]
    p = lambda x, y: {"x": x, "y": y}
    snapshot["entities"] = [
        {"kind": "wall", "id": 1, "layerId": "duvar", "attrs": {}, "width": 0.2, "closed": True,
         "arch": {"level": "zemin"}, "pts": [p(0, 0), p(6, 0), p(6, 4), p(0, 4)]},
        {"kind": "column", "id": 2, "layerId": "cizim", "attrs": {}, "width": 0.4, "depth": 0.4,
         "at": p(2, 1.5), "arch": {"level": "zemin"}},
    ]
    return cad.Document.from_bytes(json.dumps(snapshot).encode())


def column_uid(doc: cad.Document) -> str:
    return next(e.uid for e in doc.entities() if e.entity.to_json()["kind"] == "column")


class ColumnDistances(unittest.TestCase):
    def test_four_gaps_then_a_move_the_room_follows_in_one_step(self) -> None:
        doc = drawing()
        uid = column_uid(doc)
        read = cad.columns.distances(doc, uids=[uid])
        gaps = {g.side: round(g.distance, 9) for g in read.columns[0].sides}
        # Below 1.3 − 0.1, right 5.9 − 2.2, above 3.9 − 1.7, left 1.8 − 0.1.
        self.assertEqual(gaps, {0: 1.2, 1: 3.7, 2: 2.2, 3: 1.7})
        room = cad.spaces.create(doc, layer_id="mahal", spaces=[
            cad.NewSpace(seed=cad.Vec2(x=4, y=3), columns="net", arch=cad.ArchInfo(level="zemin"))])
        area = cad.spaces.quantities(doc).spaces[0].area
        self.assertAlmostEqual(area, 5.8 * 3.8 - 0.16, places=9)
        before = doc.to_bytes()
        operation = cad.ArchOperation.from_json({"kind": "distance", "side": 3, "distance": 1.0})
        planned = cad.arch.edit.plan(doc, uids=[uid], operation=operation)
        self.assertEqual(doc.to_bytes(), before, "the plan writes nothing")
        self.assertEqual(planned.elements[0].to_json()["at"], {"x": 1.3, "y": 1.5})
        cad.arch.edit(doc, uids=[uid], operation=operation)
        self.assertAlmostEqual(doc.entity(uid).entity.to_json()["at"]["x"], 1.3, places=12)
        hole = doc.entity(room.uids[0]).entity.to_json()["parts"][0]["holes"][0]["pts"]
        self.assertAlmostEqual(min(q["x"] for q in hole), 1.1, places=9, msg="the room's hole went with it")
        self.assertAlmostEqual(cad.spaces.quantities(doc).spaces[0].area, area, places=9)
        self.assertEqual(round(cad.columns.distances(doc, uids=[uid]).columns[0].sides[3].distance, 9), 1.0)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before, "one undo takes the column and the room back")

    def test_a_tied_column_refuses_and_says_how(self) -> None:
        doc = drawing()
        uid = column_uid(doc)
        grids = cad.grids.create(doc, layer_id="cizim", grids=[
            cad.NewGrid(pts=[cad.Vec2(x=-1, y=1.5), cad.Vec2(x=7, y=1.5)], label="1"),
            cad.NewGrid(pts=[cad.Vec2(x=2, y=-1), cad.Vec2(x=2, y=5)], label="A")])
        cad.arch.edit(doc, uids=[uid], operation=cad.ArchOperation.from_json(
            {"kind": "attach", "grids": [grids.uids[1], grids.uids[0]]}))
        self.assertTrue(cad.columns.distances(doc, uids=[uid]).columns[0].tied)
        before = doc.to_bytes()
        with self.assertRaises(cad.KentosError) as refused:
            cad.arch.edit(doc, uids=[uid], operation=cad.ArchOperation.from_json(
                {"kind": "distance", "side": 3, "distance": 1.0}))
        self.assertEqual(refused.exception.code, "column_on_grids")
        self.assertIn("Bağı kopar", str(refused.exception))
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
