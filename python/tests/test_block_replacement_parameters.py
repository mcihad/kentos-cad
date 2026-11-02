"""CAD-ADV-06: replacement preserves compatible instance inputs and remains saveable."""
from pathlib import Path
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing():
    doc = cad.Document.open(ROOT / "fixtures/kcad/v2/dynamic-blocks.kcad")
    return doc, doc.blocks()[0], next(doc.entities()).uid


class BlockReplacementParameters(unittest.TestCase):
    def test_incompatible_plain_target_refuses_before_any_write_and_the_file_still_opens(self):
        doc, source, uid = drawing()
        target = cad.blocks.edit(doc, operation="duplicate", block=source.id, name="Düz kapı").added[0]
        cad.blocks.edit(doc, operation="dynamic", block=target, dynamic=None)
        cad.blocks.parameters(doc, operation="set", uids=[uid], values={"Genişlik": 1.05})
        before = doc.to_bytes()
        for op in ("validate", "plan", "execute"):
            result = cad.blocks.replace.answer(doc, from_=source.id, to=target, uids=[uid], op=op)
            self.assertEqual(result.status, "failed")
            self.assertEqual(result.error.code, "invalid_parameter_value")
            self.assertEqual(result.error.path, "uids[0].parameters.Genişlik")
            self.assertEqual(doc.to_bytes(), before)
            self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(), before)

    def test_compatible_inputs_survive_selected_replacement_and_target_formulas_recompute(self):
        doc, source, uid = drawing()
        dynamic = source.dynamic.to_json()
        dynamic["parameters"].append({
            "kind": "linear", "name": "Double", "from": {"x": 0, "y": 0},
            "to": {"x": 1.8, "y": 0}, "rule": {"expression": "@Genişlik * 2"},
        })
        cad.blocks.edit(doc, operation="dynamic", block=source.id,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        target = cad.blocks.edit(doc, operation="duplicate", block=source.id, name="Yeni kapı").added[0]
        dynamic["parameters"][-1]["rule"]["expression"] = "@Genişlik * 3"
        cad.blocks.edit(doc, operation="dynamic", block=target,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        cad.blocks.parameters(doc, operation="set", uids=[uid],
                              values={"Genişlik": 1.05, "Görünüş": "Sade"})
        before = doc.to_bytes()
        before_entities = {r.uid: r.entity.to_json() for r in doc.entities()}
        definitions = [b.to_json() for b in doc.blocks()]
        args = dict(from_=source.id, to=target, uids=[uid])
        plan = cad.blocks.replace.answer(doc, **args, op="plan")
        self.assertEqual(plan.status, "completed")
        self.assertEqual(doc.to_bytes(), before)
        done = cad.blocks.replace.answer(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        self.assertEqual(done.status, "completed")
        for r in doc.entities():
            entity = r.entity.to_json()
            if r.uid != uid:
                self.assertEqual(entity, before_entities[r.uid])
                continue
            self.assertEqual(entity["block"], target)
            self.assertEqual(entity["parameters"]["Genişlik"], 1.05)
            self.assertEqual(entity["parameters"]["Görünüş"], "Sade")
            self.assertAlmostEqual(entity["parameters"]["Double"], 3.15)
            for key in ("p", "scale", "rotation", "attrs", "layerId"):
                self.assertEqual(entity[key], before_entities[uid][key])
        self.assertEqual([b.to_json() for b in doc.blocks()], definitions)
        saved = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)

    def test_a_length_override_cannot_be_reinterpreted_as_an_angle(self):
        doc, source, uid = drawing()
        target = cad.blocks.edit(doc, operation="duplicate", block=source.id, name="Açılı kapı").added[0]
        dynamic = source.dynamic.to_json()
        dynamic["parameters"][0] = {
            "kind": "angle", "name": "Genişlik", "center": {"x": 0, "y": 0}, "base": .9,
        }
        dynamic["actions"] = []
        # Lookup rows remain valid numeric values; replacement still must check parameter kind.
        cad.blocks.edit(doc, operation="dynamic", block=target, dynamic=cad.DynamicBlock.from_json(dynamic))
        cad.blocks.parameters(doc, operation="set", uids=[uid], values={"Genişlik": 1.05})
        before = doc.to_bytes()
        result = cad.blocks.replace.answer(doc, from_=source.id, to=target, uids=[uid])
        self.assertEqual(result.status, "failed")
        self.assertEqual(result.error.code, "invalid_parameter_value")
        self.assertEqual(doc.to_bytes(), before)
