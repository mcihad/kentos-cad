"""CAD-SHEET paper coordinates and reusable mixed-unit blocks via the actual typed host."""
from __future__ import annotations

import json
import sys
import unittest
import warnings
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/formats/v1/cad-sheet"
CASES = json.loads((DIR / "cases.json").read_text())
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # noqa: E402  independent file reader


class CadSheetTests(unittest.TestCase):
    def test_twenty_thin_paper_paths_import_and_survive_kcad_and_undo(self) -> None:
        doc = cad.Document.new("Paper", workspace="cad", drawing_unit="mm")
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", cad.CommandWarning)
            plan = cad.file.import_.dxf.plan(doc, path=str(DIR / "thin-paper.dxf"))
            self.assertEqual(doc.sheets(), [])
            done = cad.file.import_.dxf(doc, path=str(DIR / "thin-paper.dxf"))
        self.assertEqual(done.objects, plan.objects)
        self.assertEqual(len(doc), 1, "paper paths are not model entities")
        book = kcad.read(doc.to_bytes())[1]["sheets"]["book"]
        sheet = book["sheets"][0]
        self.assertEqual(len(sheet["items"]), 20)
        for item, case in zip(sheet["items"], CASES["paper"]):
            with self.subTest(case["id"]):
                frame = item["frame"]
                self.assertGreaterEqual(frame["width"], 1000)
                self.assertGreaterEqual(frame["height"], 1000)
                shape = item["kind"]
                points = shape["points"] if shape["type"] == "line" else shape["shape"]["points"]
                self.assertEqual(len(points), len(case["pointsUm"]))
                for p, want in zip(points, case["pointsUm"]):
                    x = frame["left"] + p[0] * frame["width"] / 1_000_000
                    y = sheet["page"]["size"]["height"] - frame["top"] - p[1] * frame["height"] / 1_000_000
                    self.assertAlmostEqual(x, want[0], delta=1.0)
                    self.assertAlmostEqual(y, want[1], delta=1.0)
        self.assertIsNotNone(doc.undo())
        self.assertEqual(doc.sheets(), [])
        self.assertEqual(len(doc), 0)
        self.assertIsNotNone(doc.redo())
        self.assertEqual(len(doc.sheets()), 1)
        restored = kcad.read(doc.to_bytes())[1]["sheets"]["book"]
        self.assertEqual(restored, book)

    def test_imported_blocks_insert_at_the_same_real_size_in_all_unit_combinations(self) -> None:
        for case in CASES["units"]:
            with self.subTest(case["file"]), warnings.catch_warnings():
                warnings.simplefilter("ignore", cad.CommandWarning)
                doc = cad.Document.new("Units", workspace="cad", drawing_unit="mm")
                args = {"path": str(DIR / case["file"])}
                if case["fileUnit"] is not None:
                    args["file_unit"] = case["fileUnit"]
                cad.file.import_.dxf(doc, **args)
                child = next(b for b in doc.blocks() if b.name == "CHILD")
                line = child.entities[0]
                self.assertAlmostEqual(line.b.x - line.a.x, 2.0, delta=1e-10)
                self.assertAlmostEqual(line.b.y - line.a.y, 1.05, delta=1e-10)
                made = cad.blocks.insert(doc, block=child.id, at=(0, 0), layer_id=doc.active_layer)
                self.assertEqual(made.unit_factor, 1.0)
                self.assertEqual(doc.entity(made.uid).entity.scale, 1.0)
                self.assertIsNotNone(doc.undo())
                self.assertEqual(len(doc), 2, "undo removes only the fresh insert")
