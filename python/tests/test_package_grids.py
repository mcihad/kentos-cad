"""The delivery package's grids (`file.package`, docs/adr/3180; plan/map-cad.md MAP-17) against an independent reading.

The drawing is fixtures/exchange/v1/devir.json with three settings naming grids: the datum choice's NTv2 grid
(fixtures/geodesy/v1/ntv2/tr.gsb), the heights' geoid model (fixtures/geoid/v1/tg.gtx, with a note) and two survey
campaigns' velocity grids, one the geoid's file again, one no library holds. The device's grid library is a scratch
folder (`KENTOS_GRID_DIR`) holding the first two under their SHA-256. The package is made through KentOS's Python, then
held without KentOS code:

- paket.json lists each grid of izgaralar/ as `grid`, its size and SHA-256 those of the file in the folder (hashlib),
  the SHA-256 the file's name, the geoid named twice with two links and its note with none;
- the velocity grid no library holds is left out (`missing`, at its campaign's setting) and the warning says it.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PackageGrids(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = Path(tempfile.mkdtemp())
        library = cls.folder / "izgara"
        library.mkdir()
        cls.geoid, cls.ntv2 = sha(ROOT / "fixtures/geoid/v1/tg.gtx"), sha(ROOT / "fixtures/geodesy/v1/ntv2/tr.gsb")
        shutil.copy(ROOT / "fixtures/geoid/v1/tg.gtx", library / f"{cls.geoid}.gtx")
        (library / f"{cls.geoid}.json").write_text('{"kind": "geoid", "name": "TG-20"}', "utf-8")
        shutil.copy(ROOT / "fixtures/geodesy/v1/ntv2/tr.gsb", library / f"{cls.ntv2}.gsb")
        drawing = json.loads((ROOT / "fixtures/exchange/v1/devir.json").read_text("utf-8"))
        settings = drawing["settings"]
        settings["datumTransforms"] = [{"from": "ED50", "to": "TUREF", "name": "Bölge 7",
                                        "grid": {"id": cls.ntv2, "file": "tr.gsb", "size": 37936}}]
        settings["heights"] = {"kind": "orthometric", "verticalSrid": 5775, "geoid": {"name": "TG-20", "sha256": cls.geoid}}
        settings["mapSurvey"] = {"campaigns": [
            {"id": "kp-1", "name": "Bir", "velocity": {"name": "Hız ızgarası", "sha256": "a" * 64}},
            {"id": "kp-2", "name": "İki", "velocity": {"name": "TG-20 yine", "sha256": cls.geoid}},
        ]}
        doc = cad.Document.from_bytes(kr.container(kr.document(drawing)))
        cls.out = cls.folder / "teslim"
        before = os.environ.get("KENTOS_GRID_DIR")
        os.environ["KENTOS_GRID_DIR"] = str(library)
        try:
            w = cad.command("file.package")
            answer = w.run(doc, w.input_type.from_json({"path": str(cls.out), "name": "ada"}), op="execute")
        finally:
            if before is None:
                os.environ.pop("KENTOS_GRID_DIR", None)
            else:
                os.environ["KENTOS_GRID_DIR"] = before
        cls.error = answer.error
        cls.warnings = [(x.code, x.message) for x in answer.warnings]

    @classmethod
    def tearDownClass(cls) -> None:
        shutil.rmtree(cls.folder, ignore_errors=True)

    def setUp(self) -> None:
        self.assertIsNone(self.error, self.error and self.error.message)

    def manifest(self) -> dict:
        return json.loads((self.out / "paket.json").read_text("utf-8"))

    def test_each_grid_is_in_the_package_under_its_digest(self) -> None:
        grids = [f for f in self.manifest()["files"] if f["kind"] == "grid"]
        self.assertEqual([(f["path"], f["links"]) for f in grids],
                         [(f"izgaralar/{self.ntv2}.gsb", 1), (f"izgaralar/{self.geoid}.gtx", 2),
                          (f"izgaralar/{self.geoid}.json", 0)])
        for f in grids:
            path = self.out / f["path"]
            data = path.read_bytes()
            self.assertEqual(len(data), f["bytes"], f["path"])
            self.assertEqual(hashlib.sha256(data).hexdigest(), f["sha256"], f["path"])
            if not f["path"].endswith(".json"):
                self.assertEqual(path.stem, f["sha256"], "the file is named by its digest")

    def test_the_grid_no_library_holds_is_left_out_and_said(self) -> None:
        left = [x for x in self.manifest()["left"] if x["kind"] == "grid"]
        self.assertEqual([(x["path"], x["why"], x["at"]) for x in left],
                         [("Hız ızgarası", "missing", "mapSurvey.campaigns[0].velocity")])
        said = [m for code, m in self.warnings if code == "package_left"]
        self.assertTrue(said and "Hız ızgarası (yerinde yok)" in said[0], self.warnings)


if __name__ == "__main__":
    unittest.main()
