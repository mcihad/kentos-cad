"""The same guarded unit correction as the GUI, followed by real parameter editing."""
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class BlockUnitRescale(unittest.TestCase):
    def test_guarded_correction_keeps_defaults_overrides_rules_and_subsequent_edits_consistent(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/block-parameter-groups.kcad")
        block = doc.blocks()[0]
        records = list(doc.entities())
        uid = records[1].uid
        before = doc.to_bytes()
        args = dict(operation="units", block=block.id, units=cad.BlockUnits.MM, rescale=True)
        plan = cad.blocks.edit.answer(doc, **args, op="plan")
        self.assertEqual(plan.status, "completed")
        self.assertEqual(doc.to_bytes(), before)
        cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        corrected = doc.to_bytes()
        values = cad.blocks.parameters(doc, operation="get", uids=[uid]).inserts[0].values
        self.assertEqual(values["Genişlik"], .001)
        self.assertEqual(values["Yön"], True)
        now = list(doc.entities())
        for a, b in zip(records, now):
            self.assertEqual(a.entity.to_json()["p"], b.entity.to_json()["p"])
        dynamic = doc.blocks()[0].to_json()["dynamic"]
        self.assertEqual(dynamic["parameters"][0]["to"]["x"], .0009)
        self.assertEqual(dynamic["parameterGroups"], block.to_json()["dynamic"]["parameterGroups"])
        cad.blocks.parameters(doc, operation="set", uids=[uid], values={"Genişlik": .00105})
        self.assertEqual(cad.blocks.parameters(doc, operation="get", uids=[uid]).inserts[0].values["Genişlik"], .00105)
        doc.undo()
        self.assertEqual(doc.to_bytes(), corrected)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), corrected)
        self.assertEqual(cad.Document.from_bytes(corrected).to_bytes(), corrected)

    def test_unit_label_only_keeps_physical_geometry_and_values(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/block-parameter-groups.kcad")
        block = doc.blocks()[0]
        records = [r.entity.to_json() for r in doc.entities()]
        cad.blocks.edit(doc, operation="units", block=block.id, units=cad.BlockUnits.MM, rescale=False)
        self.assertEqual([r.entity.to_json() for r in doc.entities()], records)
        self.assertEqual(doc.blocks()[0].to_json()["dynamic"], block.to_json()["dynamic"])
