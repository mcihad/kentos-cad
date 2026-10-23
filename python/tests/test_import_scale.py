"""Physical source lengths are independent of display units and projected CRS."""
from pathlib import Path
import unittest

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/formats/v1/dxf-units/mm.dxf"


class ImportScale(unittest.TestCase):
    def test_declared_mm_is_four_metres_in_every_project(self):
        for options in [
            dict(workspace="cad", drawing_unit="mm"),
            dict(workspace="cad", drawing_unit="m"),
            dict(srid=5256),
        ]:
            with self.subTest(options):
                doc = cad.Document.new("Birim", **options)
                result = doc.import_dxf(SOURCE)
                self.assertEqual(result.unit.factor, 0.001)
                wall = next(r.entity for r in doc.entities() if r.entity.kind == "line")
                self.assertEqual((wall.a.x, wall.b.x), (0, 4))

    def test_reference_plan_commit_reopen_and_undo_preserve_physical_lengths_and_settings(self):
        for options in [
            dict(workspace="cad", drawing_unit="mm"),
            dict(workspace="cad", drawing_unit="cm"),
            dict(srid=5256),
        ]:
            with self.subTest(options):
                doc = cad.Document.new("Ölçü", **options)
                before = doc.settings.to_json()
                reference = cad.CadImportScale(source_length=4000, real_length=12.5)
                plan = cad.file.import_.dxf.plan(
                    doc, path=str(SOURCE), scale_reference=reference
                )
                self.assertFalse(doc.info().can_undo)
                self.assertEqual(plan.unit.factor, 12.5 / 4000)
                issued = doc.import_dxf(SOURCE, scale_reference=reference)
                self.assertEqual(issued.unit, plan.unit)
                self.assertEqual(doc.settings.to_json(), before)
                reopened = cad.Document.from_bytes(doc.to_bytes())
                wall = next(r.entity for r in reopened.entities() if r.entity.kind == "line")
                self.assertEqual(wall.b.x - wall.a.x, 12.5)
                doc.undo()
                self.assertEqual(list(doc.entities()), [])
                doc.redo()
                wall = next(r.entity for r in doc.entities() if r.entity.kind == "line")
                self.assertEqual(wall.b.x, 12.5)

    def test_conflicting_or_bad_reference_is_refused_before_any_write(self):
        doc = cad.Document.new(workspace="cad", drawing_unit="m")
        for reference, unit in [
            (cad.CadImportScale(source_length=0, real_length=1), None),
            (cad.CadImportScale(source_length=1, real_length=-1), None),
            (cad.CadImportScale(source_length=1, real_length=1), cad.FileUnit.MM),
        ]:
            with self.subTest(reference=reference, unit=unit):
                with self.assertRaises(cad.CommandFailed) as error:
                    cad.file.import_.dxf(
                        doc, path=str(SOURCE), scale_reference=reference, file_unit=unit
                    )
                self.assertEqual(error.exception.code, "invalid_scale")
                self.assertFalse(doc.info().can_undo)
                self.assertEqual(list(doc.entities()), [])
