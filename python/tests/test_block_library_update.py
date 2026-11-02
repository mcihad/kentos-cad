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


class BlockLibraryBatchUpdate(unittest.TestCase):
    def nested_library(self):
        doc, library = drawing()
        child = copy.deepcopy(library["blocks"][0])
        child["id"] = "22222222-3333-4444-8555-666666666666"
        child["name"] = "Yeni çocuk"
        parent = library["blocks"][0]
        parent["entities"].append(dict(kind="insert", id=5, layerId="", attrs={},
                                       block=child["id"], p=dict(x=1.5, y=0),
                                       scale=1, rotation=0, parameters={"Genişlik": 1.0}))
        library["blocks"].append(child)
        return doc, library

    def apply_and_check(self, doc, library):
        before = doc.to_bytes()
        entities = [r.entity.to_json() for r in doc.entities()]
        arguments = dict(operation="import", library=json.dumps(library), replace=True)
        validated = cad.blocks.edit.answer(doc, **arguments, op="validate")
        self.assertEqual(validated.status, "completed", str(validated.error))
        plan = cad.blocks.edit.answer(doc, **arguments, op="plan")
        self.assertEqual(plan.status, "completed", str(plan.error))
        self.assertEqual(doc.to_bytes(), before)
        answer = cad.blocks.edit.answer(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
        self.assertEqual(answer.status, "completed", str(answer.error))
        self.assertEqual([r.entity.to_json() for r in doc.entities()], entities)
        saved = doc.to_bytes()
        self.assertNotEqual(saved, before)
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), saved)

    def test_new_child_and_existing_parent_update_together_in_either_library_order(self):
        for reverse in (False, True):
            with self.subTest(reverse=reverse):
                doc, library = self.nested_library()
                if reverse:
                    library["blocks"].reverse()
                self.apply_and_check(doc, library)
                blocks = {b.name: b for b in doc.blocks()}
                parent = blocks["Kapı (dinamik)"]
                self.assertEqual(parent.entities[-1].block, blocks["Yeni çocuk"].id)

    def test_nested_parameter_rename_updates_parent_and_child_as_one_valid_set(self):
        doc, library = self.nested_library()
        # Establish the starting graph with its child first; old sequential import
        # cannot add the child and replace the existing parent in one command.
        cad.blocks.edit(doc, operation="import", library=json.dumps(dict(
            format=library["format"], version=library["version"], blocks=[library["blocks"][1]])))
        cad.blocks.edit(doc, operation="import", library=json.dumps(library), replace=True)
        library = json.loads(cad.blocks.edit(doc, operation="export").library)
        parent = next(b for b in library["blocks"] if b["name"] == "Kapı (dinamik)")
        child = next(b for b in library["blocks"] if b["name"] == "Yeni çocuk")
        child["dynamic"] = json.loads(json.dumps(child["dynamic"], ensure_ascii=False).replace(
            "Genişlik", "Açıklık"))
        nested = parent["entities"][-1]
        nested["parameters"]["Açıklık"] = nested["parameters"].pop("Genişlik")
        self.apply_and_check(doc, library)
        exported = json.loads(cad.blocks.edit(doc, operation="export").library)
        child = next(b for b in exported["blocks"] if b["name"] == "Yeni çocuk")
        self.assertIn("Açıklık", [p["name"] for p in child["dynamic"]["parameters"]])
        parent = next(b for b in exported["blocks"] if b["name"] == "Kapı (dinamik)")
        self.assertEqual(parent["entities"][-1]["parameters"], {"Açıklık": 1.0})

    def test_file_import_brings_a_new_child_and_updates_its_existing_parent_together(self):
        source, library = self.nested_library()
        cad.blocks.edit(source, operation="import", library=json.dumps(dict(
            format=library["format"], version=library["version"], blocks=[library["blocks"][1]])))
        cad.blocks.edit(source, operation="import", library=json.dumps(library), replace=True)
        doc, _ = drawing()
        before = doc.to_bytes()
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "nested.kcad"
            source.save(path)
            arguments = dict(path=str(path), mode="blocks", replace=True)
            plan = cad.file.import_.block.answer(doc, **arguments, op="plan")
            self.assertEqual(plan.status, "completed", str(plan.error))
            self.assertEqual(doc.to_bytes(), before)
            cad.file.import_.block(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
            after = doc.to_bytes()
            self.assertEqual(len(doc.blocks()), 2)
            self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
            doc.undo()
            self.assertEqual(doc.to_bytes(), before)
            doc.redo()
            self.assertEqual(doc.to_bytes(), after)

    def test_file_import_coordinates_a_nested_parameter_rename(self):
        source, library = self.nested_library()
        child_file = dict(format=library["format"], version=library["version"], blocks=[library["blocks"][1]])
        cad.blocks.edit(source, operation="import", library=json.dumps(child_file))
        cad.blocks.edit(source, operation="import", library=json.dumps(library), replace=True)
        doc = cad.Document.from_bytes(source.to_bytes())
        before = doc.to_bytes()
        # Prepare a valid source independently of the new batch writer: detach
        # its nested reference, rename the child, then restore the adapted parent.
        parent = library["blocks"][0]
        nested = parent["entities"].pop()
        cad.blocks.edit(source, operation="import", library=json.dumps(library), replace=True)
        library["blocks"][1]["dynamic"] = json.loads(json.dumps(
            library["blocks"][1]["dynamic"], ensure_ascii=False).replace("Genişlik", "Açıklık"))
        cad.blocks.edit(source, operation="import", library=json.dumps(library), replace=True)
        nested["parameters"]["Açıklık"] = nested["parameters"].pop("Genişlik")
        parent["entities"].append(nested)
        cad.blocks.edit(source, operation="import", library=json.dumps(library), replace=True)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "renamed.kcad"
            source.save(path)
            arguments = dict(path=str(path), mode="blocks", replace=True)
            plan = cad.file.import_.block.answer(doc, **arguments, op="plan")
            self.assertEqual(plan.status, "completed", str(plan.error))
            self.assertEqual(doc.to_bytes(), before)
            cad.file.import_.block(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
            after = doc.to_bytes()
            exported = json.loads(cad.blocks.edit(doc, operation="export").library)
            child = next(b for b in exported["blocks"] if b["name"] == "Yeni çocuk")
            self.assertIn("Açıklık", [p["name"] for p in child["dynamic"]["parameters"]])
            parent = next(b for b in exported["blocks"] if b["name"] == "Kapı (dinamik)")
            self.assertEqual(parent["entities"][-1]["parameters"], {"Açıklık": 1.0})
            self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
            doc.undo()
            self.assertEqual(doc.to_bytes(), before)
            doc.redo()
            self.assertEqual(doc.to_bytes(), after)


class BlockLibraryFormulaUpdate(unittest.TestCase):
    def formula_drawing(self):
        doc, library = drawing()
        dynamic = library["blocks"][0]["dynamic"]
        dynamic["parameters"].append(dict(kind="linear", name="Yarım",
                                          **{"from": dict(x=0, y=0), "to": dict(x=.45, y=0)},
                                          rule=dict(expression="@Genişlik / 2")))
        cad.blocks.edit(doc, operation="dynamic", block=doc.blocks()[0].id,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        return doc, json.loads(cad.blocks.edit(doc, operation="export").library)

    def values(self, doc):
        return cad.blocks.parameters(doc, operation="get", uids=[r.uid for r in doc.entities()]).to_json()["inserts"]

    def test_library_formula_change_recomputes_each_existing_instance_without_resetting_inputs(self):
        doc, library = self.formula_drawing()
        before = doc.to_bytes()
        inputs = [{k: v for k, v in row["values"].items() if k != "Yarım"} for row in self.values(doc)]
        library["blocks"][0]["dynamic"]["parameters"][-1]["rule"]["expression"] = "@Genişlik / 3"
        arguments = dict(operation="import", library=json.dumps(library), replace=True)
        plan = cad.blocks.edit.answer(doc, **arguments, op="plan")
        self.assertEqual(plan.status, "completed", str(plan.error))
        self.assertEqual(doc.to_bytes(), before)
        cad.blocks.edit(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
        rows = self.values(doc)
        for row, kept in zip(rows, inputs):
            self.assertEqual({k: v for k, v in row["values"].items() if k != "Yarım"}, kept)
            self.assertAlmostEqual(row["values"]["Yarım"], kept["Genişlik"] / 3)
        after = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)

    def test_formula_change_refreshes_an_existing_parent_not_in_the_incoming_library(self):
        doc, library = self.formula_drawing()
        child = library["blocks"][0]
        parent = dict(id="22222222-3333-4444-8555-666666666666", name="Taşıyıcı",
                      base=dict(x=0, y=0), entities=[dict(kind="insert", id=1, layerId="", attrs={},
                      block=child["id"], p=dict(x=0, y=0), scale=1, rotation=0,
                      parameters={"Genişlik": 1.2, "Yarım": .6})])
        library["blocks"].append(parent)
        cad.blocks.edit(doc, operation="import", library=json.dumps(library))
        before = doc.to_bytes()
        child["dynamic"]["parameters"][-1]["rule"]["expression"] = "@Genişlik / 3"
        library["blocks"] = [child]
        cad.blocks.edit(doc, operation="import", library=json.dumps(library), replace=True)
        exported = json.loads(cad.blocks.edit(doc, operation="export").library)
        outer = next(b for b in exported["blocks"] if b["name"] == "Taşıyıcı")
        values = outer["entities"][0]["parameters"]
        self.assertEqual(values["Genişlik"], 1.2)
        self.assertAlmostEqual(values["Yarım"], .4)
        saved = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)

    def test_formula_invalid_for_one_existing_instance_refuses_all_phases_atomically(self):
        doc, library = self.formula_drawing()
        before = doc.to_bytes()
        # Default .9 resolves to 9; the existing 1.0 instance divides by zero.
        library["blocks"][0]["dynamic"]["parameters"][-1]["rule"]["expression"] = "@Genişlik / abs(@Genişlik - 1)"
        for operation in ("validate", "plan", "execute"):
            answer = cad.blocks.edit.answer(doc, operation="import", library=json.dumps(library),
                                            replace=True, op=operation)
            self.assertEqual(answer.status, "failed", operation)
            self.assertEqual(answer.error.code, "invalid_parameter_value")
            self.assertEqual(answer.error.path, "library")
            self.assertEqual(doc.to_bytes(), before)

    def test_file_redefinition_refreshes_formulas_and_keeps_one_undo(self):
        doc, library = self.formula_drawing()
        before = doc.to_bytes()
        source = cad.Document.from_bytes(before)
        dynamic = library["blocks"][0]["dynamic"]
        dynamic["parameters"][-1]["rule"]["expression"] = "@Genişlik / 3"
        cad.blocks.edit(source, operation="dynamic", block=source.blocks()[0].id,
                        dynamic=cad.DynamicBlock.from_json(dynamic))
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "formulas.kcad"
            source.save(path)
            arguments = dict(path=str(path), mode="blocks", replace=True)
            plan = cad.file.import_.block.answer(doc, **arguments, op="plan")
            self.assertEqual(plan.status, "completed", str(plan.error))
            self.assertEqual(doc.to_bytes(), before)
            cad.file.import_.block(doc, **arguments, expected=cad.Expected(plan=plan.plan_id))
            for row in self.values(doc):
                self.assertAlmostEqual(row["values"]["Yarım"], row["values"]["Genişlik"] / 3)
            saved = doc.to_bytes()
            self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
            doc.undo()
            self.assertEqual(doc.to_bytes(), before)
