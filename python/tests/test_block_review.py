"""CAD-ADV-06: the block editor's review uses the public, read-only command plan."""
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class BlockReview(unittest.TestCase):
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
