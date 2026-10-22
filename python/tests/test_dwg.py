"""The typed DWG API uses the native codec and the same file-effect command protocol."""
from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]


class DwgTests(unittest.TestCase):
    def test_typed_import_plan_undo_and_selected_export(self) -> None:
        fixture = json.loads((ROOT / "fixtures/commands/v1/file.import.dwg.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        source = str(ROOT / "fixtures/formats/v1/dwg/mm.dwg")
        planned = cad.file.import_.dwg.plan(doc, path=source)
        self.assertFalse(doc.info().can_undo)
        imported = cad.file.import_.dwg(doc, path=source)
        self.assertEqual(imported.objects, planned.objects)
        self.assertEqual(imported.objects, 5)
        self.assertTrue(doc.info().can_undo)
        uid = next(iter(doc.entities())).uid
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "selected.dwg"
            args = dict(path=str(path), uids=[uid], version=cad.DwgVersion.AC1032)
            plan = cad.file.export.dwg.plan(doc, **args)
            self.assertFalse(path.exists())
            receipt = cad.file.export.dwg(doc, request_id="dwg-selected", **args)
            self.assertEqual(receipt.sha256, plan.sha256)
            self.assertEqual(receipt.sha256, hashlib.sha256(path.read_bytes()).hexdigest())
            self.assertEqual(path.read_bytes()[:6], b"AC1032")
            repeated = cad.file.export.dwg(doc, request_id="dwg-selected", **args)
            self.assertEqual(repeated.sha256, receipt.sha256)
            with self.assertRaises(cad.CommandFailed) as refused:
                cad.file.export.dwg(doc, **args)
            self.assertEqual(refused.exception.code, "file_exists")
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), receipt.sha256)
        doc.undo()
        self.assertEqual(len(list(doc.entities())), 1)

    def test_paper_book_is_shared_by_typed_dxf_dwg_and_undo(self) -> None:
        fixture = json.loads((ROOT / "fixtures/commands/v1/file.import.dwg.json").read_text())
        def fresh():
            return cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        doc = fresh()
        cad.file.import_.dxf(doc, path=str(ROOT / "fixtures/formats/v1/cad-io/layouts.dxf"))
        self.assertEqual(len(doc.sheets()), 2)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "paper.dwg"
            cad.file.export.dwg(doc, path=str(path), version=cad.DwgVersion.AC1032)
            other = fresh()
            cad.file.import_.dwg(other, path=str(path))
            self.assertEqual(len(other.sheets()), 2)
        doc.undo()
        self.assertEqual(len(doc.sheets()), 0)

    def test_polyline_loss_needs_explicit_typed_acceptance(self) -> None:
        fixture = json.loads((ROOT / "fixtures/commands/v1/file.import.dwg.json").read_text())
        doc = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        source = str(ROOT / "fixtures/formats/v1/cad-io/ocs-0-width.dxf")
        before = len(list(doc.entities()))
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.file.import_.dxf.plan(doc, path=source)
        self.assertEqual(refused.exception.code, "polyline_geometry_loss")
        self.assertFalse(doc.info().can_undo)
        plan = cad.file.import_.dxf.plan(doc, path=source, allow_polyline_loss=True)
        self.assertEqual(len(list(doc.entities())), before)
        self.assertTrue(any(d["property"].startswith("polyline.loss.")
                            for n in plan.report["notes"] for d in n.get("details", [])))
        result = cad.file.import_.dxf(doc, path=source, allow_polyline_loss=True)
        self.assertEqual(result.objects, plan.objects)
        doc.undo()
        self.assertEqual(len(list(doc.entities())), before)
        # Both typed file writers protect an existing target before any publish.
        layer = next(iter(doc.entities())).entity.layer_id
        made = doc.run("cad.entities.create", {"layerId": layer, "objects": [{"geometry": {
            "kind": "polyline", "pts": [{"x":0,"y":0},{"x":3,"y":4}],
            "zs": [7,8], "widths": [[.2,.4],[0,0]]}}]})
        self.assertEqual(made["status"], "completed")
        with tempfile.TemporaryDirectory() as folder:
            for extension, writer in [("dxf", cad.file.export.dxf), ("dwg", cad.file.export.dwg)]:
                path = Path(folder) / f"nonplanar.{extension}"
                path.write_bytes(b"OLD TARGET")
                with self.assertRaises(cad.CommandFailed) as refused:
                    writer(doc, path=str(path), overwrite=True)
                self.assertEqual(refused.exception.code, "polyline_geometry_loss")
                self.assertEqual(path.read_bytes(), b"OLD TARGET")
                writer(doc, path=str(path), overwrite=True, allow_polyline_loss=True)
                self.assertNotEqual(path.read_bytes(), b"OLD TARGET")
