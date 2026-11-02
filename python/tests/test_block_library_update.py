"""Library replacement validates existing scene references before any definition is written."""
from pathlib import Path
import copy
import json
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing(name="dynamic-blocks"):
    doc = cad.Document.open(ROOT / f"fixtures/kcad/v2/{name}.kcad")
    library = json.loads(cad.blocks.edit(doc, operation="export").library)
    return doc, library


class BlockLibraryUpdate(unittest.TestCase):
    def unchanged_refusal(self, doc, library, code):
        before = doc.to_bytes()
        for operation in ("validate", "plan", "execute"):
            answer = cad.blocks.edit.answer(
                doc, operation="import", library=json.dumps(library, ensure_ascii=False),
                replace=True, op=operation,
            )
            self.assertEqual(answer.status, "failed", operation)
            self.assertEqual(answer.error.code, code)
            self.assertEqual(answer.error.path, "library")
            self.assertEqual(doc.to_bytes(), before)
            self.assertEqual(cad.Document.from_bytes(before).to_bytes(), before)

    def test_removing_dynamic_behavior_refuses_and_keeps_the_source_saveable(self):
        doc, library = drawing()
        library["blocks"][0].pop("dynamic")
        self.unchanged_refusal(doc, library, "invalid_parameter_value")

    def test_tightening_a_parameter_refuses_when_a_scene_override_would_be_invalid(self):
        doc, library = drawing()
        dynamic = library["blocks"][0]["dynamic"]
        dynamic["parameters"] = [p for p in dynamic["parameters"] if p["kind"] != "lookup"]
        width = next(p for p in dynamic["parameters"] if p["name"] == "Genişlik")
        width["rule"]["max"] = .95  # Existing root insert has 1.0 m; default .9 m remains valid.
        self.unchanged_refusal(doc, library, "invalid_parameter_value")

    def test_disallowing_a_moved_attribute_refuses_without_dropping_its_position(self):
        doc, library = drawing("block-cards")
        found = False
        for block in library["blocks"]:
            for attribute in block.get("attributes", []):
                if attribute["tag"] == "NO" and attribute.get("movable"):
                    attribute["movable"] = False
                    found = True
        self.assertTrue(found)
        self.unchanged_refusal(doc, library, "attribute_not_movable")

    def test_drawing_file_redefinition_also_refuses_invalid_existing_values(self):
        doc, _ = drawing()
        source, _ = drawing()
        cad.blocks.edit(source, operation="dynamic", block=source.blocks()[0].id, dynamic=None)
        before = doc.to_bytes()
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "plain.kcad"
            source.save(path)
            for operation in ("validate", "plan", "execute"):
                answer = cad.file.import_.block.answer(
                    doc, path=str(path), mode="blocks", replace=True, op=operation,
                )
                self.assertEqual(answer.status, "failed")
                self.assertEqual(answer.error.code, "block_refused")
                self.assertEqual(answer.error.path, "path")
                self.assertEqual(doc.to_bytes(), before)
                self.assertEqual(cad.Document.from_bytes(before).to_bytes(), before)

    def test_compatible_definition_update_keeps_all_instance_data_and_one_undo(self):
        doc, library = drawing()
        before = doc.to_bytes()
        entities = [r.entity.to_json() for r in doc.entities()]
        target = copy.deepcopy(library["blocks"][0])
        target["entities"].append(dict(kind="line", id=5, layerId="", attrs={},
                                      a=dict(x=.06, y=.45), b=dict(x=.3, y=.45)))
        library["blocks"][0] = target
        arguments = dict(operation="import", library=json.dumps(library, ensure_ascii=False), replace=True)
        plan = cad.blocks.edit.answer(doc, **arguments, op="plan")
        self.assertEqual(plan.status, "completed")
        self.assertEqual(doc.to_bytes(), before)
        answer = cad.blocks.edit.answer(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
        self.assertEqual(answer.status, "completed")
        self.assertEqual([r.entity.to_json() for r in doc.entities()], entities)
        self.assertEqual(len(doc.blocks()[0].entities), 5)
        saved = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
