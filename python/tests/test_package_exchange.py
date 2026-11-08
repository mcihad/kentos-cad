"""The delivery package's open files (`file.package`'s `exchange`, docs/adr/2780; plan/map-cad.md MAP-17) against
independent readers.

The drawing is fixtures/exchange/v1/devir.json (scripts/fixtures/exchange_drawing.py). The package is made through
KentOS's Python with all six formats, then held without KentOS code:

- paket.json lists each open file of devir/ with the size and SHA-256 of the file in the folder (hashlib);
- GDAL's ogrinfo reads each GIS file (the Shapefile in its zip): as many features as the format's check says were
  written (its objects less those not written); ezdxf reads the DXF: the drawing's text is in it, as the matrix says DXF
  keeps it;
- the report devir/devir-raporu.csv (the csv module, `;`): a head of the six formats and a row for each aspect, each
  cell's first word the state of the matrix's cell (Korunur, Yaklaşık, Dönüşür, Eklenir, Kaybolur);
- every format's check in paket.json is the one `file.exchange.check` gives for the package's own project file.

Skipped where GDAL or ezdxf is not installed.
"""

from __future__ import annotations

import csv
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

DRAWING = json.loads((ROOT / "fixtures/exchange/v1/devir.json").read_text("utf-8"))
WORDS = {"Korunur": "kept", "Yaklaşık": "approximated", "Dönüşür": "converted", "Eklenir": "added", "Kaybolur": "lost"}
TITLES = ["GeoPackage", "GeoParquet", "FlatGeobuf", "GML", "Shapefile", "DXF"]


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(DRAWING)))


def features(path: Path) -> int:
    """How many features with a geometry GDAL reads in the file, every layer."""
    name = f"/vsizip/{path}" if path.suffix == ".zip" else str(path)
    out = subprocess.run(["ogrinfo", "-ro", "-al", "-q", name], capture_output=True, text=True, check=True).stdout
    count, geometry = 0, False
    for line in out.splitlines() + ["OGRFeature"]:
        if line.startswith("OGRFeature"):
            count += geometry
            geometry = False
        elif line.strip().startswith(("POINT", "LINESTRING", "POLYGON", "MULTI", "CIRCULARSTRING", "COMPOUNDCURVE",
                                      "CURVEPOLYGON")):
            geometry = True
    return count


class PackageOpenFiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = Path(tempfile.mkdtemp())
        cls.out = cls.folder / "teslim"
        w = cad.command("file.package")
        answer = w.run(opened(), w.input_type.from_json({"path": str(cls.out), "name": "ada", "exchange": {}}),
                       op="execute")
        cls.error = answer.error
        cls.answer = answer.output.to_json() if answer.output is not None else None
        cls.warnings = [x.code for x in answer.warnings]

    @classmethod
    def tearDownClass(cls) -> None:
        shutil.rmtree(cls.folder, ignore_errors=True)

    def setUp(self) -> None:
        self.assertIsNone(self.error, self.error and self.error.message)

    def manifest(self) -> dict:
        return json.loads((self.out / "paket.json").read_text("utf-8"))

    def test_the_manifest_holds_each_open_file_s_size_and_digest(self) -> None:
        made = [f for f in self.manifest()["files"] if f["kind"] == "exchange"]
        self.assertEqual([f["path"] for f in made],
                         ["devir/ada.gpkg", "devir/ada.parquet", "devir/ada.fgb", "devir/ada.gml", "devir/ada.xsd",
                          "devir/ada.zip", "devir/ada.dxf", "devir/devir-raporu.csv"])
        for f in made:
            data = (self.out / f["path"]).read_bytes()
            self.assertEqual(len(data), f["bytes"], f["path"])
            self.assertEqual(hashlib.sha256(data).hexdigest(), f["sha256"], f["path"])
        self.assertIn("export_losses", self.warnings)

    def test_gdal_reads_as_many_features_as_each_check_says_were_written(self) -> None:
        if not shutil.which("ogrinfo"):
            self.skipTest("GDAL yok")
        checks = {c["format"]: c for c in self.manifest()["exchange"]["checks"]}
        for fmt, ext in (("gpkg", "gpkg"), ("parquet", "parquet"), ("fgb", "fgb"), ("gml", "gml"), ("shp", "zip")):
            c = checks[fmt]
            written = c["objects"] - sum(k["count"] for k in c["notWritten"])
            self.assertEqual(features(self.out / "devir" / f"ada.{ext}"), written, fmt)
            self.assertEqual([k["kind"] for k in c["notWritten"]], ["text"], fmt)

    def test_ezdxf_finds_the_text_dxf_keeps(self) -> None:
        try:
            import ezdxf
        except ImportError:
            self.skipTest("ezdxf yok")
        text = next(e for e in DRAWING["entities"] if e["kind"] == "text")["text"]
        dxf = ezdxf.readfile(str(self.out / "devir" / "ada.dxf"))
        said = [e.dxf.text for e in dxf.modelspace() if e.dxftype() == "TEXT"]
        said += [e.text for e in dxf.modelspace() if e.dxftype() == "MTEXT"]
        self.assertIn(text, said)
        objects = next(r for r in self.manifest()["exchange"]["rows"] if r["aspect"] == "objects")
        self.assertEqual(objects["cells"][5]["state"], "kept")

    def test_the_report_says_each_cell_s_state(self) -> None:
        with (self.out / "devir" / "devir-raporu.csv").open(encoding="utf-8", newline="") as f:
            rows = list(csv.reader(f, delimiter=";"))
        self.assertEqual(rows[0], ["Özellik"] + TITLES)
        matrix = self.manifest()["exchange"]["rows"]
        self.assertEqual(len(rows) - 1, len(matrix))
        for said, row in zip(rows[1:], matrix):
            self.assertEqual(len(said), 1 + len(TITLES))
            for word, cell in zip(said[1:], row["cells"]):
                first = word.split(":")[0].split(" (")[0]
                self.assertEqual(WORDS[first], cell["state"], f"{row['aspect']} {cell['format']}: {word}")

    def test_each_check_is_the_package_project_s_own(self) -> None:
        packed = cad.Document.from_bytes((self.out / "ada.kcad").read_bytes())
        check = cad.command("file.exchange.check")
        for c in self.manifest()["exchange"]["checks"]:
            alone = check.run(packed, check.input_type.from_json({"format": c["format"]}), op="execute")
            self.assertIsNone(alone.error, alone.error and alone.error.message)
            self.assertEqual(alone.output.to_json(), c, c["format"])


if __name__ == "__main__":
    unittest.main()
