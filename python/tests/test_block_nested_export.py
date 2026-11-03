"""Static DXF preserves nested dynamic geometry and reports loss of editability."""
from pathlib import Path
import copy
import importlib.util
import json
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("block_export_reference", ROOT / "scripts/fixtures/kcad_v2_reference.py")
_ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_ref)


def snapshot(unit="mm"):
    raw = json.loads((ROOT / "fixtures/kcad/v2/dynamic-blocks.json").read_text())
    raw["settings"]["drawingUnit"] = unit
    block = raw["blocks"][0]
    visibility = next(p for p in block["dynamic"]["parameters"] if p["kind"] == "visibility")
    visibility["states"].reverse()  # Sade is the default: an absent override must still hide the swing.
    return raw


def open_raw(raw):
    return cad.Document.from_bytes(_ref.container(_ref.document(raw)))


class BlockNestedExport(unittest.TestCase):
    def test_combined_component_export_discloses_dimension_association_loss_before_writing(self):
        baseline = (ROOT / "fixtures/interaction/v1/block-parametric-component.kcad").read_bytes()
        doc = cad.Document.from_bytes(baseline)
        self.assertAlmostEqual(cad.dimensions.query(doc).dimensions[0].measured, 1.1)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "component.dxf"
            planned = cad.file.export.dxf.plan(doc, path=str(path))
            self.assertFalse(path.exists())
            self.assertEqual(doc.to_bytes(), baseline)
            for kind in ("Dinamik blok", "Ölçü bağı"):
                note = next(n for n in planned.report["notes"] if n["what"] == kind)
                self.assertEqual(note["severity"], "warning")
            note = next(n for n in planned.report["notes"] if n["what"] == "Ölçü bağı")
            self.assertIn("kendiliğinden izlemez", note["reason"])
            self.assertIn("KCAD", note["reason"])
            written = cad.file.export.dxf(doc, path=str(path))
            self.assertEqual(written.report, planned.report)
            self.assertEqual(doc.to_bytes(), baseline)
            back = cad.Document.open(path)
            self.assertAlmostEqual(cad.dimensions.query(back).dimensions[0].measured, 1.1)
            dimension = next(r.entity for r in back.entities() if r.entity.kind == "dimension")
            self.assertIsNone(dimension.to_json().get("refs"))

    def test_default_hidden_geometry_matches_an_explicit_state_and_reset_and_single_undo(self):
        raw = snapshot()
        raw["entities"][0].pop("parameters", None)
        doc = open_raw(raw)
        uid = next(doc.entities()).uid
        baseline = doc.to_bytes()
        proxy = cad.blocks.proxy(doc, uids=[uid]).inserts[0]
        self.assertEqual(len(proxy.entities), 3)
        self.assertFalse(any(e.kind == "arc" for e in proxy.entities))
        cad.blocks.parameters(doc, operation="set", uids=[uid], values={"Görünüş": "Açılım"})
        self.assertEqual(len(cad.blocks.proxy(doc, uids=[uid]).inserts[0].entities), 4)
        doc.undo()
        self.assertEqual(doc.to_bytes(), baseline)
        self.assertEqual(len(cad.blocks.proxy(doc, uids=[uid]).inserts[0].entities), 3)
        self.assertEqual(cad.Document.from_bytes(baseline).to_bytes(), baseline)

    def test_a_nested_width_exports_at_its_values_in_mm_and_m_without_editing_the_native_document(self):
        for unit in ("mm", "m"):
            with self.subTest(unit=unit), tempfile.TemporaryDirectory() as folder:
                raw = snapshot(unit)
                parent = copy.deepcopy(raw["blocks"][0])
                parent.update(id="0192f5a1-2900-7000-8000-0000000000e1", name="Giriş")
                parent.pop("dynamic")
                child = copy.deepcopy(raw["entities"][0])
                child.update(layerId="", parameters={"Genişlik": 1.2})
                parent["entities"] = [child]
                raw["blocks"].append(parent)
                raw["entities"] = [dict(kind="insert", id=1, layerId="kapilar", attrs={},
                    block=parent["id"], p=dict(x=10., y=20.), scale=2., rotation=1.5707963267948966)]
                raw["uids"] = raw["uids"][:1]
                doc = open_raw(raw)
                before = doc.to_bytes()
                path = Path(folder) / "nested.dxf"
                plan = cad.file.export.dxf.plan(doc, path=str(path))
                self.assertFalse(path.exists())
                self.assertEqual(doc.to_bytes(), before)
                notes = [n for n in plan.report["notes"] if n["what"] == "Dinamik blok"]
                self.assertEqual(len(notes), 1, "a dynamic source used only inside a definition must be reported")
                self.assertEqual(notes[0]["severity"], "warning")
                self.assertIn("İç içe", notes[0]["reason"])
                self.assertIn("parametreleri ve eylemleri DXF'e yazılmadı", notes[0]["reason"])
                result = cad.file.export.dxf(doc, path=str(path))
                self.assertEqual(result.report, plan.report)
                self.assertEqual(doc.to_bytes(), before)
                back = cad.Document.open(path)
                self.assertTrue(all(b.dynamic is cad.UNSET or b.dynamic is None for b in back.blocks()))
                parent = next(b for b in back.blocks() if b.name == "Giriş")
                nested_id = parent.entities[0].to_json()["block"]
                nested = next(b for b in back.blocks() if b.id == nested_id)
                self.assertEqual(len(nested.entities), 3)
                self.assertFalse(any(e.kind == "arc" for e in nested.entities))
                self.assertTrue(any(e.kind == "line" and abs(e.to_json()["b"]["y"] - 1.2) < 1e-9 for e in nested.entities))
                root = next(back.entities()).entity.to_json()
                self.assertEqual(root["p"], dict(x=10., y=20.))
                self.assertAlmostEqual(root["scale"], 2.)
                self.assertAlmostEqual(root["rotation"], 1.5707963267948966)
                encoded = back.to_bytes()
                self.assertEqual(cad.Document.from_bytes(encoded).to_bytes(), encoded)
