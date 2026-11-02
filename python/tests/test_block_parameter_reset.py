"""CAD-ADV-06: inspector reset choices use the same named public operation."""
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class BlockParameterReset(unittest.TestCase):
    def test_named_and_catalog_resets_preserve_unrelated_visibility_and_undo(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/dynamic-blocks.kcad")
        uid = next(doc.entities()).uid
        cad.blocks.parameters(doc, operation="set", uids=[uid],
                              values={"Genişlik": 1.05, "Görünüş": "Sade"})
        before = doc.to_bytes()
        definition = [b.to_json() for b in doc.blocks()]
        for name in ("Genişlik", "Tip"):
            args = dict(operation="reset", uids=[uid], names=[name])
            plan = cad.blocks.parameters.answer(doc, **args, op="plan")
            self.assertEqual(plan.status, "completed")
            self.assertEqual(doc.to_bytes(), before)
            done = cad.blocks.parameters.answer(doc, **args,
                                                expected=cad.Expected(plan=plan.plan_id))
            self.assertEqual(done.status, "completed")
            values = cad.blocks.parameters(doc, operation="get", uids=[uid]).inserts[0].values
            self.assertEqual(values["Genişlik"], .9)
            self.assertEqual(values["Görünüş"], "Sade")
            self.assertEqual([b.to_json() for b in doc.blocks()], definition)
            doc.undo()
            self.assertEqual(doc.to_bytes(), before)
