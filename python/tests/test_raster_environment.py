"""The raster tools' analysis environment from Python (plan/gis.md CBS-30, docs/adr/0643).

``cad.processing.run``'s ``environment`` reaches the raster tools as the window's Analiz ortamı does: the case files of
fixtures/raster-env/v1 (scripts/fixtures/raster_env_cases.py, GDAL and independent references) give the grids; a run
here writes the same grid and says it in its record. A tool that does not take the environment refuses it.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad.errors import CommandFailed
from kentos.cad.types import RasterEnvironment

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / "fixtures/raster-env/v1"
CASES = {c["id"]: c for c in json.loads((FOLDER / "cases.json").read_text(encoding="utf-8"))["cases"]}


def drawing() -> cad.Document:
    """The cases' drawing with its rasters' files named by their whole paths."""
    setup = json.loads((FOLDER / "drawing.kcad").read_text(encoding="utf-8"))
    for e in setup["entities"]:
        if e["kind"] == "raster":
            e["file"] = str(FOLDER / e["file"])
    return cad.Document.from_bytes(json.dumps(setup).encode())


def uid_of(doc: cad.Document, layer: str) -> str:
    return next(r.uid for r in doc.entities(layer=layer))


class RasterEnvironmentTests(unittest.TestCase):
    def run_case(self, case_id: str, environment: dict) -> None:
        case = CASES[case_id]
        doc = drawing()
        with tempfile.TemporaryDirectory() as tmp:
            out = str(Path(tmp) / "sonuc.tif")
            ran = cad.processing.run(doc, tool=case["tool"], parameters={**case["values"], "output": out},
                                     selection=[uid_of(doc, "d16")], environment=RasterEnvironment.from_json(environment))
            self.assertTrue(Path(out).exists(), "the result is written")
        grids = ran.record.grids
        written = next(g for g in grids if g.role == "output")
        want = case["expect"]["grid"]
        self.assertEqual((list(written.affine), written.width, written.height),
                         (want["affine"], want["width"], want["height"]))
        self.assertTrue(any(g.role == "input" for g in grids))

    def test_an_area_cuts_the_grid(self) -> None:
        self.run_case("hesap-alan-d16", {"area": CASES["hesap-alan-d16"]["environment"]["area"]})

    def test_a_cell_size_and_a_snap_raster_make_a_new_lattice(self) -> None:
        doc = drawing()
        env = dict(CASES["hesap-hiza-25"]["environment"])
        env["snap"] = uid_of(doc, "hiza")
        case = CASES["hesap-hiza-25"]
        with tempfile.TemporaryDirectory() as tmp:
            out = str(Path(tmp) / "sonuc.tif")
            ran = cad.processing.run(doc, tool="raster.calculator",
                                     parameters={"input": {"scope": "selection"}, "expression": "[D16] * 2", "output": out},
                                     selection=[uid_of(doc, "d16")], environment=RasterEnvironment.from_json(env))
        written = next(g for g in ran.record.grids if g.role == "output")
        self.assertEqual((written.width, written.height), (case["expect"]["grid"]["width"], case["expect"]["grid"]["height"]))

    def test_a_tool_that_does_not_take_it_says_so(self) -> None:
        doc = drawing()
        with self.assertRaises(CommandFailed) as e:
            cad.processing.run(doc, tool="attributes.calculate", parameters={"field": "Alan", "value": "1"},
                               selection=[uid_of(doc, "alan")], environment=RasterEnvironment(cell=5.0))
        self.assertEqual(e.exception.path, "environment")


if __name__ == "__main__":
    unittest.main()
