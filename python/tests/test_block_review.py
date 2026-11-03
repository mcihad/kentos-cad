"""CAD-ADV-06: the block editor's review uses the public, read-only command plan."""
from pathlib import Path
import json
import math
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class BlockReview(unittest.TestCase):
    def test_closing_arc_binding_in_a_block_plans_follows_and_breaks_only_when_opened(self):
        raw = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())["setup"]
        raw["entities"] = [
            dict(kind="polyline", id=1, layerId="a", attrs={}, closed=True,
                 pts=[dict(x=0, y=0), dict(x=4, y=0), dict(x=0, y=4)],
                 bulges=[0, 0, .25], widths=[[0, 0], [0, 0], [.1, .2]]),
            dict(kind="dimension", id=2, layerId="a", attrs={},
                 a=dict(x=0, y=0), b=dict(x=4, y=0), height=.2, offset=.5),
        ]
        doc = cad.Document.from_bytes(json.dumps(raw).encode())
        source, dimension = [r.uid for r in doc.entities()]
        block = cad.blocks.define(doc, name="Son yay kenarı", base=(0, 0), uids=[source], units="m")
        insert = cad.blocks.insert(doc, block=block.block, at=(0, 0), layer_id="a")
        args = dict(uid=dimension, points=[
            cad.PointBinding(point="a", to=cad.PlaceOn(uid=insert.uid, path=[0],
                place=cad.VertexSubPlace(part=0, ring=0, index=0))),
            cad.PointBinding(point="b", to=cad.PlaceOn(uid=insert.uid, path=[0],
                place=cad.MidSubPlace(part=0, ring=0, index=2))),
        ])
        before = doc.to_bytes()
        plan = cad.dimensions.reassociate.plan(doc, **args)
        self.assertAlmostEqual(plan.value, math.sqrt(4.25))
        self.assertEqual(doc.to_bytes(), before)
        done = cad.dimensions.reassociate(doc, **args)
        self.assertEqual(done.value, plan.value)
        tied = doc.to_bytes()
        cad.entities.transform(doc, uids=[insert.uid], transform=cad.MoveTransform(dx=5, dy=3))
        d = doc.entity(dimension).entity
        self.assertAlmostEqual(d.b.x, 4.5)
        self.assertAlmostEqual(d.b.y, 5)
        self.assertTrue(all("broken" not in r for r in d.refs.to_json().values() if isinstance(r, dict)))
        moved = doc.to_bytes()
        doc.undo()
        self.assertEqual(doc.to_bytes(), tied)
        doc.redo()
        self.assertEqual(doc.to_bytes(), moved)
        path = doc.blocks()[0].entities[0].to_json()
        path["closed"] = False
        path["bulges"][-1] = 0
        path.pop("widths")  # The only wide edge was the closing one; the open path is thin.
        cad.blocks.edit(doc, operation="update", block=block.block, entities=[cad.Entity.from_json(path)])
        d = doc.entity(dimension).entity
        refs = d.refs.to_json()
        self.assertNotIn("broken", refs["a"])
        self.assertIn("broken", refs["b"])
        self.assertAlmostEqual(d.b.x, 4.5)
        self.assertAlmostEqual(d.b.y, 5)
        doc.undo()
        self.assertEqual(doc.to_bytes(), moved)

    def test_plan_keeps_the_source_and_guards_the_identical_single_undo_write(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/dynamic-blocks.kcad")
        before = doc.to_bytes()
        block = doc.blocks()[0]
        pieces = list(block.entities)
        pieces.append(cad.Entity.from_json({
            "kind": "line", "id": 5, "layerId": "", "attrs": {},
            "a": {"x": .06, "y": .45}, "b": {"x": .3, "y": .45},
        }))
        args = dict(operation="update", block=block.id, entities=pieces)
        plan = cad.blocks.edit.answer(doc, **args, op="plan")
        self.assertEqual(plan.status, "completed")
        self.assertTrue(plan.plan_id)
        self.assertEqual(plan.summary.blocks["changed"], [block.id])
        self.assertEqual(plan.summary.changed, [])
        self.assertEqual(doc.to_bytes(), before)
        done = cad.blocks.edit.answer(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        self.assertEqual(done.status, "completed")
        self.assertEqual(done.summary.blocks, plan.summary.blocks)
        self.assertEqual(done.summary.changed, plan.summary.changed)
        self.assertEqual(len(doc.blocks()[0].entities), 5)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
