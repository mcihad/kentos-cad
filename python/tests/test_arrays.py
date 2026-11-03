"""Associative arrays from Python (docs/adr/0300): ``kentos.cad.entities.array`` with
``associative=True`` makes the objects a block and the places its inserts, one array the
document keeps; ``Document.arrays()`` reads it, the block is ``Document.blocks()``'s, the
items are ordinary inserts, a file keeps it (schema 53) and one undo takes it all back.

The shared cases (fixtures/commands/v1/cad.entities.array.assoc.json) run here too through
test_command_cases.py; this test is the reading side.
"""

from __future__ import annotations

import unittest
import json
import math
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"


def drawing() -> cad.Document:
    return cad.Document.from_bytes(EDITS.read_bytes())


def uid_of(doc: cad.Document, slot: int) -> str:
    return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == slot)


def closed_path_drawing():
    setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
    setup["settings"].update(drawingUnit="mm", cadWork="architecture")
    setup["entities"] = [
        dict(kind="circle", id=1, layerId="a", attrs={}, c=dict(x=0., y=0.), r=.12),
        dict(kind="polyline", id=2, layerId="a", attrs={}, closed=True,
             pts=[dict(x=x, y=y) for x, y in [(0., 0.), (4., 0.), (4., 4.), (0., 4.)]]),
    ]
    doc = cad.Document.from_bytes(json.dumps(setup).encode())
    records = list(doc.entities())
    return doc, records[0].uid, records[1].uid


class ArrayTests(unittest.TestCase):
    def test_closed_path_preview_committed_items_and_closure_edits_agree_in_millimetre_projects(self):
        doc, source, path = closed_path_drawing()
        layout = cad.PathArrayLayout(path=path, count=8, align=False)
        before = doc.to_bytes()
        expected = [(0., 0.), (2., 0.), (4., 0.), (4., 2.), (4., 4.), (2., 4.), (0., 4.), (0., 2.)]
        plan = cad.entities.array.plan(doc, uids=[source], layout=layout, associative=True)
        self.assertEqual([(e.c.x, e.c.y) for e in plan.entities], expected[1:])
        self.assertEqual(doc.to_bytes(), before)
        made = cad.entities.array(doc, uids=[source], layout=layout, associative=True)
        self.assertEqual([(doc.entity(u).entity.p.x, doc.entity(u).entity.p.y) for u in made.created], expected)
        after = doc.to_bytes()
        cad.curves.edit(doc, uid=path, edit=cad.OpenCurveEdit())
        opened = doc.to_bytes()
        self.assertEqual((doc.entity(made.created[-1]).entity.p.x, doc.entity(made.created[-1]).entity.p.y), (0., 4.))
        cad.curves.edit(doc, uid=path, edit=cad.CloseCurveEdit())
        self.assertEqual(doc.to_bytes(), after)
        self.assertEqual([i["uid"] for i in doc.arrays()[0]["items"]], made.created)
        doc.undo(); self.assertEqual(doc.to_bytes(), opened)
        doc.undo(); self.assertEqual(doc.to_bytes(), after)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), after)
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)

    def test_closed_path_spacing_keeps_two_removed_items_and_a_manual_override_until_explicit_reset(self):
        doc, source, path = closed_path_drawing()
        made = cad.entities.array(doc, uids=[source],
            layout=cad.PathArrayLayout(path=path, count=8, spacing=2., align=False), associative=True)
        cad.entities.transform(doc, uids=[made.created[1]], transform=cad.MoveTransform(dx=.5, dy=.25))
        cad.arrays.edit(doc, array=made.array, edit=cad.RemoveArrayEdit(items=[2, 6]))
        before = doc.to_bytes()
        cad.arrays.edit(doc, array=made.array,
            edit=cad.LayoutArrayEdit(layout=cad.PathArrayLayout(path=path, count=12, align=False)))
        items = doc.arrays()[0]["items"]
        self.assertEqual(len(items), 12)
        self.assertNotIn("uid", items[2]); self.assertNotIn("uid", items[6])
        for k in [0, 1, 3, 4, 5, 7]: self.assertEqual(items[k]["uid"], made.created[k])
        held = doc.entity(made.created[1]).entity.p
        self.assertEqual((held.x, held.y), (2.5, .25))
        self.assertTrue(items[1]["kept"])
        expanded = doc.to_bytes()
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), expanded)
        cad.arrays.edit(doc, array=made.array, edit=cad.ResetArrayEdit())
        reset = doc.arrays()[0]["items"]
        self.assertTrue(all(i.get("uid") and not i.get("kept", False) for i in reset))
        for k in [0, 1, 3, 4, 5, 7]: self.assertEqual(reset[k]["uid"], made.created[k])
        self.assertAlmostEqual(doc.entity(reset[-1]["uid"]).entity.p.x, 0.)
        self.assertAlmostEqual(doc.entity(reset[-1]["uid"]).entity.p.y, 4./3.)
        doc.undo(); self.assertEqual(doc.to_bytes(), expanded)

    def test_closing_arc_spacing_uses_the_arc_length_and_tangent_in_both_windings(self):
        for winding in [1., -1.]:
            doc, source, path = closed_path_drawing()
            raw = doc.entity(path).entity.to_json()
            raw.update(pts=[dict(x=0., y=0.), dict(x=4., y=0.)], bulges=[0., winding])
            setup = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
            setup["entities"] = [doc.entity(source).entity.to_json(), raw]
            doc = cad.Document.from_bytes(json.dumps(setup).encode())
            source, path = [r.uid for r in doc.entities()]
            made = cad.entities.array(doc, uids=[source],
                layout=cad.PathArrayLayout(path=path, count=3, spacing=4., align=True), associative=True)
            last = doc.entity(made.created[-1]).entity
            self.assertAlmostEqual(last.p.x, 2.+2.*math.cos(2.), places=12)
            self.assertAlmostEqual(last.p.y, winding*2.*math.sin(2.), places=12)
            angle = winding*(2.+math.pi/2.)
            self.assertAlmostEqual(math.cos(last.rotation), math.cos(angle), places=12)
            self.assertAlmostEqual(math.sin(last.rotation), math.sin(angle), places=12)

    def test_an_associative_grid_is_a_block_its_inserts_and_the_array(self) -> None:
        doc = drawing()
        line = uid_of(doc, 2)
        made = cad.entities.array(
            doc, uids=[line], layout=cad.GridArrayLayout(rows=2, cols=2, dx=5.0, dy=4.0), associative=True
        )
        self.assertIsNotNone(made.array)
        self.assertEqual(len(made.created), 4)
        arrays = doc.arrays()
        self.assertEqual(len(arrays), 1)
        array = arrays[0]
        self.assertEqual(array["id"], made.array)
        self.assertEqual([i["uid"] for i in array["items"]], made.created)
        self.assertEqual(array["layout"], {"kind": "grid", "rows": 2, "cols": 2, "dx": 5.0, "dy": 4.0})
        block = next(b for b in doc.blocks() if b.id == array["block"])
        self.assertEqual(block.name, "Dizi 1")
        kinds = {r.entity.to_json()["kind"] for r in doc.entities(page_size=1000) if r.uid in made.created}
        self.assertEqual(kinds, {"insert"})
        # A file keeps the array; one undo takes the block, the items and the array back.
        again = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(again.arrays(), arrays)
        self.assertEqual(doc.undo(), "Dizi")
        self.assertEqual(doc.arrays(), [])
        self.assertEqual(doc.blocks(), [])


if __name__ == "__main__":
    unittest.main()
