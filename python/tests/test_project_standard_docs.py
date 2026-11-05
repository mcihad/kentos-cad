"""Project standards use the public Python command and survive KCAD saving."""

import json
from pathlib import Path
import sys
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # The independent KCAD reader.


class ProjectStandardDocuments(unittest.TestCase):
    def test_complete_annotation_style_resources_are_read_only_through_python(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        resources = cad.StandardResources(
            text_style_definitions=[cad.TextStyleDef.from_json({
                "id": "office-text", "name": "A-NOT", "font": "arimo", "height": 2.5,
            })],
            dimension_style_definitions=[cad.DimensionStyleDef.from_json({
                "id": "office-dim", "name": "A-DIM", "height": 2.5,
            })],
        )
        standard = cad.DrawingStandard.from_json({"name": "Ofis", "layers": []})
        before = document.to_bytes()
        report = cad.standards.check(document, standard=standard, resources=resources)
        kinds = {issue.kind for issue in report.issues}
        self.assertIn(cad.StandardIssueKind.MISSING_TEXT_STYLE_DEFINITION, kinds)
        self.assertIn(cad.StandardIssueKind.MISSING_DIMENSION_STYLE_DEFINITION, kinds)
        self.assertTrue(all(not issue.fixable for issue in report.issues
                            if issue.kind in {
                                cad.StandardIssueKind.MISSING_TEXT_STYLE_DEFINITION,
                                cad.StandardIssueKind.MISSING_DIMENSION_STYLE_DEFINITION,
                            }))
        self.assertEqual(document.to_bytes(), before)

    def test_named_leader_style_check_is_available_through_python(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        style = cad.LeaderStyleDef.from_json({
            "id": "office-note", "name": "A-NOT", "height": 2.5,
            "arrow": "open", "arrowSize": 1.5,
        })
        resources = cad.StandardResources(leader_styles=[style])
        standard = cad.DrawingStandard.from_json({"name": "Ofis", "layers": []})
        before = document.to_bytes()
        report = cad.standards.check(document, standard=standard, resources=resources)
        self.assertTrue(any(issue.kind == cad.StandardIssueKind.MISSING_LEADER_STYLE_DEFINITION
                            and issue.subject == "A-NOT" and not issue.fixable
                            for issue in report.issues))
        self.assertEqual(document.to_bytes(), before)

    def test_sheet_template_revision_check_is_available_through_python(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        book = json.loads((ROOT / "fixtures/sheet/v1/drawing-set.json").read_text())["cases"][0]["book"]
        sheet = book["sheets"][0]
        sheet["origin"] = {"templateId": "office-a3", "revision": 1}
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        cad.sheets.edit(document, ops=[{"op": "addSheet", "sheet": sheet}])
        resources = cad.StandardResources(sheet_templates=[
            cad.StandardSheetTemplateReference(template_id="office-a3", revision=2)
        ])
        standard = cad.DrawingStandard.from_json({"name": "Ofis", "layers": []})
        before = document.to_bytes()
        report = cad.standards.check(document, standard=standard, resources=resources)
        self.assertTrue(any(issue.kind == cad.StandardIssueKind.SHEET_TEMPLATE_ORIGIN
                            and not issue.fixable for issue in report.issues))
        self.assertEqual(document.to_bytes(), before)

    def test_published_block_reference_check_is_available_through_python(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        publication = {
            "familyId": "22222222-3333-4444-8555-666666666666",
            "typeId": "33333333-4444-4555-8666-777777777777",
            "typeName": "Standart tek kanat", "release": 2,
        }
        resources = cad.StandardResources(blocks=[cad.StandardBlockReference.from_json({
            "name": "Ofis kapısı", "publication": publication,
        })])
        standard = cad.DrawingStandard.from_json({"name": "Ofis", "layers": []})
        before = document.to_bytes()
        report = cad.standards.check(document, standard=standard, resources=resources)
        self.assertTrue(any(issue.kind == cad.StandardIssueKind.MISSING_BLOCK_REFERENCE
                            and issue.subject == "Ofis kapısı" and not issue.fixable
                            for issue in report.issues))
        self.assertEqual(document.to_bytes(), before)

    def test_conflicts_are_available_in_python_without_changing_the_drawing(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        standard = json.loads((ROOT / "catalog/v1/standards/mimari-ofis.json").read_text())
        changed = json.loads(json.dumps(standard))
        changed["layers"][0]["color"] = "#ff0000"
        changed["byLayer"] = not standard.get("byLayer", False)
        def package(identity, value):
            return cad.ProjectStandardDocument(text=json.dumps({
                "format": "kentos.drawing-standard", "version": 1,
                "id": identity, "revision": "1", "standard": value,
            }))
        cad.settings.set(document, standard_docs=[package("first", standard), package("second", changed)])
        before = document.to_bytes()
        report = cad.standards.conflicts(document)
        self.assertEqual(len(report.conflicts), 2)
        self.assertEqual((report.conflicts[0].first_id, report.conflicts[0].later_id), ("first", "second"))
        self.assertEqual(report.conflicts[0].fields, ["byLayer"])
        self.assertEqual(report.conflicts[1].fields, ["color"])
        self.assertEqual(report.conflicts[0].first_values["byLayer"], standard.get("byLayer", False))
        self.assertEqual(report.conflicts[1].later_values["color"], "#ff0000")
        resolved = cad.standards.resolve(document)
        self.assertEqual(resolved.sources, ["first", "second"])
        self.assertEqual(resolved.provenance.rules.id, "first")
        self.assertEqual(resolved.provenance.rules.revision, "1")
        self.assertEqual(resolved.provenance.layers[standard["layers"][0]["name"]].id, "first")
        self.assertEqual(resolved.standard.layers[0].color, standard["layers"][0]["color"])
        self.assertEqual(resolved.standard.by_layer, standard.get("byLayer", False))
        cad.standards.check(document, standard=resolved.standard)
        cad.standards.apply.plan(document, standard=resolved.standard, expected_revision=resolved.revision)
        self.assertEqual(document.to_bytes(), before)

    def test_associate_save_reopen_and_remove(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        source = json.loads((ROOT / "fixtures/kcad/v2/standard-docs.json").read_text())
        documents = [cad.ProjectStandardDocument(text=item["text"])
                     for item in source["settings"]["standardDocs"]]
        before = document.to_bytes()
        cad.settings.set.plan(document, standard_docs=documents)
        self.assertEqual(document.to_bytes(), before)
        changed = cad.settings.set(document, standard_docs=documents)
        self.assertIn("standardDocs", changed.changed)
        saved = document.to_bytes()
        self.assertEqual(kcad.read(saved)[1]["settings"]["standardDocs"],
                         source["settings"]["standardDocs"])
        reopened = cad.Document.from_bytes(saved)
        self.assertEqual([d.text for d in reopened.info().settings.standard_docs],
                         [d.text for d in documents])
        cad.settings.set(reopened, standard_docs=[])
        self.assertNotIn("standardDocs", kcad.read(reopened.to_bytes())[1]["settings"])


if __name__ == "__main__":
    unittest.main()
