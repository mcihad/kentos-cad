"""Presentation groups share the native command, file contract and guarded AI surface."""
import copy
import json
from pathlib import Path
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class BlockParameterGroups(unittest.TestCase):
    def drawing(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/block-publication.kcad")
        return doc, doc.blocks()[0]

    def test_guarded_group_edit_preserves_all_geometry_and_values_roundtrips_and_undo(self):
        doc, block = self.drawing()
        before = doc.to_bytes()
        records = [r.entity.to_json() for r in doc.entities()]
        dynamic = block.to_json()["dynamic"]
        dynamic["parameterGroups"] = [dict(name="Kanat", parameters=["Yön", "Genişlik"]),
                                      dict(name="Ürün", parameters=["Tip"])]
        args = dict(operation="dynamic", block=block.id, dynamic=cad.DynamicBlock.from_json(dynamic))
        plan = cad.blocks.edit.answer(doc, **args, op="plan")
        self.assertEqual(plan.status, "completed")
        self.assertEqual(doc.to_bytes(), before)
        cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        after = doc.to_bytes()
        self.assertEqual([r.entity.to_json() for r in doc.entities()], records)
        self.assertEqual(doc.blocks()[0].to_json()["card"], block.to_json()["card"])
        self.assertEqual(doc.blocks()[0].to_json()["dynamic"], dynamic)
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)

    def test_invalid_membership_and_names_refuse_validate_plan_and_execute_atomically(self):
        for groups in ([dict(name="Kanat", parameters=["Genişlik", "Genişlik"])],
                       [dict(name="Kanat", parameters=["Olmayan"])],
                       [dict(name="KANAT", parameters=["Tip"]), dict(name="kanat", parameters=["Yön"])],
                       [dict(name="x" * 65, parameters=["Tip"])],
                       [dict(name="Kanat", parameters=[])]):
            doc, block = self.drawing()
            before = doc.to_bytes()
            dynamic = block.to_json()["dynamic"]
            dynamic["parameterGroups"] = copy.deepcopy(groups)
            for op in ("validate", "plan", "execute"):
                answer = cad.blocks.edit.answer(doc, operation="dynamic", block=block.id,
                                               dynamic=cad.DynamicBlock.from_json(dynamic), op=op)
                self.assertEqual(answer.status, "failed", (groups, op, answer))
                self.assertEqual(answer.error.code, "invalid_dynamic_block")
                self.assertEqual(doc.to_bytes(), before)

    def test_clearing_groups_restores_automatic_presentation_without_a_geometry_version(self):
        doc = cad.Document.open(ROOT / "fixtures/kcad/v2/block-parameter-groups.kcad")
        before = doc.to_bytes()
        block = doc.blocks()[0]
        dynamic = block.to_json()["dynamic"]
        dynamic.pop("parameterGroups")
        cad.blocks.edit(doc, operation="dynamic", block=block.id, dynamic=cad.DynamicBlock.from_json(dynamic))
        self.assertEqual(doc.blocks()[0].to_json()["card"], block.to_json()["card"])
        self.assertNotIn("parameterGroups", doc.blocks()[0].to_json()["dynamic"])
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(), doc.to_bytes())
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)

    def test_changed_group_plan_cannot_apply_a_stale_ai_review(self):
        doc, block = self.drawing()
        before = doc.to_bytes()
        dynamic = block.to_json()["dynamic"]
        dynamic["parameterGroups"] = [dict(name="Kanat", parameters=["Genişlik"])]
        args = dict(operation="dynamic", block=block.id, dynamic=cad.DynamicBlock.from_json(dynamic))
        plan = cad.blocks.edit.answer(doc, **args, op="plan")
        dynamic["parameterGroups"][0]["name"] = "Başka başlık"
        args["dynamic"] = cad.DynamicBlock.from_json(dynamic)
        answer = cad.blocks.edit.answer(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        self.assertNotEqual(answer.status, "completed")
        self.assertEqual(doc.to_bytes(), before)

    def test_group_only_change_keeps_formula_caches_and_incoming_nested_overrides(self):
        doc, block = self.drawing()
        dynamic = block.to_json()["dynamic"]
        dynamic["parameters"].append(dict(kind="linear", name="Yarım",
            **{"from": dict(x=0, y=0), "to": dict(x=.45, y=0)},
            rule=dict(expression="@Genişlik / 2")))
        cad.blocks.edit(doc, operation="dynamic", block=block.id,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        library = json.loads(cad.blocks.edit(doc, operation="export").library)
        parent = dict(id="22222222-3333-4444-8555-666666666666", name="Taşıyıcı",
            base=dict(x=0, y=0), entities=[dict(kind="insert", id=1, layerId="", attrs={},
            block=block.id, p=dict(x=0, y=0), scale=1, rotation=0,
            parameters={"Genişlik": 1.2, "Yarım": .6})])
        library["blocks"].append(parent)
        cad.blocks.edit(doc, operation="import", library=json.dumps(library))
        before = doc.to_bytes()
        old_blocks = [b.to_json() for b in doc.blocks()]
        old_instances = [r.entity.to_json() for r in doc.entities()]
        dynamic["parameterGroups"] = [dict(name="Kanat", parameters=["Genişlik", "Yarım"])]
        cad.blocks.edit(doc, operation="dynamic", block=block.id,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        new_blocks = [b.to_json() for b in doc.blocks()]
        self.assertEqual(new_blocks[0]["card"], old_blocks[0]["card"])
        self.assertEqual(new_blocks[1:], old_blocks[1:])
        self.assertEqual([r.entity.to_json() for r in doc.entities()], old_instances)
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(), doc.to_bytes())
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
